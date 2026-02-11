//! rusqlite driver implementation for SQLite table macro.
//!
//! Generates TryFrom implementations for `rusqlite::Row` using the `FromSQLiteValue` trait.
//!
//! This implementation differs from libsql/turso in that it uses column names instead of
//! indices, and leverages our custom `FromSQLiteValue` trait for all non-JSON conversions.

use super::errors;
use super::{FieldInfo, MacroContext};
use crate::common::{type_is_bool, type_is_float, type_is_int};
use crate::paths;
use crate::sqlite::field::{SQLiteType, TypeCategory};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

// =============================================================================
// Public API
// =============================================================================

/// Generate TryFrom implementations for rusqlite::Row for a table's models
pub(crate) fn generate_rusqlite_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let drizzle_error = paths::core::drizzle_error();
    let _from_sqlite_value = paths::sqlite::from_sqlite_value();
    let MacroContext {
        field_infos,
        select_model_ident,
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
        impl ::std::convert::TryFrom<&::rusqlite::Row<'_>> for #select_model_ident {
            type Error = #drizzle_error;

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
            type Error = #drizzle_error;

            fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
    })
}

// =============================================================================
// Field Conversion Generators
// =============================================================================

/// Generate field conversion for SelectModel
fn generate_field_from_row(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let from_sqlite_value = paths::sqlite::from_sqlite_value();
    let name = info.ident;
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
            #name: row.get(#idx)?,
        });
    }

    if matches!(
        info.type_category(),
        TypeCategory::Integer
            | TypeCategory::Real
            | TypeCategory::Bool
            | TypeCategory::String
            | TypeCategory::Blob
    ) {
        return match info.column_type {
            SQLiteType::Integer => {
                let is_bool = type_is_bool(info.base_type);
                let is_i64 = type_is_int(info.base_type, "i64");

                if info.is_nullable {
                    if is_bool {
                        Ok(quote! {
                            #name: row.get::<_, Option<i64>>(#idx)?.map(|v| v != 0),
                        })
                    } else if !is_i64 {
                        Ok(quote! {
                            #name: row
                                .get::<_, Option<i64>>(#idx)?
                                .map(TryInto::try_into)
                                .transpose()?,
                        })
                    } else {
                        Ok(quote! {
                            #name: row.get(#idx)?,
                        })
                    }
                } else if is_bool {
                    Ok(quote! {
                        #name: {
                            #[cfg(feature = "unchecked")]
                            {
                                row.get_unwrap::<_, i64>(#idx) != 0
                            }
                            #[cfg(not(feature = "unchecked"))]
                            {
                                row.get::<_, i64>(#idx)? != 0
                            }
                        },
                    })
                } else if !is_i64 {
                    Ok(quote! {
                        #name: row.get::<_, i64>(#idx)?.try_into()?,
                    })
                } else {
                    Ok(quote! {
                        #name: {
                            #[cfg(feature = "unchecked")]
                            {
                                row.get_unwrap(#idx)
                            }
                            #[cfg(not(feature = "unchecked"))]
                            {
                                row.get(#idx)?
                            }
                        },
                    })
                }
            }
            SQLiteType::Text => {
                if info.is_nullable {
                    Ok(quote! {
                        #name: row.get::<_, Option<String>>(#idx)?,
                    })
                } else {
                    Ok(quote! {
                        #name: {
                            #[cfg(feature = "unchecked")]
                            {
                                row.get_unwrap::<_, String>(#idx)
                            }
                            #[cfg(not(feature = "unchecked"))]
                            {
                                row.get::<_, String>(#idx)?
                            }
                        },
                    })
                }
            }
            SQLiteType::Real => {
                let is_f32 = type_is_float(info.base_type, "f32");
                if info.is_nullable {
                    if is_f32 {
                        Ok(quote! {
                            #name: row.get::<_, Option<f64>>(#idx)?.map(|v| v as f32),
                        })
                    } else {
                        Ok(quote! {
                            #name: row.get(#idx)?,
                        })
                    }
                } else if is_f32 {
                    Ok(quote! {
                        #name: {
                            #[cfg(feature = "unchecked")]
                            {
                                row.get_unwrap::<_, f64>(#idx) as f32
                            }
                            #[cfg(not(feature = "unchecked"))]
                            {
                                row.get::<_, f64>(#idx)? as f32
                            }
                        },
                    })
                } else {
                    Ok(quote! {
                        #name: {
                            #[cfg(feature = "unchecked")]
                            {
                                row.get_unwrap(#idx)
                            }
                            #[cfg(not(feature = "unchecked"))]
                            {
                                row.get(#idx)?
                            }
                        },
                    })
                }
            }
            SQLiteType::Blob => {
                if info.is_nullable {
                    Ok(quote! {
                        #name: row.get::<_, Option<Vec<u8>>>(#idx)?,
                    })
                } else {
                    Ok(quote! {
                        #name: {
                            #[cfg(feature = "unchecked")]
                            {
                                row.get_unwrap::<_, Vec<u8>>(#idx)
                            }
                            #[cfg(not(feature = "unchecked"))]
                            {
                                row.get::<_, Vec<u8>>(#idx)?
                            }
                        },
                    })
                }
            }
            SQLiteType::Numeric | SQLiteType::Any => {
                if info.is_nullable {
                    Ok(quote! {
                        #name: {
                            let value_ref = row.get_ref(#idx)?;
                            match value_ref {
                                ::rusqlite::types::ValueRef::Null => None,
                                _ => Some(<#base_type as #from_sqlite_value>::from_value_ref(value_ref)?),
                            }
                        },
                    })
                } else {
                    Ok(quote! {
                        #name: {
                            let value_ref = row.get_ref(#idx)?;
                            <#base_type as #from_sqlite_value>::from_value_ref(value_ref)?
                        },
                    })
                }
            }
        };
    }

    // All other types use FromSQLiteValue::from_value_ref
    if info.is_nullable {
        Ok(quote! {
            #name: {
                let value_ref = row.get_ref(#idx)?;
                match value_ref {
                    ::rusqlite::types::ValueRef::Null => None,
                    _ => Some(<#base_type as #from_sqlite_value>::from_value_ref(value_ref)?),
                }
            },
        })
    } else {
        Ok(quote! {
            #name: {
                let value_ref = row.get_ref(#idx)?;
                <#base_type as #from_sqlite_value>::from_value_ref(value_ref)?
            },
        })
    }
}

/// Generate field conversion for PartialSelectModel (all fields are Option<T>)
fn generate_partial_field_from_row(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let from_sqlite_value = paths::sqlite::from_sqlite_value();
    let name = info.ident;
    let base_type = info.base_type;

    // JSON fields use rusqlite's FromSql directly
    if info.type_category() == TypeCategory::Json {
        return Ok(quote! {
            #name: row.get(#idx).unwrap_or_default(),
        });
    }

    // Partial models have all fields as Option<T>
    Ok(quote! {
        #name: {
            let value_ref = row.get_ref(#idx).unwrap_or(::rusqlite::types::ValueRef::Null);
            match value_ref {
                ::rusqlite::types::ValueRef::Null => None,
                _ => <#base_type as #from_sqlite_value>::from_value_ref(value_ref).ok(),
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
                        let json_data = serde_json::to_string(self)
                            .map_err(|e| ::rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                        Ok(::rusqlite::types::ToSqlOutput::Owned(::rusqlite::types::Value::Text(json_data)))
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
                        let json_data = serde_json::to_vec(self)
                            .map_err(|e| ::rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                        Ok(::rusqlite::types::ToSqlOutput::Owned(::rusqlite::types::Value::Blob(json_data)))
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
