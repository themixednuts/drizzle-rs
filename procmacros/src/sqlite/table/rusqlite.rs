//! rusqlite driver implementation for `SQLite` table macro.
//!
//! Generates `TryFrom` implementations for `rusqlite::Row` using the `FromSQLiteValue` trait.
//!
//! This implementation differs from libsql/turso in that it uses column names instead of
//! indices, and leverages our custom `FromSQLiteValue` trait for conversions (JSON fields
//! through `drizzle::core::Json<T>`).

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

/// Generate `TryFrom` implementations for `rusqlite::Row` for a table's models
pub fn generate_rusqlite_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let drizzle_error = paths::core::drizzle_error();
    let row_column_list = paths::core::row_column_list();
    let type_set_nil = paths::core::type_set_nil();
    let type_set_cons = paths::core::type_set_cons();
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
                generate_partial_field_from_row(idx, info),
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&drizzle::sqlite::rusqlite::Row<'_>> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::sqlite::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let partial_ident = format_ident!("Partial{}", select_model_ident);

    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&drizzle::sqlite::rusqlite::Row<'_>> for #partial_ident {
            type Error = #drizzle_error;

            fn try_from(row: &drizzle::sqlite::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    let from_drizzle_row_fields: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            let name = info.ident;
            let base_type = info.base_type;
            let idx_expr = quote!(offset + #idx);
            let select_type = info.get_select_type();
            let is_select_optional = syn::parse2::<syn::Type>(select_type)
                .map_or(info.is_nullable && !info.has_default, |ty| {
                    crate::common::is_option_type(&ty)
                });

            let value_expr = if info.is_json_column() {
                super::json::row_decode(&idx_expr, info, is_select_optional)
            } else if is_select_optional {
                quote! {
                    {
                        use drizzle::sqlite::traits::DrizzleRowByIndex;
                        DrizzleRowByIndex::get_column::<Option<#base_type>>(row, #idx_expr)?
                    }
                }
            } else {
                quote! {
                    {
                        use drizzle::sqlite::traits::DrizzleRowByIndex;
                        DrizzleRowByIndex::get_column::<#base_type>(row, #idx_expr)?
                    }
                }
            };

            quote! {
                #name: #value_expr,
            }
        })
        .collect();

    let field_count = field_infos.len();
    let mut column_list = quote!(#type_set_nil);
    for info in field_infos.iter().rev() {
        let select_ty = info.get_select_type();
        column_list = quote!(#type_set_cons<#select_ty, #column_list>);
    }
    let from_drizzle_row_impl = quote! {
        impl<'__drizzle_r> drizzle::core::FromDrizzleRow<drizzle::sqlite::rusqlite::Row<'__drizzle_r>> for #select_model_ident {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &drizzle::sqlite::rusqlite::Row<'__drizzle_r>, offset: usize) -> ::std::result::Result<Self, #drizzle_error> {
                Ok(Self {
                    #(#from_drizzle_row_fields)*
                })
            }
        }
    };
    let row_column_list_impl = quote! {
        impl<'__drizzle_r> #row_column_list<drizzle::sqlite::rusqlite::Row<'__drizzle_r>> for #select_model_ident {
            type Columns = #column_list;
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
        #from_drizzle_row_impl
        #row_column_list_impl
    })
}

// =============================================================================
// Field Conversion Generators
// =============================================================================

/// Generate field conversion for `SelectModel`
fn generate_field_from_row(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let from_sqlite_value = paths::sqlite::from_sqlite_value();
    let name = info.ident;
    let base_type = info.base_type;

    // JSON documents decode through `Json<Payload>`'s codec.
    if info.is_json_column() {
        let is_select_optional = syn::parse2::<syn::Type>(info.get_select_type())
            .map_or(info.is_nullable && !info.has_default, |ty| {
                crate::common::is_option_type(&ty)
            });
        let decode = super::json::row_decode(&quote!(#idx), info, is_select_optional);
        return Ok(quote! {
            #name: #decode,
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
                        #name: row.get::<_, i64>(#idx)? != 0,
                    })
                } else if !is_i64 {
                    Ok(quote! {
                        #name: row.get::<_, i64>(#idx)?.try_into()?,
                    })
                } else {
                    Ok(quote! {
                        #name: row.get(#idx)?,
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
                        #name: row.get::<_, String>(#idx)?,
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
                        #name: row.get::<_, f64>(#idx)? as f32,
                    })
                } else {
                    Ok(quote! {
                        #name: row.get(#idx)?,
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
                        #name: row.get::<_, Vec<u8>>(#idx)?,
                    })
                }
            }
            SQLiteType::Numeric | SQLiteType::Any => {
                if info.is_nullable {
                    Ok(quote! {
                        #name: {
                            let value_ref = row.get_ref(#idx)?;
                            match value_ref {
                                drizzle::sqlite::rusqlite::types::ValueRef::Null => None,
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
                    drizzle::sqlite::rusqlite::types::ValueRef::Null => None,
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

/// Generate field conversion for `PartialSelectModel` (all fields are Option<T>)
fn generate_partial_field_from_row(idx: usize, info: &FieldInfo) -> TokenStream {
    let from_sqlite_value = paths::sqlite::from_sqlite_value();
    let name = info.ident;
    let base_type = info.base_type;

    // JSON documents decode through `Json<Payload>`'s codec.
    if info.is_json_column() {
        let decode = super::json::partial_row_decode(&quote!(#idx), info);
        return quote! {
            #name: #decode,
        };
    }

    // Partial models have all fields as Option<T>
    quote! {
        #name: {
                    let value_ref = row.get_ref(#idx).unwrap_or(drizzle::sqlite::rusqlite::types::ValueRef::Null);
                    match value_ref {
                        drizzle::sqlite::rusqlite::types::ValueRef::Null => None,
                _ => <#base_type as #from_sqlite_value>::from_value_ref(value_ref).ok(),
            }
        },
    }
}
