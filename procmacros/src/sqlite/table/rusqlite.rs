use crate::sqlite::field::SQLiteType;

use super::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident, Result};

/// Generate TryFrom implementations for rusqlite::Row for a table's models
pub(crate) fn generate_rusqlite_impls(
    select_model_name: &Ident,
    insert_model_name: &Ident,
    update_model_name: &Ident,
    field_infos: &[FieldInfo<'_>],
) -> Result<TokenStream> {
    let (select, insert, update) = field_infos
        .iter()
        .map(|info| {
            Ok((
                generate_field_from_row(info, info.get_select_type())?,
                generate_field_from_row(info, info.get_insert_type())?,
                generate_field_from_row(info, info.get_update_type())?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for #select_model_name {
            type Error = ::rusqlite::Error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let insert_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for #insert_model_name {
            type Error = ::rusqlite::Error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#insert)*
                })
            }
        }
    };

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for #update_model_name {
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
        #insert_model_try_from_impl
        #update_model_try_from_impl
    })
}

/// Handles both standard types and conditional JSON deserialization.
fn generate_field_from_row(info: &FieldInfo, field_type: TokenStream) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;

    if info.is_json && !cfg!(feature = "serde") {
        return Err(Error::new_spanned(
            info.ident,
            "JSON fields require the 'serde' feature to be enabled",
        ));
    } else if info.is_uuid {
        if let Some(SQLiteType::Text) = info.column_type {
            if field_type.to_string().contains("Option") {
                Ok(quote! {
                    #name: Some(uuid::Uuid::parse_str(&row.get::<_, String>(#column_name)?).map_err(|_| rusqlite::types::FromSqlError::InvalidType)?),
                })
            } else {
                Ok(quote! {
                    #name: uuid::Uuid::parse_str(&row.get::<_, String>(#column_name)?).map_err(|_| rusqlite::types::FromSqlError::InvalidType)?,
                })
            }
        } else {
            Ok(quote! {
                #name: row.get(#column_name)?,
            })
        }
    } else {
        Ok(quote! {
            #name: row.get(#column_name)?,
        })
    }
}

pub fn generate_rusqlite_from_to_sql(infos: &[&FieldInfo]) -> Result<Vec<TokenStream>> {
    infos
        .iter()
        .map(|info| {
            let name = info.base_type;

            let to_sql_impl = match info.column_type {
                    Some(SQLiteType::Blob) => {
                        quote! {
                            impl rusqlite::types::ToSql for #name {
                                fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                                    let json = serde_json::to_vec(self)
                                        .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?;
                                    Ok(rusqlite::types::ToSqlOutput::Owned(json.into()))
                                }
                            }
                        }
                    },
                    Some(SQLiteType::Text) => {
                        quote! {
                            impl rusqlite::types::ToSql for #name {
                                fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                                    let json = serde_json::to_string(self)
                                        .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?;
                                    Ok(rusqlite::types::ToSqlOutput::Owned(json.into()))
                                }
                            }
                        }
                    },
                    _ =>  return Err(syn::Error::new_spanned(info.ident, "Json only supported for #[text] or #[blob]"))
            };

            Ok(quote! {
                impl rusqlite::types::FromSql for #name {
                     fn column_result(
                         value: rusqlite::types::ValueRef<'_>,
                     ) -> rusqlite::types::FromSqlResult<Self> {
                         match value {
                             rusqlite::types::ValueRef::Text(items) => serde_json::from_slice(items)
                                 .map_err(|_| rusqlite::types::FromSqlError::InvalidType),
                             rusqlite::types::ValueRef::Blob(items) => serde_json::from_slice(items)
                                 .map_err(|_| rusqlite::types::FromSqlError::InvalidType),
                             _ => Err(rusqlite::types::FromSqlError::InvalidType),
                         }
                     }
                 }

                #to_sql_impl

            })
        })
        .collect::<Result<Vec<_>>>()
}
