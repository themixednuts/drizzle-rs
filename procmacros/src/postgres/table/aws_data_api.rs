//! AWS Aurora Data API codegen for `#[PostgresTable]`.
//!
//! Emits `impl TryFrom<&drizzle::postgres::aws_data_api::Row>`,
//! `FromDrizzleRow<...>`, `NullProbeRow<...>`, and `RowColumnList<...>` for the
//! table's Select / Partial models. Unlike the postgres-sync / tokio-postgres
//! path (which uses `postgres-types::FromSql` via `row.get::<_, T>(idx)`), the
//! AWS Data API returns rows as pre-decoded `Field` enums — we dispatch through
//! the `FromDrizzleRow<aws_data_api::Row>` leaf impls defined in
//! `drizzle-postgres/src/aws_data_api.rs`.
//!
//! Field-level behaviour:
//!
//! * **Scalars / blobs / `Option<T>`**: delegated to `FromDrizzleRow` for the
//!   field type. Every type in `PostgresValue` has a matching leaf impl.
//! * **Integer-stored enum** → read as `i64`, narrowed to `i32`, converted via
//!   `TryFrom<i32>`.
//! * **Text-stored enum / native `#[PostgresEnum]`** → read as `String`, parsed
//!   via `FromStr`.
//! * **JSON with custom target struct** → read JSON text and deserialize into
//!   the target type.
//! * **Custom `DrizzlePostgresColumn` types** → rely on the user supplying a
//!   `FromDrizzleRow<aws_data_api::Row>` impl. Otherwise the build fails with
//!   a clear diagnostic pointing at the missing trait.

use proc_macro2::TokenStream;

#[cfg(feature = "aws-data-api")]
use super::context::MacroContext;
#[cfg(feature = "aws-data-api")]
use crate::paths;
#[cfg(feature = "aws-data-api")]
use crate::postgres::field::{FieldInfo, PostgreSQLType, TypeCategory};
#[cfg(feature = "aws-data-api")]
use quote::quote;

#[cfg(feature = "aws-data-api")]
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

/// Generate field conversion for the (non-partial) Select model.
#[cfg(feature = "aws-data-api")]
fn generate_select_field_conversion(idx: &TokenStream, info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let value = generate_field_value(idx, info);
    quote! { #name: #value, }
}

/// Decode one field's value at `idx`, propagating every decode error.
#[cfg(feature = "aws-data-api")]
fn generate_field_value(idx: &TokenStream, info: &FieldInfo) -> TokenStream {
    let drizzle_error = paths::core::drizzle_error();
    let base_type = &info.base_type;
    let field_type = &info.field_type;
    let type_category = TypeCategory::from_type(base_type);
    let read = |ty: TokenStream| {
        quote! {
            <#ty as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                ::from_row_at(row, #idx)?
        }
    };
    // Decode a stored value `v` (read as `stored`) into the field's base type.
    let stored_and_decode = if info.is_enum && is_integer_column(&info.column_type) {
        // Integer-stored enum: read as i64, narrow to i32 (checked: an `as`
        // cast would wrap an out-of-range value onto another variant), then
        // TryFrom<i32>.
        Some((
            quote!(i64),
            quote! {
                i32::try_from(v)
                    .ok()
                    .and_then(|n| <#base_type as ::core::convert::TryFrom<i32>>::try_from(n).ok())
                    .ok_or_else(|| #drizzle_error::ConversionError(
                        format!("AWS Data API: invalid integer enum {}", v).into()
                    ))?
            },
        ))
    } else if info.is_enum {
        // Legacy text-stored enums. PostgresEnum delegates through its
        // type-owned FromDrizzleRow implementation in the default branch.
        Some((
            quote!(String),
            quote! {
                v.parse::<#base_type>()
                    .map_err(|_| #drizzle_error::ConversionError(
                        format!("AWS Data API: failed to parse enum from {:?}", v).into()
                    ))?
            },
        ))
    } else if info.is_json && type_category != TypeCategory::Json {
        // JSON with custom target struct.
        Some((
            quote!(String),
            quote! {
                drizzle::core::Json::<#base_type>::from_json_str(&v).map(drizzle::core::Json::into_inner)
                    .map_err(|e| #drizzle_error::ConversionError(
                        format!("AWS Data API: JSON deserialize: {}", e).into()
                    ))?
            },
        ))
    } else {
        None
    };

    match stored_and_decode {
        Some((stored, decode)) if info.is_nullable => {
            let read = read(quote!(Option<#stored>));
            quote! {
                match #read {
                    Some(v) => Some(#decode),
                    None => None,
                }
            }
        }
        Some((stored, decode)) => {
            let read = read(stored);
            quote! {{
                let v = #read;
                #decode
            }}
        }
        // Default: delegate to FromDrizzleRow for the field type (handles Option<T> etc).
        None => read(quote!(#field_type)),
    }
}

/// Field conversion for the Partial Select model. All fields are
/// `Option<BaseType>`. A column past the end of the row is absent and decodes
/// as `None`; a NULL cell is `None`; a decode failure is an error.
#[cfg(feature = "aws-data-api")]
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

#[cfg(feature = "aws-data-api")]
pub fn generate_aws_data_api_impls(ctx: &MacroContext) -> TokenStream {
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

    let base_impls = quote! {
        impl ::std::convert::TryFrom<&drizzle::postgres::aws_data_api::Row>
            for #select_model_ident
        {
            type Error = #drizzle_error;

            fn try_from(
                row: &drizzle::postgres::aws_data_api::Row,
            ) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select_field_inits)*
                })
            }
        }

        impl ::std::convert::TryFrom<&drizzle::postgres::aws_data_api::Row>
            for #select_model_partial_ident
        {
            type Error = #drizzle_error;

            fn try_from(
                row: &drizzle::postgres::aws_data_api::Row,
            ) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial_field_inits)*
                })
            }
        }
    };

    let fdr_impl = quote! {
        impl drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>
            for #select_model_ident
        {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(
                row: &drizzle::postgres::aws_data_api::Row,
                offset: usize,
            ) -> ::std::result::Result<Self, #drizzle_error> {
                Ok(Self {
                    #(#from_drizzle_field_inits)*
                })
            }
        }

        impl #row_column_list<drizzle::postgres::aws_data_api::Row>
            for #select_model_ident
        {
            type Columns = #column_list;
        }
    };

    let null_probe_impl = if field_infos.is_empty() {
        quote! {}
    } else {
        quote! {
            impl drizzle::core::NullProbeRow<drizzle::postgres::aws_data_api::Row>
                for #select_model_ident
            {
                fn is_null_at(
                    row: &drizzle::postgres::aws_data_api::Row,
                    offset: usize,
                ) -> ::std::result::Result<bool, #drizzle_error> {
                    // Absent only when every column is NULL, as on the
                    // native drivers.
                    for column in offset..offset + #field_count {
                        if !drizzle::postgres::aws_data_api::is_null_at(row, column)? {
                            return ::std::result::Result::Ok(false);
                        }
                    }
                    ::std::result::Result::Ok(true)
                }
            }
        }
    };

    quote! { #base_impls #fdr_impl #null_probe_impl }
}

/// Fallback when the `aws-data-api` feature is off: emit nothing.
#[cfg(not(feature = "aws-data-api"))]
pub(crate) fn generate_aws_data_api_impls(_ctx: &super::context::MacroContext) -> TokenStream {
    TokenStream::new()
}
