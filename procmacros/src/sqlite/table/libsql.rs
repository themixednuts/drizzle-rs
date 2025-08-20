use super::{FieldInfo, MacroContext};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Error, Result};

/// Generate TryFrom implementations for libsql::Row for a table's models
pub(crate) fn generate_libsql_impls(ctx: &MacroContext) -> Result<TokenStream> {
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
            Ok((
                generate_field_from_row_for_select(i, info)?,
                generate_field_from_row_for_update(i, info)?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::libsql::Row> for #select_model_ident {
            type Error = ::drizzle_rs::error::DrizzleError;

            fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::libsql::Row> for #update_model_ident {
            type Error = ::drizzle_rs::error::DrizzleError;

            fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
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

/// Generate field conversion for Select model
fn generate_field_from_row_for_select(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let select_type = info.get_select_type();
    generate_field_from_row_impl(idx, info, &select_type)
}

/// Generate field conversion for Update model  
fn generate_field_from_row_for_update(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let update_type = info.get_update_type();
    generate_field_from_row_impl(idx, info, &update_type)
}

/// Core implementation for field conversion from libsql Row
fn generate_field_from_row_impl(
    idx: usize,
    info: &FieldInfo,
    target_type: &TokenStream,
) -> Result<TokenStream> {
    let idx = idx as i32;
    let name = &info.ident;
    let target_type_str = target_type.to_string();
    let is_optional = target_type_str.contains("Option");
    let base_type_str = info.base_type.to_token_stream().to_string();

    if needs_reference_type(&base_type_str) {
        return Err(Error::new_spanned(name, "Can't support reference types."));
    }

    if info.is_json && !cfg!(feature = "serde") {
        return Err(Error::new_spanned(
            info.ident,
            "JSON fields require the 'serde' feature to be enabled",
        ));
    } else if info.is_json {
        return handle_json_field(idx, name, info, is_optional);
    } else if info.is_uuid {
        return handle_uuid_field(idx, name, info, is_optional);
    }

    // Standard field types
    match info.column_type {
        crate::sqlite::field::SQLiteType::Integer => {
            handle_integer_field(idx, name, info, is_optional, &base_type_str)
        }
        crate::sqlite::field::SQLiteType::Text => handle_text_field(idx, name, info, is_optional),
        crate::sqlite::field::SQLiteType::Real => {
            handle_real_field(idx, name, info, is_optional, &base_type_str)
        }
        crate::sqlite::field::SQLiteType::Blob => handle_blob_field(idx, name, info, is_optional),
        crate::sqlite::field::SQLiteType::Numeric => {
            // Treat as integer
            handle_integer_field(idx, name, info, is_optional, &base_type_str)
        }
        crate::sqlite::field::SQLiteType::Any => {
            // Default to text
            handle_text_field(idx, name, info, is_optional)
        }
    }
}

/// Handle JSON fields
fn handle_json_field(
    idx: i32,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        crate::sqlite::field::SQLiteType::Text => {
            if is_optional {
                quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| serde_json::from_str(&v).ok())))
            } else {
                quote!(row.get::<String>(#idx).map(|v|serde_json::from_str(v.as_str()))?)
            }
        }
        crate::sqlite::field::SQLiteType::Blob => {
            if is_optional {
                quote!(row.get::<Option<Vec<u8>>>(#idx).map(|opt| opt.and_then(|v| serde_json::from_slice(&v).ok())))
            } else {
                quote!(row.get::<Vec<u8>>(#idx).map(|v|serde_json::from_slice(v.as_slice()))?)
            }
        }
        _ => {
            return Err(Error::new_spanned(
                info.ident,
                "JSON fields must use TEXT or BLOB column types",
            ));
        }
    };

    Ok(quote! {
        #name: #accessor?,
    })
}

/// Handle UUID fields
fn handle_uuid_field(
    idx: i32,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        crate::sqlite::field::SQLiteType::Blob => {
            if is_optional {
                quote!(row.get::<Option<[u8;16]>>(#idx).map(|opt| opt.map(::uuid::Uuid::from_bytes)))
            } else {
                quote!(row.get::<[u8;16]>(#idx).map(::uuid::Uuid::from_bytes))
            }
        }
        crate::sqlite::field::SQLiteType::Text => {
            if is_optional {
                quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| ::uuid::Uuid::parse_str(&v).ok())))
            } else {
                quote!(::uuid::Uuid::parse_str(&row.get::<String>(#idx)?).map_err(Into::into))
            }
        }
        _ => {
            return Err(Error::new_spanned(
                info.ident,
                "UUID fields must use BLOB or TEXT column types",
            ));
        }
    };

    Ok(quote! {
        #name: #accessor?,
    })
}

/// Handle integer fields
fn handle_integer_field(
    idx: i32,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let is_not_i64 = !base_type_str.contains("i64");
    let is_bool = base_type_str.contains("bool");

    let accessor = if is_bool {
        if is_optional {
            quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.map(|v| v != 0)))
        } else {
            quote!(row.get::<i64>(#idx).map(|v| v != 0))
        }
    } else if info.is_enum || is_not_i64 {
        if is_optional {
            quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.and_then(|v| v.try_into().ok())))
        } else {
            quote!(row.get::<i64>(#idx).map(TryInto::try_into)?)
        }
    } else {
        quote!(row.get(#idx))
    };

    Ok(quote! {
        #name: #accessor?,
    })
}

/// Handle text fields
fn handle_text_field(
    idx: i32,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = if info.is_enum {
        if is_optional {
            quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| v.try_into().ok())))
        } else {
            quote!(row.get::<String>(#idx).map(TryInto::try_into)?)
        }
    } else if is_optional {
        quote!(row.get::<Option<String>>(#idx))
    } else {
        quote!(row.get::<String>(#idx))
    };

    Ok(quote! {
        #name: #accessor?,
    })
}

/// Handle real/float fields
fn handle_real_field(
    idx: i32,
    name: &syn::Ident,
    _info: &FieldInfo,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let accessor = if base_type_str.contains("f32") {
        if is_optional {
            quote!(row.get::<Option<f64>>(#idx).map(|opt| opt.map(|v| v as f32)))
        } else {
            quote!(row.get::<f64>(#idx).map(|v| v as f32))
        }
    } else {
        quote!(row.get(#idx))
    };

    Ok(quote! {
        #name: #accessor?,
    })
}

/// Handle blob fields
fn handle_blob_field(
    idx: i32,
    name: &syn::Ident,
    _info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = if is_optional {
        quote!(row.get::<Option<Vec<u8>>>(#idx))
    } else {
        quote!(row.get::<Vec<u8>>(#idx))
    };

    Ok(quote! {
        #name: #accessor?,
    })
}

/// Generate libsql JSON implementations (Into<libsql::Value>) - per JSON type approach
pub(crate) fn generate_json_impls(
    json_type_storage: &std::collections::HashMap<
        String,
        (crate::sqlite::field::SQLiteType, &FieldInfo),
    >,
) -> Result<Vec<TokenStream>> {
    if json_type_storage.is_empty() {
        return Ok(vec![]);
    }

    json_type_storage
        .iter()
        .map(|(_, (storage_type, info))| {
            let struct_name = info.base_type;
            let into_value_impl = match storage_type {
                crate::sqlite::field::SQLiteType::Text => quote! {
                    impl From<#struct_name> for ::libsql::Value {
                        fn from(value: #struct_name) -> Self {
                            match serde_json::to_string(&value) {
                                Ok(json) => ::libsql::Value::Text(json),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }

                    impl From<&#struct_name> for ::libsql::Value {
                        fn from(value: &#struct_name) -> Self {
                            match serde_json::to_string(value) {
                                Ok(json) => ::libsql::Value::Text(json),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }
                },
                crate::sqlite::field::SQLiteType::Blob => quote! {
                    impl From<#struct_name> for ::libsql::Value {
                        fn from(value: #struct_name) -> Self {
                            match serde_json::to_vec(&value) {
                                Ok(json) => ::libsql::Value::Blob(json),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }

                    impl From<&#struct_name> for ::libsql::Value {
                        fn from(value: &#struct_name) -> Self {
                            match serde_json::to_vec(value) {
                                Ok(json) => ::libsql::Value::Blob(json),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        info.ident,
                        "JSON fields must use either TEXT or BLOB column types",
                    ));
                }
            };

            Ok(into_value_impl)
        })
        .collect::<Result<Vec<_>>>()
}

/// Generate libsql enum implementations (Into<libsql::Value>) - per field approach
pub(crate) fn generate_enum_impls(info: &FieldInfo) -> Result<TokenStream> {
    if !info.is_enum {
        return Ok(quote! {});
    }

    let value_type = info.base_type;

    match info.column_type {
        crate::sqlite::field::SQLiteType::Integer => Ok(quote! {
            // ::libsql::Value for integer enums
            impl From<#value_type> for ::libsql::Value {
                fn from(value: #value_type) -> Self {
                    let integer: i64 = value.into();
                    ::libsql::Value::Integer(integer)
                }
            }

            impl From<&#value_type> for ::libsql::Value {
                fn from(value: &#value_type) -> Self {
                    let integer: i64 = (*value).clone().into();
                    ::libsql::Value::Integer(integer)
                }
            }

            // // IntoValue trait for libsql params! macro
            // impl ::libsql::params::IntoValue for #value_type {
            //     fn into_value(self) -> ::libsql::Result<::libsql::Value> {
            //         let integer: i64 = self.into();
            //         Ok(::libsql::Value::Integer(integer))
            //     }
            // }

            // impl ::libsql::params::IntoValue for &#value_type {
            //     fn into_value(self) -> ::libsql::Result<::libsql::Value> {
            //         let integer: i64 = (*self).into();
            //         Ok(::libsql::Value::Integer(integer))
            //     }
            // }
        }),
        crate::sqlite::field::SQLiteType::Text => Ok(quote! {
            // ::libsql::Value for text enums
            impl From<#value_type> for ::libsql::Value {
                fn from(value: #value_type) -> Self {
                    ::libsql::Value::Text(value.to_string())
                }
            }

            impl From<&#value_type> for ::libsql::Value {
                fn from(value: &#value_type) -> Self {
                    ::libsql::Value::Text(value.to_string())
                }
            }

            // // IntoValue trait for libsql params! macro
            // impl ::libsql::params::IntoValue for #value_type {
            //     fn into_value(self) -> ::libsql::Result<::libsql::Value> {
            //         let text: String = self.into();
            //         Ok(::libsql::Value::Text(text))
            //     }
            // }

            // impl ::libsql::params::IntoValue for &#value_type {
            //     fn into_value(self) -> ::libsql::Result<::libsql::Value> {
            //         let text: String = (*self).into();
            //         Ok(::libsql::Value::Text(text))
            //     }
            // }
        }),
        _ => Err(syn::Error::new_spanned(
            info.ident,
            "Enum is only supported in text or integer column types",
        )),
    }
}

/// Check if the base type needs a reference (&str, &[u8], &i64, etc.)
fn needs_reference_type(base_type_str: &str) -> bool {
    base_type_str.starts_with('&')
}
