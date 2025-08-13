use super::{FieldInfo, MacroContext};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Error, Result};

/// Generate TryFrom implementations for turso::Row for a table's models
pub(crate) fn generate_turso_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {
        field_infos,
        select_model_ident,
        update_model_ident,
        ..
    } = ctx;
    let (select, update, partial) = field_infos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            Ok((
                generate_field_from_row_for_select(i, info)?,
                generate_field_from_row_for_update(i, info)?,
                generate_field_from_row_for_partial_select(i, info)?,
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

    let select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&turso::Row> for #select_model_ident {
            type Error = ::drizzle_rs::error::DrizzleError;

            fn try_from(row: &turso::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select)*
                })
            }
        }
    };

    let partial_ident = format_ident!("Partial{}", select_model_ident);

    let partial_select_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&turso::Row> for #partial_ident {
            type Error = ::drizzle_rs::error::DrizzleError;

            fn try_from(row: &turso::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial)*
                })
            }
        }
    };

    let update_model_try_from_impl = quote! {
        impl ::std::convert::TryFrom<&turso::Row> for #update_model_ident {
            type Error = ::drizzle_rs::error::DrizzleError;

            fn try_from(row: &turso::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#update)*
                })
            }
        }
    };

    // Generate IntoValue implementations for enums
    let enum_impls = generate_enum_into_value_impls(ctx)?;

    Ok(quote! {
        #select_model_try_from_impl
        #partial_select_model_try_from_impl
        #update_model_try_from_impl
        #enum_impls
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

/// Generate field conversion for PartialSelect model
fn generate_field_from_row_for_partial_select(idx: usize, info: &FieldInfo) -> Result<TokenStream> {
    let select_type = info.get_select_type();
    let partial_type = quote!(Option<#select_type>);
    generate_field_from_row_impl(idx, info, &partial_type)
}

/// Core implementation for field conversion from turso Row - clean approach with proper error handling
fn generate_field_from_row_impl(
    idx: usize,
    info: &FieldInfo,
    target_type: &TokenStream,
) -> Result<TokenStream> {
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

    // Standard field types - much cleaner dispatch
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
    idx: usize,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        crate::sqlite::field::SQLiteType::Text => quote!(row.get_value(#idx)?.as_text()),
        crate::sqlite::field::SQLiteType::Blob => quote!(row.get_value(#idx)?.as_blob()),
        _ => {
            return Err(Error::new_spanned(
                info.ident,
                "JSON fields must use TEXT or BLOB column types",
            ));
        }
    };

    let converter = match info.column_type {
        crate::sqlite::field::SQLiteType::Text => {
            |v: TokenStream| quote!(#v.map(|v| serde_json::from_str(v)).transpose()?)
        }
        crate::sqlite::field::SQLiteType::Blob => {
            |v: TokenStream| quote!(#v.map(|v| serde_json::from_slice(v)).transpose()?)
        }
        _ => unreachable!(),
    };

    Ok(wrap_optional(converter(accessor), name, is_optional))
}

fn wrap_optional(inner: TokenStream, name: &syn::Ident, is_optional: bool) -> TokenStream {
    if is_optional {
        // Already handled mapping/transposing in the converter, just assign
        quote! {
            #name: #inner,
        }
    } else {
        let error_msg = format!("Error converting required field `{}`", name);
        quote! {
            #name: #inner
                .ok_or_else(|| ::drizzle_rs::error::DrizzleError::ConversionError(#error_msg.to_string()))?,
        }
    }
}

/// Handle UUID fields
fn handle_uuid_field(
    idx: usize,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        crate::sqlite::field::SQLiteType::Blob => quote!(row.get_value(#idx)?.as_blob()),
        crate::sqlite::field::SQLiteType::Text => quote!(row.get_value(#idx)?.as_text()),
        _ => {
            return Err(Error::new_spanned(
                info.ident,
                "UUID fields must use BLOB or TEXT column types",
            ));
        }
    };

    let converter = match info.column_type {
        crate::sqlite::field::SQLiteType::Blob => {
            |v: TokenStream| quote!(#v.map(|v| uuid::Uuid::from_slice(v)).transpose()?)
        }
        crate::sqlite::field::SQLiteType::Text => {
            |v: TokenStream| quote!(#v.map(|v| uuid::Uuid::parse_str(v)).transpose()?)
        }
        _ => unreachable!(),
    };

    Ok(wrap_optional(converter(accessor), name, is_optional))
}

/// Handle integer fields
/// Handle integer fields (i64, i32, enums, bool, Option<_>)
fn handle_integer_field(
    idx: usize,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_integer());

    let is_not_i64 = !base_type_str.contains("i64");
    let is_bool = base_type_str.contains("bool");

    // Decide conversion per type
    let converter = if is_bool {
        |v: TokenStream| quote!(#v.map(|&v| v != 0))
    } else if info.is_enum || is_not_i64 {
        |v: TokenStream| quote!(#v.map(|&v| v.try_into()).transpose()?)
    } else {
        |v: TokenStream| quote!(#v)
    };

    Ok(wrap_optional(converter(accessor), name, is_optional))
}

/// Handle text fields
fn handle_text_field(
    idx: usize,
    name: &syn::Ident,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_text());

    let converter = if info.is_enum {
        |v: TokenStream| quote!(#v.map(|v| v.try_into()).transpose()?)
    } else {
        |v: TokenStream| quote!(#v.cloned())
    };

    Ok(wrap_optional(converter(accessor), name, is_optional))
}

/// Handle real/float fields
fn handle_real_field(
    idx: usize,
    name: &syn::Ident,
    _info: &FieldInfo,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_real());

    let converter = if base_type_str.contains("f32") {
        |v: TokenStream| quote!(#v.map(|&v| v.try_into()).transpose()?)
    } else {
        |v: TokenStream| quote!(#v.cloned())
    };

    Ok(wrap_optional(converter(accessor), name, is_optional))
}

/// Handle blob fields
fn handle_blob_field(
    idx: usize,
    name: &syn::Ident,
    _info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_blob());

    let converter = |v: TokenStream| quote!(#v.cloned());

    Ok(wrap_optional(converter(accessor), name, is_optional))
}

/// Check if the base type needs a reference (&str, &[u8], &i64, etc.)
fn needs_reference_type(base_type_str: &str) -> bool {
    base_type_str.starts_with('&')
}

/// Generate IntoValue implementations for enum fields
fn generate_enum_into_value_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let enum_fields: Vec<_> = ctx.field_infos.iter().filter(|info| info.is_enum).collect();
    
    if enum_fields.is_empty() {
        return Ok(quote!());
    }

    let mut enum_impls = Vec::new();
    
    // Keep track of processed enum types to avoid duplicates
    let mut processed_types = std::collections::HashSet::new();
    
    for info in enum_fields {
        let value_type = &info.base_type;
        let type_str = value_type.to_token_stream().to_string();
        
        // Skip if we've already processed this enum type
        if processed_types.contains(&type_str) {
            continue;
        }
        processed_types.insert(type_str);
        
        let impl_code = match info.column_type {
            crate::sqlite::field::SQLiteType::Integer => {
                quote! {
                    impl turso::IntoValue for #value_type {
                        fn into_value(self) -> turso::Result<turso::Value> {
                            let integer: i64 = self.into();
                            Ok(turso::Value::Integer(integer))
                        }
                    }
                }
            },
            crate::sqlite::field::SQLiteType::Text => {
                quote! {
                    impl turso::IntoValue for #value_type {
                        fn into_value(self) -> turso::Result<turso::Value> {
                            let text: &str = self.into();
                            Ok(turso::Value::Text(text.to_owned()))
                        }
                    }
                }
            },
            _ => {
                return Err(Error::new_spanned(
                    info.ident,
                    "Enum fields are only supported with INTEGER or TEXT column types for turso",
                ));
            }
        };
        
        enum_impls.push(impl_code);
    }

    Ok(quote! {
        #(#enum_impls)*
    })
}
