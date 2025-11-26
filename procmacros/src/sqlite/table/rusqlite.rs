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

    let (select, update, partial) = field_infos
        .iter()
        .map(|info| {
            Ok((
                generate_field_from_row(info)?,
                generate_update_field_from_row(info)?,
                generate_partial_field_from_row(info)?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #select_model_ident {
            type Error = ::drizzle_core::error::DrizzleError;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let partial_ident = format_ident!("Partial{}", select_model_ident);

    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #partial_ident {
            type Error = ::drizzle_core::error::DrizzleError;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #update_model_ident {
            type Error = ::drizzle_core::error::DrizzleError;

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
/// NOTE: This is now a no-op since SQLiteEnum derive generates these impls directly.
/// SQLiteEnum generates a FromSql that handles both TEXT and INTEGER storage,
/// so it works regardless of how the enum is stored in the table.
pub(crate) fn generate_enum_impls(_info: &FieldInfo) -> Result<TokenStream> {
    // SQLiteEnum now generates FromSql/ToSql implementations directly,
    // so we don't need to generate them here anymore.
    Ok(quote! {})
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

/// Generate partial model field assignment (for rusqlite PartialSelect models)
fn generate_partial_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;

    if info.is_uuid {
        // Handle UUIDs based on their column type for partial models
        match info.column_type {
            crate::sqlite::field::SQLiteType::Text => {
                // UUID stored as TEXT - get Option<String> and map to Option<Uuid>
                Ok(quote! {
                    #name: row.get::<_, Option<String>>(#column_name)
                        .map(|v| v.map(|v| ::uuid::Uuid::parse_str(&v)).transpose())?.unwrap_or_default(),
                })
            }
            crate::sqlite::field::SQLiteType::Blob => {
                // UUID stored as BLOB - use direct conversion
                Ok(quote! {
                    #name: row.get(#column_name).unwrap_or_default(),
                })
            }
            _ => Err(Error::new_spanned(
                info.ident,
                "UUID fields must use either TEXT or BLOB column types",
            )),
        }
    } else {
        // Standard field - use unwrap_or_default
        Ok(quote! { #name: row.get(#column_name).unwrap_or_default(), })
    }
}

/// Generate update model field assignment (always wraps values in Some() for Option fields)
fn generate_update_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;

    if info.is_json && !cfg!(feature = "serde") {
        Err(Error::new_spanned(
            info.ident,
            "JSON fields require the 'serde' feature to be enabled",
        ))
    } else if info.is_uuid {
        // Handle UUIDs based on their column type - always wrap in Some() for Update models
        match info.column_type {
            crate::sqlite::field::SQLiteType::Text => {
                // UUID stored as TEXT - convert from string and wrap in Some()
                Ok(quote! {
                    #name: {
                        let v: String = row.get(#column_name)?;
                        Some(::uuid::Uuid::parse_str(&v)?)
                    },
                })
            }
            crate::sqlite::field::SQLiteType::Blob => {
                // UUID stored as BLOB - use direct conversion and wrap in Some()
                Ok(quote! {
                    #name: Some(row.get(#column_name)?),
                })
            }
            _ => Err(Error::new_spanned(
                info.ident,
                "UUID fields must use either TEXT or BLOB column types",
            )),
        }
    } else {
        // Standard field - wrap in Some()
        Ok(quote! {
            #name: Some(row.get(#column_name)?),
        })
    }
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
        // Handle UUIDs based on their column type
        match info.column_type {
            crate::sqlite::field::SQLiteType::Text => {
                // UUID stored as TEXT - convert from string
                if info.is_nullable {
                    Ok(quote! {
                        #name: row.get::<_, Option<String>>(#column_name)
                            .map(|v| v.map(|v| ::uuid::Uuid::parse_str(&v)).transpose())??,
                    })
                } else {
                    Ok(quote! {
                        #name: ::uuid::Uuid::parse_str(&row.get::<_, String>(#column_name)?)?,
                    })
                }
            }
            crate::sqlite::field::SQLiteType::Blob => {
                // UUID stored as BLOB - use direct conversion
                Ok(quote! {
                    #name: row.get(#column_name)?,
                })
            }
            _ => Err(Error::new_spanned(
                info.ident,
                "UUID fields must use either TEXT or BLOB column types",
            )),
        }
    } else {
        Ok(quote! {
            #name: row.get(#column_name)?,
        })
    }
}
