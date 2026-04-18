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
//! * **Scalars / blobs / option<T>** → delegated to `FromDrizzleRow` for the
//!   field type. Every type in `PostgresValue` has a matching leaf impl.
//! * **Integer-stored enum** → read as `i64`, narrowed to `i32`, converted via
//!   `TryFrom<i32>`.
//! * **Text-stored enum / native `#[PostgresEnum]`** → read as `String`, parsed
//!   via `FromStr`.
//! * **JSON with custom target struct** → read as `serde_json::Value`, then
//!   `serde_json::from_value`.
//! * **Custom `DrizzlePostgresColumn` types** → rely on the user supplying a
//!   `FromDrizzleRow<aws_data_api::Row>` impl. Otherwise the build fails with
//!   a clear diagnostic pointing at the missing trait.

use proc_macro2::TokenStream;
use syn::Result;

#[cfg(feature = "aws-data-api")]
use super::context::MacroContext;
#[cfg(feature = "aws-data-api")]
use crate::paths;
#[cfg(feature = "aws-data-api")]
use crate::postgres::field::{FieldInfo, PostgreSQLType, TypeCategory};
#[cfg(feature = "aws-data-api")]
use quote::quote;

#[cfg(feature = "aws-data-api")]
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

/// Generate field conversion for the (non-partial) Select model.
#[cfg(feature = "aws-data-api")]
fn generate_select_field_conversion(idx: TokenStream, info: &FieldInfo) -> TokenStream {
    let drizzle_error = paths::core::drizzle_error();
    let name = &info.ident;
    let base_type = &info.base_type;
    let field_type = &info.field_type;
    let type_category = TypeCategory::from_type(base_type);

    // Integer-stored enum: read as i64, narrow to i32, TryFrom<i32>.
    if info.is_enum && is_integer_column(&info.column_type) {
        if info.is_nullable {
            return quote! {
                #name: {
                    let v: Option<i64> =
                        <Option<i64> as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                            ::from_row_at(row, #idx)?;
                    match v {
                        Some(v) => Some(
                            <#base_type as ::core::convert::TryFrom<i32>>::try_from(v as i32)
                                .map_err(|_| #drizzle_error::ConversionError(
                                    format!("AWS Data API: invalid integer enum {}", v).into()
                                ))?
                        ),
                        None => None,
                    }
                },
            };
        } else {
            return quote! {
                #name: {
                    let v: i64 =
                        <i64 as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                            ::from_row_at(row, #idx)?;
                    <#base_type as ::core::convert::TryFrom<i32>>::try_from(v as i32)
                        .map_err(|_| #drizzle_error::ConversionError(
                            format!("AWS Data API: invalid integer enum {}", v).into()
                        ))?
                },
            };
        }
    }

    // Text-stored enum (including native `#[PostgresEnum]` — AWS always returns
    // enum values as StringValue regardless of column representation).
    if info.is_enum || info.is_pgenum {
        if info.is_nullable {
            return quote! {
                #name: {
                    let s: Option<String> =
                        <Option<String> as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                            ::from_row_at(row, #idx)?;
                    match s {
                        Some(s) => Some(
                            s.parse::<#base_type>()
                                .map_err(|_| #drizzle_error::ConversionError(
                                    format!("AWS Data API: failed to parse enum from {:?}", s).into()
                                ))?
                        ),
                        None => None,
                    }
                },
            };
        } else {
            return quote! {
                #name: {
                    let s: String =
                        <String as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                            ::from_row_at(row, #idx)?;
                    s.parse::<#base_type>()
                        .map_err(|_| #drizzle_error::ConversionError(
                            format!("AWS Data API: failed to parse enum from {:?}", s).into()
                        ))?
                },
            };
        }
    }

    // JSON with custom target struct — read Value, deserialize.
    if info.is_json && type_category != TypeCategory::Json {
        if info.is_nullable {
            return quote! {
                #name: {
                    let v: Option<::serde_json::Value> =
                        <Option<::serde_json::Value> as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                            ::from_row_at(row, #idx)?;
                    match v {
                        Some(v) => Some(
                            ::serde_json::from_value(v)
                                .map_err(|e| #drizzle_error::ConversionError(
                                    format!("AWS Data API: JSON deserialize: {}", e).into()
                                ))?
                        ),
                        None => None,
                    }
                },
            };
        } else {
            return quote! {
                #name: {
                    let v: ::serde_json::Value =
                        <::serde_json::Value as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                            ::from_row_at(row, #idx)?;
                    ::serde_json::from_value(v)
                        .map_err(|e| #drizzle_error::ConversionError(
                            format!("AWS Data API: JSON deserialize: {}", e).into()
                        ))?
                },
            };
        }
    }

    // Default: delegate to FromDrizzleRow for the field type (handles Option<T> etc).
    quote! {
        #name: <#field_type as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
            ::from_row_at(row, #idx)?,
    }
}

/// Field conversion for the Partial Select model. All fields are `Option<BaseType>`;
/// errors fall back to `None` rather than propagating.
#[cfg(feature = "aws-data-api")]
fn generate_partial_field_conversion(idx: usize, info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let base_type = &info.base_type;
    let type_category = TypeCategory::from_type(base_type);

    if info.is_enum && is_integer_column(&info.column_type) {
        return quote! {
            #name: {
                let v: ::core::result::Result<i64, _> =
                    <i64 as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                        ::from_row_at(row, #idx);
                v.ok().and_then(|v| <#base_type as ::core::convert::TryFrom<i32>>::try_from(v as i32).ok())
            },
        };
    }

    if info.is_enum || info.is_pgenum {
        return quote! {
            #name: {
                let s: ::core::result::Result<String, _> =
                    <String as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                        ::from_row_at(row, #idx);
                s.ok().and_then(|s| s.parse::<#base_type>().ok())
            },
        };
    }

    if info.is_json && type_category != TypeCategory::Json {
        return quote! {
            #name: {
                let v: ::core::result::Result<::serde_json::Value, _> =
                    <::serde_json::Value as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
                        ::from_row_at(row, #idx);
                v.ok().and_then(|v| ::serde_json::from_value::<#base_type>(v).ok())
            },
        };
    }

    quote! {
        #name: <#base_type as drizzle::core::FromDrizzleRow<drizzle::postgres::aws_data_api::Row>>
            ::from_row_at(row, #idx).ok(),
    }
}

// =============================================================================
// Public API
// =============================================================================

#[cfg(feature = "aws-data-api")]
pub(crate) fn generate_aws_data_api_impls(ctx: &MacroContext) -> Result<TokenStream> {
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

    let null_probe_impl = if !field_infos.is_empty() {
        quote! {
            impl drizzle::core::NullProbeRow<drizzle::postgres::aws_data_api::Row>
                for #select_model_ident
            {
                fn is_null_at(
                    row: &drizzle::postgres::aws_data_api::Row,
                    offset: usize,
                ) -> ::std::result::Result<bool, #drizzle_error> {
                    drizzle::postgres::aws_data_api::is_null_at(row, offset)
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! { #base_impls #fdr_impl #null_probe_impl })
}

/// Fallback when the `aws-data-api` feature is off: emit nothing.
#[cfg(not(feature = "aws-data-api"))]
pub(crate) fn generate_aws_data_api_impls(
    _ctx: &super::context::MacroContext,
) -> Result<TokenStream> {
    Ok(TokenStream::new())
}
