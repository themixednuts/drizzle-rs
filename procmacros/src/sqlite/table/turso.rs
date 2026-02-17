//! Turso driver implementation for SQLite table macro.
//!
//! Generates TryFrom implementations for `turso::Row` using the shared driver infrastructure.

use super::drivers::{self, DriverConfig};
use super::errors;
use super::{FieldInfo, MacroContext};
use crate::common::is_option_type;
use crate::paths;
use crate::sqlite::field::{SQLiteType, TypeCategory};
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
        ..
    } = ctx;

    let select: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let select_type = info.get_select_type();
            let is_select_optional = syn::parse2::<syn::Type>(select_type)
                .map(|ty| is_option_type(&ty))
                .unwrap_or(info.is_nullable && !info.has_default);
            generate_field_conversion(i, info, is_select_optional)
        })
        .collect::<Result<Vec<_>>>()?;

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

    // Generate FromDrizzleRow impl for SelectModel only when all fields have leaf impls
    let has_unsupported_fields = field_infos.iter().any(|info| {
        let cat = {
            let select_type = info.get_select_type();
            let is_select_optional = syn::parse2::<syn::Type>(select_type)
                .map(|ty| is_option_type(&ty))
                .unwrap_or(info.is_nullable && !info.has_default);
            // Reuse the same category check as the field conversion
            let _ = is_select_optional;
            info.type_category()
        };
        matches!(
            cat,
            TypeCategory::Enum
                | TypeCategory::Json
                | TypeCategory::ArrayString
                | TypeCategory::ArrayVec
        )
    });

    let from_drizzle_row_impl = if has_unsupported_fields {
        quote! {}
    } else {
        let field_count = field_infos.len();
        let from_drizzle_row_fields: Vec<_> = field_infos
            .iter()
            .enumerate()
            .map(|(idx, info)| {
                let name = &info.ident;
                quote! {
                    #name: drizzle::core::FromDrizzleRow::from_row_at(row, offset + #idx)?,
                }
            })
            .collect();

        quote! {
            impl drizzle::core::FromDrizzleRow<::turso::Row> for #select_model_ident {
                const COLUMN_COUNT: usize = #field_count;

                fn from_row_at(row: &::turso::Row, offset: usize) -> ::std::result::Result<Self, #drizzle_error> {
                    Ok(Self {
                        #(#from_drizzle_row_fields)*
                    })
                }
            }
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #from_drizzle_row_impl
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
