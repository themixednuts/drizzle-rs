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

    if field_name.is_none() {
        // For tuple structs, just return the value without field name
        return Ok(generate_tuple_value(idx, &base_type_str, &field_type_str));
    }

    let name = field_name.unwrap();

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
        // Default to string for unknown types
        handle_text_field(idx, name, is_optional)
    }
}

/// Handle JSON fields
fn handle_json_field(idx: usize, name: &syn::Ident, is_optional: bool) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_text());
    let converter = quote!(#accessor.map(|v| serde_json::from_str(v)).transpose()?);
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle UUID fields  
fn handle_uuid_field(idx: usize, name: &syn::Ident, is_optional: bool) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_blob());
    let converter = quote!(#accessor.map(|v| uuid::Uuid::from_slice(v)).transpose()?);
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle boolean fields
fn handle_bool_field(idx: usize, name: &syn::Ident, is_optional: bool) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_integer());
    let converter = quote!(#accessor.map(|&v| v != 0));
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle integer fields (i8, i16, i32, i64)
fn handle_integer_field(
    idx: usize,
    name: &syn::Ident,
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
    name: &syn::Ident,
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
fn handle_text_field(idx: usize, name: &syn::Ident, is_optional: bool) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_text());
    let converter = quote!(#accessor.cloned());
    Ok(wrap_optional(converter, name, is_optional))
}

/// Handle blob/Vec<u8> fields
fn handle_blob_field(idx: usize, name: &syn::Ident, is_optional: bool) -> Result<TokenStream> {
    let accessor = quote!(row.get_value(#idx)?.as_blob());
    let converter = quote!(#accessor.map(|v| v.to_vec()));
    Ok(wrap_optional(converter, name, is_optional))
}

fn wrap_optional(inner: TokenStream, name: &syn::Ident, is_optional: bool) -> TokenStream {
    if is_optional {
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

/// Type checking helpers
fn is_uuid_type(type_str: &str) -> bool {
    type_str.contains("Uuid")
}

fn is_integer_type(base_type_str: &str) -> bool {
    base_type_str.contains("i8")
        || base_type_str.contains("i16")
        || base_type_str.contains("i32")
        || base_type_str.contains("i64")
}

fn is_float_type(base_type_str: &str) -> bool {
    base_type_str.contains("f32") || base_type_str.contains("f64")
}

/// Extract base type from Option<T> or T
fn extract_base_type(type_str: &str) -> String {
    if let Some(inner) = type_str.strip_prefix("Option < ") {
        if let Some(inner) = inner.strip_suffix(" >") {
            return inner.trim().to_string();
        }
    }
    type_str.to_string()
}

/// Check if field has json attribute
fn has_json_attribute(field: &Field) -> bool {
    field.attrs.iter().any(|attr| {
        attr.path()
            .get_ident()
            .map_or(false, |ident| ident == "json")
    })
}

/// Generate tuple struct value assignment (no field name)
fn generate_tuple_value(idx: usize, base_type_str: &str, field_type_str: &str) -> TokenStream {
    if is_uuid_type(field_type_str) {
        return quote!(row.get_value(#idx)?.as_blob().map(|v| uuid::Uuid::from_slice(v)).transpose()?.unwrap_or_default(),);
    }

    if base_type_str.contains("bool") {
        quote!(row.get_value(#idx)?.as_integer().map(|&v| v != 0).unwrap_or_default(),)
    } else if is_integer_type(base_type_str) {
        if base_type_str.contains("i64") {
            quote!(row.get_value(#idx)?.as_integer().copied().unwrap_or_default(),)
        } else {
            quote!(row.get_value(#idx)?.as_integer().map(|&v| v.try_into()).transpose()?.unwrap_or_default(),)
        }
    } else if is_float_type(base_type_str) {
        if base_type_str.contains("f32") {
            quote!(row.get_value(#idx)?.as_real().map(|&v| v as f32).unwrap_or_default(),)
        } else {
            quote!(row.get_value(#idx)?.as_real().cloned().unwrap_or_default(),)
        }
    } else if base_type_str.contains("String") {
        quote!(row.get_value(#idx)?.as_text().cloned().unwrap_or_default(),)
    } else if base_type_str.contains("Vec") && base_type_str.contains("u8") {
        quote!(row.get_value(#idx)?.as_blob().map(|v| v.to_vec()).unwrap_or_default(),)
    } else {
        // Default to string
        quote!(row.get_value(#idx)?.as_text().cloned().unwrap_or_default(),)
    }
}
