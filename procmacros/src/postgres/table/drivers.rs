//! Shared driver infrastructure for `PostgreSQL` row conversion.
//!
//! This module provides a unified approach to generating `TryFrom` implementations
//! for different `PostgreSQL` drivers (postgres, tokio-postgres), reducing code duplication
//! and ensuring consistent behavior.
//!
//! This mirrors the pattern used by `SQLite`'s driver infrastructure.

use proc_macro2::TokenStream;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use super::context::MacroContext;
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use crate::paths;
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use crate::postgres::field::{FieldInfo, PostgreSQLType, TypeCategory};
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use quote::quote;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn driver_json_type() -> TokenStream {
    quote!(drizzle::postgres::driver_types::Json)
}

/// Check if a `PostgreSQL` column type is integer-based
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const fn is_integer_column(col_type: &PostgreSQLType) -> bool {
    matches!(
        col_type,
        PostgreSQLType::Integer
            | PostgreSQLType::Bigint
            | PostgreSQLType::Smallint
            | PostgreSQLType::Smallserial
            | PostgreSQLType::Serial
            | PostgreSQLType::Bigserial
    )
}

/// `row.try_get::<_, T>(idx)` with the driver error mapped into a
/// `DrizzleError`. `Row::get` panics on a type mismatch, so generated code
/// never calls it.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn try_get(ty: &TokenStream, idx: &TokenStream) -> TokenStream {
    let drizzle_error = paths::core::drizzle_error();
    quote! {
        row.try_get::<_, #ty>(#idx)
            .map_err(|e| #drizzle_error::ConversionError(e.to_string().into()))?
    }
}

/// Generate field conversion for SELECT model (non-partial).
///
/// The field type in the Select model matches the original table definition.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn generate_select_field_conversion(idx: &TokenStream, info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let value = generate_field_value(idx, info);
    quote! { #name: #value, }
}

