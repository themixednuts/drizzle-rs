use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Field, Result};

/// Generate turso field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    let field_type_str = field.ty.clone().into_token_stream().to_string();
    let is_optional = field_type_str.contains("Option");
    let base_type_str = extract_base_type(&field_type_str);

    let name = field_name;

    // Check for special attributes
    if has_json_attribute(field) {
        return handle_json_field(idx, name, is_optional);
    }

    if is_uuid_type(&field_type_str) {
        return handle_uuid_field(idx, name, is_optional);
    }

    // Handle standard types based on Rust type inference
    if base_type_str.contains("bool") {
        handle_bool_field(idx, name, is_optional)
    } else if is_integer_type(&base_type_str) {
        handle_integer_field(idx, name, is_optional, &base_type_str)
    } else if is_float_type(&base_type_str) {
        handle_float_field(idx, name, is_optional, &base_type_str)
    } else if base_type_str.contains("String") {
        handle_text_field(idx, name, is_optional)
    } else if base_type_str.contains("Vec") && base_type_str.contains("u8") {
        handle_blob_field(idx, name, is_optional)
    } else {
        // For unknown types (like enums), use TryFrom conversion
        handle_try_from_field(idx, name, is_optional, &field.ty)
    }
}

/// Handle JSON fields
fn handle_json_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_text());
    let converter = quote!(#accessor.map(|v| serde_json::from_str(v)).transpose()?);
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle UUID fields  
fn handle_uuid_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_blob());
    let converter = quote!(#accessor.map(|v| uuid::Uuid::from_slice(v)).transpose()?);
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle boolean fields
fn handle_bool_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_integer());
    let converter = quote!(#accessor.map(|&v| v != 0));
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle integer fields (i8, i16, i32, i64)
fn handle_integer_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_integer());

    let converter = if base_type_str.contains("i64") {
        quote!(#accessor.copied())
    } else {
        // For i32, i16, i8 - need conversion
        quote!(#accessor.map(|&v| v.try_into()).transpose()?)
    };

    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle float fields (f32, f64)
fn handle_float_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_real());

    let converter = if base_type_str.contains("f32") {
        quote!(#accessor.map(|&v| v as f32))
    } else {
        quote!(#accessor.cloned())
    };

    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle text/string fields
fn handle_text_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_text());
    let converter = quote!(#accessor.map(|s| s.to_string()));
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle blob/Vec<u8> fields
fn handle_blob_field(
    idx: usize,
    name: Option<&syn::Ident>,
    is_optional: bool,
) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_blob());
    let converter = quote!(#accessor.map(|v| v.to_vec()));
    Ok(wrap_optional(converter, name, is_optional))
}

fn wrap_optional(inner: TokenStream, name: Option<&syn::Ident>, is_optional: bool) -> TokenStream {
    if let Some(field_name) = name {
        // Named struct field
        if is_optional {
            quote! {
                #field_name: #inner,
            }
        } else {
            let error_msg = format!("Error converting required field `{}`", field_name);
            quote! {
                #field_name: #inner
                    .ok_or_else(|| ::drizzle_core::error::DrizzleError::ConversionError(#error_msg.to_string().into()))?,
            }
        }
    } else {
        // Tuple struct field
        if is_optional {
            quote! {
                #inner,
            }
        } else {
            quote! {
                #inner
                    .ok_or_else(|| ::drizzle_core::error::DrizzleError::ConversionError("Error converting tuple field".to_string().into()))?,
            }
        }
    }
}

/// Type checking helpers
fn is_uuid_type(type_str: &str) -> bool {
    type_str.contains("Uuid")
}

fn is_integer_type(base_type_str: &str) -> bool {
    base_type_str.eq("i8")
        || base_type_str.eq("i16")
        || base_type_str.eq("i32")
        || base_type_str.eq("i64")
        || base_type_str.eq("isize")
        || base_type_str.eq("u8")
        || base_type_str.eq("u16")
        || base_type_str.eq("u32")
        || base_type_str.eq("u64")
        || base_type_str.eq("usize")
}

fn is_float_type(base_type_str: &str) -> bool {
    base_type_str.eq("f32") || base_type_str.eq("f64")
}

/// Extract base type from Option<T> or T
fn extract_base_type(type_str: &str) -> String {
    if let Some(inner) = type_str.strip_prefix("Option < ")
        && let Some(inner) = inner.strip_suffix(" >")
    {
        return inner.trim().to_string();
    }
    type_str.to_string()
}

/// Check if field has json attribute
fn has_json_attribute(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().get_ident().is_some_and(|ident| ident == "json"))
}

/// Handle unknown types using DrizzleRow::get_column (for enums and custom types)
/// This uses the FromSQLiteValue trait for clean, unified conversion
fn handle_try_from_field(
    idx: usize,
    name: Option<&syn::Ident>,
    _is_optional: bool,
    field_type: &syn::Type,
) -> Result<TokenStream> {
    // Use DrizzleRow::get_column which handles all conversions via FromSQLiteValue trait
    // The trait implementation handles Option types automatically
    let accessor = quote! {
        {
            use ::drizzle_sqlite::traits::DrizzleRow;
            <_ as DrizzleRow>::get_column::<#field_type>(row, #idx)
        }
    };

    if let Some(field_name) = name {
        Ok(quote! {
            #field_name: #accessor?,
        })
    } else {
        Ok(quote! {
            #accessor?,
        })
    }
}
