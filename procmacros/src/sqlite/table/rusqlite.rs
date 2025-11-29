//! rusqlite driver implementation for SQLite table macro.
//!
//! Generates TryFrom implementations for `rusqlite::Row` using the `FromSQLiteValue` trait.
//!
//! This implementation differs from libsql/turso in that it uses column names instead of
//! indices, and leverages our custom `FromSQLiteValue` trait for all non-JSON conversions.

use super::errors;
use super::{FieldInfo, MacroContext};
use crate::sqlite::field::{SQLiteType, TypeCategory};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

// =============================================================================
// Public API
// =============================================================================

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

    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
        #update_model_try_from_impl
    })
}

// =============================================================================
// Field Conversion Generators
// =============================================================================

/// Generate field conversion for SelectModel
fn generate_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;
    let base_type = info.base_type;

    // JSON fields use rusqlite's FromSql directly
    if info.type_category() == TypeCategory::Json {
        if !cfg!(feature = "serde") {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::json::SERDE_REQUIRED,
            ));
        }
        return Ok(quote! {
            #name: row.get(#column_name)?,
        });
    }

    // All other types use FromSQLiteValue::from_value_ref
    if info.is_nullable {
        Ok(quote! {
            #name: {
                let value_ref = row.get_ref(#column_name)?;
                match value_ref {
                    ::rusqlite::types::ValueRef::Null => None,
                    _ => Some(<#base_type as ::drizzle_sqlite::traits::FromSQLiteValue>::from_value_ref(value_ref)?),
                }
            },
        })
    } else {
        Ok(quote! {
            #name: {
                let value_ref = row.get_ref(#column_name)?;
                <#base_type as ::drizzle_sqlite::traits::FromSQLiteValue>::from_value_ref(value_ref)?
            },
        })
    }
}

/// Generate field conversion for UpdateModel (always wraps in Some)
fn generate_update_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;
    let base_type = info.base_type;

    // JSON fields use rusqlite's FromSql directly
    if info.type_category() == TypeCategory::Json {
        if !cfg!(feature = "serde") {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::json::SERDE_REQUIRED,
            ));
        }
        return Ok(quote! {
            #name: Some(row.get(#column_name)?),
        });
    }

    // Update models wrap all values in Some()
    Ok(quote! {
        #name: {
            let value_ref = row.get_ref(#column_name)?;
            Some(<#base_type as ::drizzle_sqlite::traits::FromSQLiteValue>::from_value_ref(value_ref)?)
        },
    })
}

/// Generate field conversion for PartialSelectModel (all fields are Option<T>)
fn generate_partial_field_from_row(info: &FieldInfo) -> Result<TokenStream> {
    let name = info.ident;
    let column_name = &info.column_name;
    let base_type = info.base_type;

    // JSON fields use rusqlite's FromSql directly
    if info.type_category() == TypeCategory::Json {
        return Ok(quote! {
            #name: row.get(#column_name).unwrap_or_default(),
        });
    }

    // Partial models have all fields as Option<T>
    Ok(quote! {
        #name: {
            let value_ref = row.get_ref(#column_name).unwrap_or(::rusqlite::types::ValueRef::Null);
            match value_ref {
                ::rusqlite::types::ValueRef::Null => None,
                _ => <#base_type as ::drizzle_sqlite::traits::FromSQLiteValue>::from_value_ref(value_ref).ok(),
            }
        },
    })
}

// =============================================================================
// JSON/Enum Implementation Generation
// =============================================================================

/// Generate rusqlite enum implementations (FromSql/ToSql)
/// NOTE: This is now a no-op since SQLiteEnum derive generates these impls directly.
pub(crate) fn generate_enum_impls(_info: &FieldInfo) -> Result<TokenStream> {
    // SQLiteEnum now generates FromSql/ToSql implementations directly,
    // so we don't need to generate them here anymore.
    Ok(quote! {})
}

/// Generate rusqlite JSON implementations (FromSql/ToSql)
pub(crate) fn generate_json_impls(
    json_type_storage: &std::collections::HashMap<String, (SQLiteType, &FieldInfo)>,
) -> Result<Vec<TokenStream>> {
    if json_type_storage.is_empty() {
        return Ok(vec![]);
    }

    json_type_storage
        .iter()
        .map(|(_, (storage_type, info))| {
            let struct_name = info.base_type;
            let (from_impl, to_impl) = match storage_type {
                SQLiteType::Text => (
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
                    },
                ),
                SQLiteType::Blob => (
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
                    },
                ),
                _ => {
                    return Err(syn::Error::new_spanned(
                        info.ident,
                        errors::json::INVALID_COLUMN_TYPE,
                    ))
                }
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
        })
        .collect::<Result<Vec<_>>>()
}
