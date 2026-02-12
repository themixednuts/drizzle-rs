use super::{FieldInfo, MacroContext};
use crate::paths;
use crate::postgres::field::{PostgreSQLFlag, PostgreSQLType};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, Result};

/// Generate TryFrom implementations for sqlx::postgres::PgRow for a table's models
pub(crate) fn generate_sqlx_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let drizzle_error = paths::core::drizzle_error();
    let MacroContext {
        field_infos,
        select_model_ident,
        select_model_partial_ident,
        ..
    } = ctx;

    let (select, partial) = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            Ok((
                generate_field_from_row(idx, info)?,
                generate_partial_field_from_row(idx, info)?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::sqlx::postgres::PgRow> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &::sqlx::postgres::PgRow) -> ::std::result::Result<Self, Self::Error> {
                use ::sqlx::Row;
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::sqlx::postgres::PgRow> for #select_model_partial_ident {
            type Error = #drizzle_error;

            fn try_from(row: &::sqlx::postgres::PgRow) -> ::std::result::Result<Self, Self::Error> {
                use ::sqlx::Row;
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    // Final return with all implementations combined
    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
    })
}

/// Generate sqlx enum implementations (Type/Encode/Decode)
pub(crate) fn generate_enum_impls(info: &FieldInfo) -> Result<TokenStream> {
    if !info.is_enum && !info.is_pgenum {
        return Ok(quote! {});
    }

    let value_type = &info.field_type;

    if info.is_pgenum {
        // Native PostgreSQL enum - sqlx should handle this automatically
        // We just need to ensure the type implements the necessary traits
        Ok(quote! {
            // Native PostgreSQL enum support
            // sqlx will handle encoding/decoding automatically for native enum types
        })
    } else {
        // Our enum mapping (text or integer storage)
        match info.column_type {
            PostgreSQLType::Integer
            | PostgreSQLType::Bigint
            | PostgreSQLType::Smallint
            | PostgreSQLType::Smallserial => {
                Ok(quote! {
                    // sqlx Type/Encode/Decode for integer-stored enums
                    impl ::sqlx::Type<::sqlx::Postgres> for #value_type {
                        fn type_info() -> ::sqlx::postgres::PgTypeInfo {
                            <i64 as ::sqlx::Type<::sqlx::Postgres>>::type_info()
                        }
                    }

                    impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for #value_type {
                        fn encode_by_ref(&self, buf: &mut ::sqlx::postgres::PgArgumentBuffer) -> ::sqlx::encode::IsNull {
                            let val: i64 = (*self).into();
                            val.encode_by_ref(buf)
                        }
                    }

                    impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for #value_type {
                        fn decode(value: ::sqlx::postgres::PgValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                            let val = <i64 as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                            Self::try_from(val).map_err(|e| format!("Failed to convert {} to enum: {}", val, e).into())
                        }
                    }
                })
            }
            PostgreSQLType::Text | PostgreSQLType::Varchar | PostgreSQLType::Char => Ok(quote! {
                // sqlx Type/Encode/Decode for text-stored enums
                impl ::sqlx::Type<::sqlx::Postgres> for #value_type {
                    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
                        <String as ::sqlx::Type<::sqlx::Postgres>>::type_info()
                    }
                }

                impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for #value_type {
                    fn encode_by_ref(&self, buf: &mut ::sqlx::postgres::PgArgumentBuffer) -> ::sqlx::encode::IsNull {
                        let val: String = self.to_string();
                        val.encode_by_ref(buf)
                    }
                }

                impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for #value_type {
                    fn decode(value: ::sqlx::postgres::PgValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                        let val = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                        Self::try_from(val.as_str()).map_err(|e| format!("Failed to convert '{}' to enum: {}", val, e).into())
                    }
                }
            }),
            _ => Err(syn::Error::new_spanned(
                &info.name,
                "Enum is only supported with text or integer column types",
            )),
        }
    }
}