/// Decode one field's value at `idx`. Every path is fallible and returns a
/// decode error rather than panicking.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn generate_field_value(idx: &TokenStream, info: &FieldInfo) -> TokenStream {
    let drizzle_error = paths::core::drizzle_error();
    let base_type = &info.base_type;
    let field_type = &info.field_type;
    let type_category = TypeCategory::from_type(base_type);

    // Determine if we need special handling via FromPostgresValue
    let needs_from_postgres_value = matches!(
        type_category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec
    );

    // PostgresEnum owns its storage codec, including native and integer-backed enums.
    if info.is_pgenum {
        return quote! {
            <#field_type as drizzle::core::FromDrizzleRow<drizzle::postgres::Row>>
                ::from_row_at(row, #idx)?
        };
    }

    // Custom types use DrizzlePostgresColumn for deserialization.
    if info.is_custom_type {
        if info.is_nullable {
            // The probe accepts every wire type: a concrete probe such as
            // `Option<i32>` fails its type check on any other column type.
            let probe = try_get(&quote!(Option<drizzle::core::AnyNullProbe>), idx);
            return quote! {
                if #probe.is_none() {
                    None
                } else {
                    Some(<#base_type as drizzle::postgres::traits::DrizzlePostgresColumn>::decode(row, #idx)?)
                }
            };
        }
        return quote! {
            <#base_type as drizzle::postgres::traits::DrizzlePostgresColumn>::decode(row, #idx)?
        };
    }

    // Handle legacy enums - check column type for storage format
    // - is_enum with INTEGER column: stored as integer, use try_into()
    // - is_enum with TEXT column: stored as text, use parse()
    if info.is_enum {
        let is_integer_enum = is_integer_column(&info.column_type);

        let (stored, decode) = if is_integer_enum {
            (
                quote!(i32),
                quote! {
                    <#base_type as TryFrom<i32>>::try_from(v).map_err(|_| #drizzle_error::ConversionError(format!("Failed to convert {} to enum", v).into()))?
                },
            )
        } else {
            (
                quote!(String),
                quote! {
                    v.parse::<#base_type>().map_err(|_| #drizzle_error::ConversionError(format!("Failed to parse enum from '{}'", v).into()))?
                },
            )
        };
        if info.is_nullable {
            let read = try_get(&quote!(Option<#stored>), idx);
            quote! {
                match #read {
                    Some(v) => Some(#decode),
                    None => None,
                }
            }
        } else {
            let read = try_get(&stored, idx);
            quote! {{
                let v = #read;
                #decode
            }}
        }
    } else if needs_from_postgres_value {
        let target = if info.is_nullable {
            quote!(Option<#base_type>)
        } else {
            quote!(#base_type)
        };
        quote! {{
            use drizzle::postgres::traits::DrizzleRowByIndex;
            DrizzleRowByIndex::get_column::<#target>(row, #idx)?
        }}
    } else if info.is_json && type_category != TypeCategory::Json {
        let json_type = driver_json_type();
        if info.is_nullable {
            let read = try_get(&quote!(::std::option::Option<#json_type<#base_type>>), idx);
            quote! { #read.map(|document| document.0) }
        } else {
            let read = try_get(&quote!(#json_type<#base_type>), idx);
            quote! { #read.0 }
        }
    } else {
        try_get(&quote!(#field_type), idx)
    }
}

/// Generate field conversion for PARTIAL SELECT model.
///
/// In partial models, all fields are `Option<BaseType>` where `BaseType` is
/// the unwrapped inner type (i.e. `Option<T>` fields use just `T`). A column
/// past the end of the row is absent and decodes as `None`; a NULL cell is
/// `None`; a cell that fails to decode is an error, not `None`.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn generate_partial_field_conversion(idx: usize, info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let base_type = &info.base_type;
    let mut optional = info.clone();
    optional.is_nullable = true;
    optional.field_type = syn::parse_quote!(::std::option::Option<#base_type>);
    let value = generate_field_value(&quote!(#idx), &optional);
    quote! {
        #name: if #idx < row.len() { #value } else { None },
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Generate `TryFrom` implementations for `PostgreSQL` drivers.
///
/// This generates `TryFrom<&drizzle::postgres::Row>` implementations.
/// The Row type is re-exported from whichever driver is active
/// (tokio-postgres or postgres-sync), so this single implementation works for both.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
pub fn generate_all_driver_impls(ctx: &MacroContext) -> TokenStream {
    let drizzle_error = paths::core::drizzle_error();
    let row_column_list = paths::core::row_column_list();
    let type_set_nil = paths::core::type_set_nil();
    let type_set_cons = paths::core::type_set_cons();
    let MacroContext {
        field_infos,
        select_model_ident,
        select_model_partial_ident,
        ..
    } = ctx;

    let select_field_inits: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| generate_select_field_conversion(&quote!(#idx), info))
        .collect();

    let from_drizzle_field_inits: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| generate_select_field_conversion(&quote!(offset + #idx), info))
        .collect();

    let partial_field_inits: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| generate_partial_field_conversion(idx, info))
        .collect();

    let field_count = field_infos.len();
    let mut column_list = quote!(#type_set_nil);
    for info in field_infos.iter().rev() {
        let select_ty = &info.field_type;
        column_list = quote!(#type_set_cons<#select_ty, #column_list>);
    }

    // Generate implementation using drizzle::postgres::Row which re-exports
    // the Row type from whichever driver is active
    let base_impls = quote! {
        impl ::std::convert::TryFrom<&drizzle::postgres::Row> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select_field_inits)*
                })
            }
        }

        impl ::std::convert::TryFrom<&drizzle::postgres::Row> for #select_model_partial_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial_field_inits)*
                })
            }
        }
    };

    let fdr_impl = quote! {
        impl drizzle::core::FromDrizzleRow<drizzle::postgres::Row> for #select_model_ident {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &drizzle::postgres::Row, offset: usize) -> ::std::result::Result<Self, #drizzle_error> {
                Ok(Self {
                    #(#from_drizzle_field_inits)*
                })
            }
        }

        impl #row_column_list<drizzle::postgres::Row> for #select_model_ident {
            type Columns = #column_list;
        }
    };

    // Generate NullProbeRow impl for LEFT JOIN support.
    // Enables `Option<SelectModel>` via blanket impl in drizzle-core. A LEFT
    // JOIN miss leaves every column NULL, while a match may hold NULL in any
    // nullable column, so the model is absent only when all of them are.
    // The probe accepts every wire type, so the check never depends on how
    // a column is stored.
    let null_probe_impl = if field_infos.is_empty() {
        quote! {}
    } else {
        quote! {
            impl drizzle::core::NullProbeRow<drizzle::postgres::Row> for #select_model_ident {
                fn is_null_at(row: &drizzle::postgres::Row, offset: usize) -> ::std::result::Result<bool, #drizzle_error> {
                    for column in offset..offset + #field_count {
                        let cell: Option<drizzle::core::AnyNullProbe> = row.try_get(column)
                            .map_err(|e| #drizzle_error::ConversionError(e.to_string().into()))?;
                        if cell.is_some() {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
            }
        }
    };

    quote! { #base_impls #fdr_impl #null_probe_impl }
}

/// Fallback when no postgres driver is enabled - returns empty TokenStream
#[cfg(not(any(feature = "postgres-sync", feature = "tokio-postgres")))]
pub(crate) fn generate_all_driver_impls(_ctx: &super::context::MacroContext) -> TokenStream {
    TokenStream::new()
}
