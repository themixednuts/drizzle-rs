use super::{FieldInfo, MacroContext};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, Result};

/// Generate TryFrom implementations for rusqlite::Row for a table's models
pub(crate) fn generate_rusqlite_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {
        field_infos,
        select_model_ident,
        update_model_ident,
        ..
    } = ctx;

    #[cfg(feature = "rusqlite")]
    let (select, update, partial) = field_infos
        .iter()
        .map(|info| {
            let name = &info.ident;
            let column = &info.column_name;

            Ok((
                generate_field_from_row(info)?,
                generate_field_from_row(info)?,
                quote! { #name: row.get(#column).unwrap_or_default(), },
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (select, update) = field_infos
        .iter()
        .map(|info| {
            let name = &info.ident;
            let column = &info.column_name;

            Ok((
                generate_field_from_row(info)?,
                generate_field_from_row(info)?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #select_model_ident {
            type Error = ::rusqlite::Error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    #[cfg(feature = "rusqlite")]
    let partial_ident = format_ident!("Partial{}", select_model_ident);

    #[cfg(feature = "rusqlite")]
    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #partial_ident {
            type Error = ::rusqlite::Error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let partial_select_model_try_from_impl = quote! {};

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #update_model_ident {
            type Error = ::rusqlite::Error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#update)*
                })
            }
        }
    };

    // Final return with all implementations combined
    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
        #update_model_try_from_impl
    })
}

/// Generate rusqlite enum implementations (FromSql/ToSql)
pub(crate) fn generate_enum_impls(info: &FieldInfo) -> Result<TokenStream> {
    if !info.is_enum {
        return Ok(quote! {});
    }

    let value_type = info.base_type;

    match info.column_type {
        crate::sqlite::field::SQLiteType::Integer => Ok(quote! {
            // ::rusqlite::FromSql and ToSql for integer enums
            impl ::rusqlite::types::FromSql for #value_type {
                fn column_result(value: ::rusqlite::types::ValueRef<'_>) -> ::rusqlite::types::FromSqlResult<Self> {
                    match value {
                        ::rusqlite::types::ValueRef::Integer(i) => {
                            Self::try_from(i).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                        },
                        _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                    }
                }
            }

            impl ::rusqlite::types::ToSql for #value_type {
                fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                    let val: i64 = self.into();
                    Ok(::rusqlite::types::ToSqlOutput::Owned(::rusqlite::types::Value::Integer(val)))
                }
            }
        }),
        crate::sqlite::field::SQLiteType::Text => Ok(quote! {
            // ::rusqlite::FromSql and ToSql for text enums
            impl ::rusqlite::types::FromSql for #value_type {
                fn column_result(value: ::rusqlite::types::ValueRef<'_>) -> ::rusqlite::types::FromSqlResult<Self> {
                    match value {
                        ::rusqlite::types::ValueRef::Text(s) => {
                            let s_str = ::std::str::from_utf8(s).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)?;
                            Self::try_from(s_str).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                        },
                        _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                    }
                }
            }

            impl ::rusqlite::types::ToSql for #value_type {
                fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                    let val: &str = self.into();
                    Ok(::rusqlite::types::ToSqlOutput::Borrowed(::rusqlite::types::ValueRef::Text(val.as_bytes())))
                }
            }
        }),
        _ => Err(syn::Error::new_spanned(
            info.ident,
            "Enum is only supported in text or integer column types",
        )),
    }
}

/// Generate rusqlite JSON implementations (FromSql/ToSql)
pub(crate) fn generate_json_impls(
    json_type_storage: &std::collections::HashMap<
        String,
        (crate::sqlite::field::SQLiteType, &FieldInfo),
    >,
) -> Result<Vec<TokenStream>> {
    if json_type_storage.is_empty() {
        return Ok(vec![]);
    }

    json_type_storage.iter().map(|(_, (storage_type, info))| {
        let struct_name = info.base_type;
        let (from_impl, to_impl) = match storage_type {
            crate::sqlite::field::SQLiteType::Text => (
                quote! {
                    match value {
                        ::rusqlite::types::ValueRef::Text(items) => serde_json::from_slice(items)
                            .map_err(|_| ::rusqlite::types::FromSqlError::InvalidType),
                        _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                    }
                },
                quote! {
                    let json = serde_json::to_string(self)
                        .map_err(|e| ::rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    Ok(::rusqlite::types::ToSqlOutput::Owned(::rusqlite::types::Value::Text(json)))
                }
            ),
            crate::sqlite::field::SQLiteType::Blob => (
                quote! {
                    match value {
                        ::rusqlite::types::ValueRef::Blob(items) => serde_json::from_slice(items)
                            .map_err(|_| ::rusqlite::types::FromSqlError::InvalidType),
                        _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                    }
                },
                quote! {
                    let json = serde_json::to_vec(self)
                        .map_err(|e| ::rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    Ok(::rusqlite::types::ToSqlOutput::Owned(::rusqlite::types::Value::Blob(json)))
                }
            ),
            _ => return Err(syn::Error::new_spanned(
                info.ident,
                "JSON fields must use either TEXT or BLOB column types"
            )),
        };

        Ok(quote! {
            impl ::rusqlite::types::FromSql for #struct_name {
                fn column_result(
                    value: ::rusqlite::types::ValueRef<'_>,
                ) -> ::rusqlite::types::FromSqlResult<Self> {
                    #from_impl
                }
            }

            impl ::rusqlite::types::ToSql for #struct_name {
                fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                    #to_impl
                }
            }
        })
    }).collect::<Result<Vec<_>>>()
}

/// Handles both standard types and conditional JSON deserialization.
fn generate_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;

    if info.is_json && !cfg!(feature = "serde") {
        Err(Error::new_spanned(
            info.ident,
            "JSON fields require the 'serde' feature to be enabled",
        ))
    } else if info.is_uuid {
        // Handle all UUIDs as BLOB - rusqlite handles this perfectly with built-in support
        Ok(quote! {
            #name: row.get(#column_name)?,
        })
    } else {
        Ok(quote! {
            #name: row.get(#column_name)?,
        })
    }
}