/// Generate sqlx JSON implementations (Type/Encode/Decode)
pub(crate) fn generate_json_impls(
    json_type_storage: &std::collections::HashMap<String, (PostgreSQLType, &FieldInfo)>,
) -> Result<Vec<TokenStream>> {
    if json_type_storage.is_empty() {
        return Ok(vec![]);
    }

    json_type_storage.iter().map(|(_, (storage_type, info))| {
        let struct_name = &info.field_type;
        let (type_info, encode_impl, decode_impl) = match storage_type {
            #[cfg(feature = "serde")]
            PostgreSQLType::Json => (
                quote! {
                    ::sqlx::postgres::PgTypeInfo::with_name("JSON")
                },
                quote! {
                    let json = ::serde_json::to_string(self)
                        .map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
                    json.encode_by_ref(buf)
                },
                quote! {
                    let json_str = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                    ::serde_json::from_str(&json_str)
                        .map_err(|e| format!("Failed to deserialize JSON: {}", e).into())
                }
            ),
            #[cfg(feature = "serde")]
            PostgreSQLType::Jsonb => (
                quote! {
                    ::sqlx::postgres::PgTypeInfo::with_name("JSONB")
                },
                quote! {
                    let json = ::serde_json::to_string(self)
                        .map_err(|e| format!("Failed to serialize to JSONB: {}", e))?;
                    json.encode_by_ref(buf)
                },
                quote! {
                    let json_str = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                    ::serde_json::from_str(&json_str)
                        .map_err(|e| format!("Failed to deserialize JSONB: {}", e).into())
                }
            ),
            PostgreSQLType::Text | PostgreSQLType::Varchar => (
                quote! {
                    <String as ::sqlx::Type<::sqlx::Postgres>>::type_info()
                },
                quote! {
                    let json = ::serde_json::to_string(self)
                        .map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
                    json.encode_by_ref(buf)
                },
                quote! {
                    let json_str = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                    ::serde_json::from_str(&json_str)
                        .map_err(|e| format!("Failed to deserialize JSON: {}", e).into())
                }
            ),
            PostgreSQLType::Bytea => (
                quote! {
                    <Vec<u8> as ::sqlx::Type<::sqlx::Postgres>>::type_info()
                },
                quote! {
                    let json = ::serde_json::to_vec(self)
                        .map_err(|e| format!("Failed to serialize to JSON bytes: {}", e))?;
                    json.encode_by_ref(buf)
                },
                quote! {
                    let json_bytes = <Vec<u8> as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                    ::serde_json::from_slice(&json_bytes)
                        .map_err(|e| format!("Failed to deserialize JSON bytes: {}", e).into())
                }
            ),
            _ => return Err(syn::Error::new_spanned(
                &info.name,
                "JSON fields must use JSON, JSONB, TEXT, VARCHAR, or BYTEA column types"
            )),
        };

        Ok(quote! {
            impl ::sqlx::Type<::sqlx::Postgres> for #struct_name {
                fn type_info() -> ::sqlx::postgres::PgTypeInfo {
                    #type_info
                }
            }

            impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for #struct_name {
                fn encode_by_ref(&self, buf: &mut ::sqlx::postgres::PgArgumentBuffer) -> Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
                    #encode_impl
                }
            }

            impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for #struct_name {
                fn decode(value: ::sqlx::postgres::PgValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                    #decode_impl
                }
            }
        })
    }).collect::<Result<Vec<_>>>()
}

/// Generate partial model field assignment (for sqlx PartialSelect models)
fn generate_partial_field_from_row(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let name = &info.name;

    if info.is_json && !cfg!(feature = "serde") {
        return Err(Error::new_spanned(
            &info.name,
            "JSON fields require the 'serde' feature to be enabled",
        ));
    }

    // For partial selects, all fields are Option<T>
    Ok(quote! {
        #name: row.try_get(#idx).unwrap_or_default(),
    })
}

/// Handles both standard types and conditional JSON deserialization.
fn generate_field_from_row(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let name = &info.name;

    if info.is_json && !cfg!(feature = "serde") {
        return Err(Error::new_spanned(
            &info.name,
            "JSON fields require the 'serde' feature to be enabled",
        ));
    }

    // For SELECT models, use direct field access
    Ok(quote! {
        #name: row.try_get(#idx)?,
    })
}
