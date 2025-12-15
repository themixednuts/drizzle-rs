//! Turso driver implementation for SQLite table macro.
//!
//! Generates TryFrom implementations for `turso::Row` using the shared driver infrastructure.

use super::drivers::{self, DriverConfig};
use super::errors;
use super::{FieldInfo, MacroContext};
use crate::paths;
use crate::sqlite::field::SQLiteType;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

// =============================================================================
// Turso Driver Configuration
// =============================================================================

/// Turso-specific driver configuration.
pub(crate) struct TursoDriver;

impl DriverConfig for TursoDriver {
    fn row_type() -> TokenStream {
        quote!(::turso::Row)
    }

    fn integer_accessor(idx: &TokenStream) -> TokenStream {
        quote!(row.get_value(#idx)?.as_integer())
    }

    fn text_accessor(idx: &TokenStream) -> TokenStream {
        quote!(row.get_value(#idx)?.as_text())
    }

    fn blob_accessor(idx: &TokenStream) -> TokenStream {
        quote!(row.get_value(#idx)?.as_blob())
    }

    fn real_accessor(idx: &TokenStream) -> TokenStream {
        quote!(row.get_value(#idx)?.as_real())
    }

    fn wrap_required(inner: TokenStream, name: &syn::Ident) -> TokenStream {
        let drizzle_error = paths::core::drizzle_error();
        let error_msg = errors::conversion::required_field(&name.to_string());
        quote! {
            #name: #inner
                .ok_or_else(|| #drizzle_error::ConversionError(#error_msg.to_string().into()))?,
        }
    }

    fn wrap_optional(inner: TokenStream, name: &syn::Ident) -> TokenStream {
        quote! {
            #name: #inner,
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Generate TryFrom implementations for turso::Row for a table's models
pub(crate) fn generate_turso_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let drizzle_error = paths::core::drizzle_error();
    let MacroContext {
        field_infos,
        select_model_ident,
        update_model_ident,
        ..
    } = ctx;

    let (select, update) = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let select_type = info.get_select_type();
            let update_type = info.get_update_type();
            let is_select_optional = select_type.to_string().contains("Option");
            let is_update_optional = update_type.to_string().contains("Option");

            Ok((
                generate_field_conversion(i, info, is_select_optional)?,
                generate_field_conversion(i, info, is_update_optional)?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>)>>()?;

    let row_type = TursoDriver::row_type();

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&#row_type> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &#row_type) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&#row_type> for #update_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &#row_type) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#update)*
                })
            }
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #update_model_try_from_impl
    })
}

// =============================================================================
// Field Conversion (delegating to shared infrastructure)
// =============================================================================

fn generate_field_conversion(
    idx: usize,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    // Use the shared driver infrastructure
    drivers::generate_field_conversion::<TursoDriver>(idx, info, is_optional)
}

// =============================================================================
// JSON/Enum Implementation Generation
// =============================================================================

/// Generate turso JSON implementations (IntoValue)
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
            let into_value_impl = match storage_type {
                SQLiteType::Text => quote! {
                    impl ::turso::IntoValue for #struct_name {
                        fn into_value(self) -> ::turso::Result<::turso::Value> {
                            let json_data = serde_json::to_string(&self)
                                .map_err(|e| ::turso::Error::ToSqlConversionFailure(Box::new(e)))?;
                            Ok(::turso::Value::Text(json_data))
                        }
                    }

                    impl ::turso::IntoValue for &#struct_name {
                        fn into_value(self) -> ::turso::Result<::turso::Value> {
                            let json_data = serde_json::to_string(self)
                                .map_err(|e| ::turso::Error::ToSqlConversionFailure(Box::new(e)))?;
                            Ok(::turso::Value::Text(json_data))
                        }
                    }
                },
                SQLiteType::Blob => quote! {
                    impl ::turso::IntoValue for #struct_name {
                        fn into_value(self) -> ::turso::Result<::turso::Value> {
                            let json_data = serde_json::to_vec(&self)
                                .map_err(|e| ::turso::Error::ToSqlConversionFailure(Box::new(e)))?;
                            Ok(::turso::Value::Blob(json_data))
                        }
                    }

                    impl ::turso::IntoValue for &#struct_name {
                        fn into_value(self) -> ::turso::Result<::turso::Value> {
                            let json_data = serde_json::to_vec(self)
                                .map_err(|e| ::turso::Error::ToSqlConversionFailure(Box::new(e)))?;
                            Ok(::turso::Value::Blob(json_data))
                        }
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        info.ident,
                        errors::json::INVALID_COLUMN_TYPE,
                    ));
                }
            };

            Ok(into_value_impl)
        })
        .collect::<Result<Vec<_>>>()
}

/// Generate turso enum implementations (IntoValue)
pub(crate) fn generate_enum_impls(info: &FieldInfo) -> Result<TokenStream> {
    if !info.is_enum {
        return Ok(quote! {});
    }

    let value_type = info.base_type;

    match info.column_type {
        SQLiteType::Integer => Ok(quote! {
            impl ::turso::IntoValue for #value_type {
                fn into_value(self) -> ::turso::Result<::turso::Value> {
                    let integer: i64 = self.into();
                    Ok(::turso::Value::Integer(integer))
                }
            }

            impl ::turso::IntoValue for &#value_type {
                fn into_value(self) -> ::turso::Result<::turso::Value> {
                    let integer: i64 = (*self).clone().into();
                    Ok(::turso::Value::Integer(integer))
                }
            }
        }),
        SQLiteType::Text => Ok(quote! {
            impl ::turso::IntoValue for #value_type {
                fn into_value(self) -> ::turso::Result<::turso::Value> {
                    Ok(::turso::Value::Text(self.to_string()))
                }
            }

            impl ::turso::IntoValue for &#value_type {
                fn into_value(self) -> ::turso::Result<::turso::Value> {
                    Ok(::turso::Value::Text(self.to_string()))
                }
            }
        }),
        _ => Err(syn::Error::new_spanned(
            info.ident,
            errors::enums::INVALID_COLUMN_TYPE,
        )),
    }
}
