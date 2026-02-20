//! Shared driver infrastructure for PostgreSQL row conversion.
//!
//! This module provides a unified approach to generating TryFrom implementations
//! for different PostgreSQL drivers (postgres, tokio-postgres), reducing code duplication
//! and ensuring consistent behavior.
//!
//! This mirrors the pattern used by SQLite's driver infrastructure.

use super::context::MacroContext;
use crate::paths;
use crate::postgres::field::{FieldInfo, PostgreSQLType, TypeCategory};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Check if a PostgreSQL column type is integer-based
fn is_integer_column(col_type: &PostgreSQLType) -> bool {
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

/// Generate field conversion for SELECT model (non-partial).
///
/// The field type in the Select model matches the original table definition.
fn generate_select_field_conversion(idx: TokenStream, info: &FieldInfo) -> TokenStream {
    let drizzle_error = paths::core::drizzle_error();
    let name = &info.ident;
    let base_type = &info.base_type;
    let type_category = TypeCategory::from_type(base_type);

    // Determine if we need special handling via FromPostgresValue
    let needs_from_postgres_value = matches!(
        type_category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec
    );

    // Handle enums - check column type for storage format
    // - is_enum with INTEGER column: stored as integer, use try_into()
    // - is_enum with TEXT column: stored as text, use parse()
    // - is_pgenum: native PostgreSQL enum, transmitted as text
    if info.is_enum || info.is_pgenum {
        let is_integer_enum = info.is_enum && is_integer_column(&info.column_type);

        if is_integer_enum {
            // Integer-stored enum: read as i32/i64 and use TryFrom
            if info.is_nullable {
                quote! {
                    #name: {
                        let v: Option<i32> = row.get::<_, Option<i32>>(#idx);
                        match v {
                            Some(v) => Some(<#base_type as TryFrom<i32>>::try_from(v).map_err(|_| #drizzle_error::ConversionError(format!("Failed to convert {} to enum", v).into()))?),
                            None => None,
                        }
                    },
                }
            } else {
                quote! {
                    #name: {
                        let v: i32 = row.get::<_, i32>(#idx);
                        <#base_type as TryFrom<i32>>::try_from(v).map_err(|_| drizzle::error::DrizzleError::ConversionError(format!("Failed to convert {} to enum", v).into()))?
                    },
                }
            }
        } else {
            // Text-stored enum or native pg enum: read as String and use parse()
            if info.is_nullable {
                quote! {
                    #name: {
                        let s: Option<String> = row.get::<_, Option<String>>(#idx);
                        match s {
                            Some(s) => Some(s.parse::<#base_type>().map_err(|_| drizzle::error::DrizzleError::ConversionError(format!("Failed to parse enum from '{}'", s).into()))?),
                            None => None,
                        }
                    },
                }
            } else {
                quote! {
                    #name: {
                        let s: String = row.get::<_, String>(#idx);
                        s.parse::<#base_type>().map_err(|_| drizzle::error::DrizzleError::ConversionError(format!("Failed to parse enum from '{}'", s).into()))?
                    },
                }
            }
        }
    } else if needs_from_postgres_value {
        // Use DrizzleRowByName::get_column_by_name with FromPostgresValue
        if info.is_nullable {
            quote! {
                #name: {
                    use drizzle::postgres::traits::DrizzleRowByIndex;
                    DrizzleRowByIndex::get_column::<Option<#base_type>>(row, #idx)?
                },
            }
        } else {
            quote! {
                #name: {
                    use drizzle::postgres::traits::DrizzleRowByIndex;
                    DrizzleRowByIndex::get_column::<#base_type>(row, #idx)?
                },
            }
        }
    } else if info.is_json && type_category != TypeCategory::Json {
        // JSON/JSONB with custom struct (not serde_json::Value)
        // Read as serde_json::Value and deserialize to target type
        if info.is_nullable {
            quote! {
                #name: {
                    let json_val: Option<::serde_json::Value> = row.get::<_, Option<::serde_json::Value>>(#idx);
                    match json_val {
                        Some(v) => Some(::serde_json::from_value(v).map_err(|e| drizzle::error::DrizzleError::ConversionError(format!("Failed to deserialize JSON: {}", e).into()))?),
                        None => None,
                    }
                },
            }
        } else {
            quote! {
                #name: {
                    let json_val: ::serde_json::Value = row.get::<_, ::serde_json::Value>(#idx);
                    ::serde_json::from_value(json_val).map_err(|e| drizzle::error::DrizzleError::ConversionError(format!("Failed to deserialize JSON: {}", e).into()))?
                },
            }
        }
    } else {
        // Standard types: use native driver's get
        let ty = &info.field_type;
        quote! {
            #name: row.get::<_, #ty>(#idx),
        }
    }
}

/// Generate field conversion for PARTIAL SELECT model.
///
/// In partial models, fields are Option<OriginalType>:
/// - String field -> Option<String>
/// - Option<String> field -> Option<Option<String>>
///
/// We use try_get which returns Result<T, Error> and fall back to None on error.
fn generate_partial_field_conversion(idx: usize, info: &FieldInfo) -> TokenStream {
    let _drizzle_error = paths::core::drizzle_error();
    let name = &info.ident;
    let base_type = &info.base_type;
    let type_category = TypeCategory::from_type(base_type);

    // Determine if we need special handling via FromPostgresValue
    let needs_from_postgres_value = matches!(
        type_category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec
    );

    // Handle enums - check column type for storage format
    if info.is_enum || info.is_pgenum {
        let is_integer_enum = info.is_enum && is_integer_column(&info.column_type);

        if is_integer_enum {
            // Integer-stored enum
            if info.is_nullable {
                // Original is Option<EnumType>, partial is Option<Option<EnumType>>
                quote! {
                    #name: {
                        let v: Option<Option<i32>> = row.try_get::<_, Option<i32>>(#idx).ok();
                        v.map(|opt| opt.and_then(|v| <#base_type as TryFrom<i32>>::try_from(v).ok()))
                    },
                }
            } else {
                // Original is EnumType, partial is Option<EnumType>
                quote! {
                    #name: {
                        let v: Option<i32> = row.try_get::<_, i32>(#idx).ok();
                        v.and_then(|v| <#base_type as TryFrom<i32>>::try_from(v).ok())
                    },
                }
            }
        } else {
            // Text-stored or native pg enum
            if info.is_nullable {
                // Original is Option<EnumType>, partial is Option<Option<EnumType>>
                quote! {
                    #name: {
                        let s: Option<Option<String>> = row.try_get::<_, Option<String>>(#idx).ok();
                        s.map(|opt| opt.and_then(|s| s.parse::<#base_type>().ok()))
                    },
                }
            } else {
                // Original is EnumType, partial is Option<EnumType>
                quote! {
                    #name: {
                        let s: Option<String> = row.try_get::<_, String>(#idx).ok();
                        s.and_then(|s| s.parse::<#base_type>().ok())
                    },
                }
            }
        }
    } else if needs_from_postgres_value {
        // Use DrizzleRowByName for FromPostgresValue types
        if info.is_nullable {
            // Original is Option<T>, partial is Option<Option<T>>
            quote! {
                #name: {
                    use drizzle::postgres::traits::DrizzleRowByIndex;
                    Some(DrizzleRowByIndex::get_column::<Option<#base_type>>(row, #idx).ok().flatten())
                },
            }
        } else {
            // Original is T, partial is Option<T>
            quote! {
                #name: {
                    use drizzle::postgres::traits::DrizzleRowByIndex;
                    DrizzleRowByIndex::get_column::<#base_type>(row, #idx).ok()
                },
            }
        }
    } else if info.is_json && type_category != TypeCategory::Json {
        // JSON/JSONB with custom struct (not serde_json::Value)
        // Read as serde_json::Value and deserialize to target type
        if info.is_nullable {
            // Original is Option<T>, partial is Option<Option<T>>
            quote! {
                #name: {
                    let json_val: Option<Option<::serde_json::Value>> = row.try_get::<_, Option<::serde_json::Value>>(#idx).ok();
                    json_val.map(|opt| opt.and_then(|v| ::serde_json::from_value(v).ok()))
                },
            }
        } else {
            // Original is T, partial is Option<T>
            quote! {
                #name: {
                    let json_val: Option<::serde_json::Value> = row.try_get::<_, ::serde_json::Value>(#idx).ok();
                    json_val.and_then(|v| ::serde_json::from_value(v).ok())
                },
            }
        }
    } else {
        // For standard types, try to get the original type (including Option wrapper if nullable)
        let ty = &info.field_type;
        quote! {
            #name: row.try_get::<_, #ty>(#idx).ok(),
        }
    }
}

/// Determine a "probe type" for NULL checking on the first column.
///
/// Used by `Option<SelectModel>` to detect LEFT JOIN misses:
/// read the first column as `Option<ProbeType>` — if `None`, all columns are NULL.
fn null_probe_type(info: &FieldInfo) -> TokenStream {
    if info.is_enum || info.is_pgenum {
        if info.is_enum && is_integer_column(&info.column_type) {
            quote!(i32)
        } else {
            quote!(String)
        }
    } else if info.is_json {
        quote!(::serde_json::Value)
    } else {
        let base = &info.base_type;
        let cat = TypeCategory::from_type(base);
        match cat {
            TypeCategory::ArrayString | TypeCategory::ArrayVec => quote!(String),
            _ => quote!(#base),
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Generate TryFrom implementations for PostgreSQL drivers.
///
/// This generates `TryFrom<&drizzle::postgres::Row>` implementations.
/// The Row type is re-exported from whichever driver is active
/// (tokio-postgres or postgres-sync), so this single implementation works for both.
#[cfg(feature = "postgres")]
pub(crate) fn generate_all_driver_impls(ctx: &MacroContext) -> Result<TokenStream> {
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
        .map(|(idx, info)| generate_select_field_conversion(quote!(#idx), info))
        .collect();

    let from_drizzle_field_inits: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| generate_select_field_conversion(quote!(offset + #idx), info))
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
    // Enables `Option<SelectModel>` via blanket impl in drizzle-core.
    let null_probe_impl = if let Some(first_field) = field_infos.first() {
        let probe_ty = null_probe_type(first_field);
        quote! {
            impl drizzle::core::NullProbeRow<drizzle::postgres::Row> for #select_model_ident {
                fn is_null_at(row: &drizzle::postgres::Row, offset: usize) -> ::std::result::Result<bool, #drizzle_error> {
                    let first_col: Option<#probe_ty> = row.try_get(offset)
                        .map_err(|e| #drizzle_error::ConversionError(e.to_string().into()))?;
                    Ok(first_col.is_none())
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! { #base_impls #fdr_impl #null_probe_impl })
}

/// Fallback when no postgres driver is enabled - returns empty TokenStream
#[cfg(not(feature = "postgres"))]
pub(crate) fn generate_all_driver_impls(_ctx: &MacroContext) -> Result<TokenStream> {
    Ok(TokenStream::new())
}
