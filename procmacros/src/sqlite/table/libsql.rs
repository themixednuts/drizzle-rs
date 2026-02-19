//! libsql driver implementation for SQLite table macro.
//!
//! Generates TryFrom implementations for `libsql::Row` using the shared driver infrastructure.

use super::errors;
use super::{FieldInfo, MacroContext};
use crate::common::{is_option_type, type_is_bool, type_is_float, type_is_int};
use crate::paths;
use crate::sqlite::field::{SQLiteType, TypeCategory};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

// =============================================================================
// Public API
// =============================================================================

/// Generate TryFrom implementations for libsql::Row for a table's models
pub(crate) fn generate_libsql_impls(ctx: &MacroContext) -> Result<TokenStream> {
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

    // libsql has simpler row access, so we use a custom implementation
    // that's more suited to its API
    let select: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| generate_field_from_row_for_select(i, info))
        .collect::<Result<Vec<_>>>()?;

    let from_drizzle_select: Vec<_> = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| generate_field_from_row_for_select_with_index(quote!(offset + #i), info))
        .collect::<Result<Vec<_>>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&::libsql::Row> for #select_model_ident {
            type Error = #drizzle_error;

            fn try_from(row: &::libsql::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let field_count = field_infos.len();
    let mut column_list = quote!(#type_set_nil);
    for _ in 0..field_count {
        column_list = quote!(#type_set_cons<(), #column_list>);
    }
    let from_drizzle_row_impl = quote! {
        impl drizzle::core::FromDrizzleRow<::libsql::Row> for #select_model_ident {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &::libsql::Row, offset: usize) -> ::std::result::Result<Self, #drizzle_error> {
                Ok(Self {
                    #(#from_drizzle_select)*
                })
            }
        }
    };
    let row_column_list_impl = quote! {
        impl #row_column_list<::libsql::Row> for #select_model_ident {
            type Columns = #column_list;
        }
    };

    Ok(quote! {
        #select_model_try_from_impl
        #from_drizzle_row_impl
        #row_column_list_impl
    })
}

// =============================================================================
// Field Conversion (libsql-specific due to its unique API)
// =============================================================================

fn generate_field_from_row_for_select(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let select_type = info.get_select_type();
    let is_optional = syn::parse2::<syn::Type>(select_type)
        .map(|ty| is_option_type(&ty))
        .unwrap_or(info.is_nullable && !info.has_default);
    generate_field_from_row_impl(quote!(#idx), info, is_optional)
}

fn generate_field_from_row_for_select_with_index(
    idx: TokenStream,
    info: &FieldInfo,
) -> Result<TokenStream> {
    let select_type = info.get_select_type();
    let is_optional = syn::parse2::<syn::Type>(select_type)
        .map(|ty| is_option_type(&ty))
        .unwrap_or(info.is_nullable && !info.has_default);
    generate_field_from_row_impl(idx, info, is_optional)
}

fn generate_field_from_row_impl(
    idx: TokenStream,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let idx = quote!((#idx) as i32);
    let name = info.ident;
    let base_type = info.base_type;

    // Check for unsupported reference types
    if matches!(base_type, syn::Type::Reference(_)) {
        return Err(syn::Error::new_spanned(
            name,
            errors::conversion::REFERENCE_TYPE_UNSUPPORTED,
        ));
    }

    // Dispatch based on type category
    match info.type_category() {
        TypeCategory::Json => handle_json_field(idx, name, info, is_optional),
        TypeCategory::Uuid => handle_uuid_field(idx, name, info, is_optional),
        TypeCategory::Enum => handle_enum_field(idx, name, info, is_optional),
        TypeCategory::ArrayString => handle_arraystring_field(idx, name, info, is_optional),
        TypeCategory::ArrayVec => handle_arrayvec_field(idx, name, info, is_optional),
        _ => handle_standard_field(idx, name, info, is_optional),
    }
}

fn handle_json_field(
    idx: TokenStream,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    if !cfg!(feature = "serde") {
        return Err(syn::Error::new_spanned(
            info.ident,
            errors::json::SERDE_REQUIRED,
        ));
    }

    let accessor = match info.column_type {
        SQLiteType::Text => {
            if is_optional {
                quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| serde_json::from_str(&v).ok())))
            } else {
                quote!(row.get::<String>(#idx).map(|v| serde_json::from_str(v.as_str()))?)
            }
        }
        SQLiteType::Blob => {
            if is_optional {
                quote!(row.get::<Option<Vec<u8>>>(#idx).map(|opt| opt.and_then(|v| serde_json::from_slice(&v).ok())))
            } else {
                quote!(row.get::<Vec<u8>>(#idx).map(|v| serde_json::from_slice(v.as_slice()))?)
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::json::INVALID_COLUMN_TYPE,
            ));
        }
    };

    Ok(quote! { #name: #accessor?, })
}

fn handle_uuid_field(
    idx: TokenStream,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        SQLiteType::Blob => {
            if is_optional {
                quote!(row.get::<Option<[u8;16]>>(#idx).map(|opt| opt.map(::uuid::Uuid::from_bytes)))
            } else {
                quote!(row.get::<[u8;16]>(#idx).map(::uuid::Uuid::from_bytes))
            }
        }
        SQLiteType::Text => {
            if is_optional {
                quote!(row.get::<Option<String>>(#idx).map(|opt| opt.map(|v| ::uuid::Uuid::parse_str(&v)).transpose())?)
            } else {
                quote!(row.get::<String>(#idx).map(|v| ::uuid::Uuid::parse_str(&v))?)
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::uuid::INVALID_COLUMN_TYPE,
            ));
        }
    };

    Ok(quote! { #name: #accessor?, })
}

fn handle_enum_field(
    idx: TokenStream,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        SQLiteType::Integer => {
            if is_optional {
                quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.and_then(|v| v.try_into().ok())))
            } else {
                quote!(row.get::<i64>(#idx).map(TryInto::try_into)?)
            }
        }
        SQLiteType::Text => {
            if is_optional {
                quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| v.try_into().ok())))
            } else {
                quote!(row.get::<String>(#idx).map(TryInto::try_into)?)
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::enums::INVALID_COLUMN_TYPE,
            ));
        }
    };

    Ok(quote! { #name: #accessor?, })
}

fn handle_arraystring_field(
    idx: TokenStream,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let from_sqlite_value = paths::sqlite::from_sqlite_value();
    let base_type = info.base_type;
    let accessor = if is_optional {
        quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| <#base_type as #from_sqlite_value>::from_sqlite_text(&v).ok())))
    } else {
        quote!(row.get::<String>(#idx).map(|v| <#base_type as #from_sqlite_value>::from_sqlite_text(&v))?)
    };

    Ok(quote! { #name: #accessor?, })
}

fn handle_arrayvec_field(
    idx: TokenStream,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let from_sqlite_value = paths::sqlite::from_sqlite_value();
    let base_type = info.base_type;
    let accessor = if is_optional {
        quote!(row.get::<Option<Vec<u8>>>(#idx).map(|opt| opt.and_then(|v| <#base_type as #from_sqlite_value>::from_sqlite_blob(&v).ok())))
    } else {
        quote!(row.get::<Vec<u8>>(#idx).map(|v| <#base_type as #from_sqlite_value>::from_sqlite_blob(&v))?)
    };

    Ok(quote! { #name: #accessor?, })
}

fn handle_standard_field(
    idx: TokenStream,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    match info.column_type {
        SQLiteType::Integer => {
            let is_bool = type_is_bool(info.base_type);
            let is_i64 = type_is_int(info.base_type, "i64");

            let accessor = if is_bool {
                if is_optional {
                    quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.map(|v| v != 0)))
                } else {
                    quote!(row.get::<i64>(#idx).map(|v| v != 0))
                }
            } else if !is_i64 {
                if is_optional {
                    quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.and_then(|v| v.try_into().ok())))
                } else {
                    quote!(row.get::<i64>(#idx).map(TryInto::try_into)?)
                }
            } else {
                quote!(row.get(#idx))
            };

            Ok(quote! { #name: #accessor?, })
        }
        SQLiteType::Text => {
            let accessor = if is_optional {
                quote!(row.get::<Option<String>>(#idx))
            } else {
                quote!(row.get::<String>(#idx))
            };
            Ok(quote! { #name: #accessor?, })
        }
        SQLiteType::Real => {
            let is_f32 = type_is_float(info.base_type, "f32");
            let accessor = if is_f32 {
                if is_optional {
                    quote!(row.get::<Option<f64>>(#idx).map(|opt| opt.map(|v| v as f32)))
                } else {
                    quote!(row.get::<f64>(#idx).map(|v| v as f32))
                }
            } else {
                quote!(row.get(#idx))
            };
            Ok(quote! { #name: #accessor?, })
        }
        SQLiteType::Blob => {
            let accessor = if is_optional {
                quote!(row.get::<Option<Vec<u8>>>(#idx))
            } else {
                quote!(row.get::<Vec<u8>>(#idx))
            };
            Ok(quote! { #name: #accessor?, })
        }
        SQLiteType::Numeric | SQLiteType::Any => {
            // Treat as integer/text
            let accessor = quote!(row.get(#idx));
            Ok(quote! { #name: #accessor?, })
        }
    }
}

// =============================================================================
// JSON/Enum Implementation Generation
// =============================================================================

/// Generate libsql JSON implementations (Into<libsql::Value>)
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
                    impl From<#struct_name> for ::libsql::Value {
                        fn from(value: #struct_name) -> Self {
                            match serde_json::to_string(&value) {
                                Ok(json_data) => ::libsql::Value::Text(json_data),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }

                    impl From<&#struct_name> for ::libsql::Value {
                        fn from(value: &#struct_name) -> Self {
                            match serde_json::to_string(value) {
                                Ok(json_data) => ::libsql::Value::Text(json_data),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }
                },
                SQLiteType::Blob => quote! {
                    impl From<#struct_name> for ::libsql::Value {
                        fn from(value: #struct_name) -> Self {
                            match serde_json::to_vec(&value) {
                                Ok(json_data) => ::libsql::Value::Blob(json_data),
                                Err(_) => ::libsql::Value::Null,
                            }
                        }
                    }

                    impl From<&#struct_name> for ::libsql::Value {
                        fn from(value: &#struct_name) -> Self {
                            match serde_json::to_vec(value) {
                                Ok(json_data) => ::libsql::Value::Blob(json_data),
                                Err(_) => ::libsql::Value::Null,
                            }
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

/// Generate libsql enum implementations (Into<libsql::Value>)
pub(crate) fn generate_enum_impls(info: &FieldInfo) -> Result<TokenStream> {
    if !info.is_enum {
        return Ok(quote! {});
    }

    let value_type = info.base_type;

    match info.column_type {
        SQLiteType::Integer => Ok(quote! {
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
        }),
        SQLiteType::Text => Ok(quote! {
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
        }),
        _ => Err(syn::Error::new_spanned(
            info.ident,
            errors::enums::INVALID_COLUMN_TYPE,
        )),
    }
}
