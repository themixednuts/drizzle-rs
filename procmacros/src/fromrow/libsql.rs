use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Field, Result};

/// Generate libsql field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    let field_type_str = field.ty.clone().into_token_stream().to_string();
    let is_optional = field_type_str.contains("Option");
    let base_type_str = extract_base_type(&field_type_str);
    let idx = idx as i32;

    let field_ident = field_name.cloned();

    // Check for special attributes
    if has_json_attribute(field) {
        return handle_json_field(idx, field_ident, is_optional);
    }

    if is_uuid_type(&field_type_str) {
        return handle_uuid_field(idx, field_ident, is_optional);
    }

    // Handle standard types based on Rust type inference
    if base_type_str.contains("bool") {
        handle_bool_field(idx, field_ident, is_optional)
    } else if is_integer_type(&base_type_str) {
        handle_integer_field(idx, field_ident, is_optional, &base_type_str)
    } else if is_float_type(&base_type_str) {
        handle_float_field(idx, field_ident, is_optional, &base_type_str)
    } else if base_type_str.contains("String") {
        handle_text_field(idx, field_ident, is_optional)
    } else if base_type_str.contains("Vec") && base_type_str.contains("u8") {
        handle_blob_field(idx, field_ident, is_optional)
    } else {
        // Default to string for unknown types
        handle_text_field(idx, field_ident, is_optional)
    }
}

/// Handle JSON fields
fn handle_json_field(idx: i32, name: Option<syn::Ident>, is_optional: bool) -> Result<TokenStream> {
    let accessor = if is_optional {
        quote!(row.get::<Option<String>>(#idx).map(|opt| opt.and_then(|v| serde_json::from_str(&v).ok())))
    } else {
        quote!(serde_json::from_str(&row.get::<String>(#idx)?).map_err(Into::into))
    };

    Ok(format_field_assignment(name, accessor))
}

/// Handle UUID fields  
fn handle_uuid_field(idx: i32, name: Option<syn::Ident>, is_optional: bool) -> Result<TokenStream> {
    // Default to BLOB type for UUID in FromRow (since we don't have column type info)
    let accessor = if is_optional {
        quote!(row.get::<Option<[u8;16]>>(#idx).map(|opt| opt.map(::uuid::Uuid::from_bytes)))
    } else {
        quote!(row.get::<[u8;16]>(#idx).map(::uuid::Uuid::from_bytes))
    };

    Ok(format_field_assignment(name, accessor))
}

/// Helper function to format field assignments for both named and tuple structs
fn format_field_assignment(
    name: Option<syn::Ident>,
    accessor: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if let Some(field_name) = name {
        quote! {
            #field_name: #accessor?,
        }
    } else {
        // Tuple struct
        quote! {
            #accessor?,
        }
    }
}

/// Handle boolean fields
fn handle_bool_field(idx: i32, name: Option<syn::Ident>, is_optional: bool) -> Result<TokenStream> {
    let accessor = if is_optional {
        quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.map(|v| v != 0)))
    } else {
        quote!(row.get::<i64>(#idx).map(|v| v != 0))
    };
    Ok(format_field_assignment(name, accessor))
}

/// Handle integer fields (i8, i16, i32, i64)
fn handle_integer_field(
    idx: i32,
    name: Option<syn::Ident>,
    is_optional: bool,
    base_type_str: &str,
) -> Result<TokenStream> {
    let is_not_i64 = !base_type_str.eq("i64");
    let is_bool = base_type_str.eq("bool");

    let accessor = if is_bool {
        if is_optional {
            quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.map(|v| v != 0)))
        } else {
            quote!(row.get::<i64>(#idx).map(|v| v != 0))
        }
    } else if is_not_i64 {
        if is_optional {
            quote!(row.get::<Option<i64>>(#idx).map(|opt| opt.and_then(|v| v.try_into().ok())))
        } else {
            quote!(row.get::<i64>(#idx).map(TryInto::try_into)?)
        }
    } else {
        quote!(row.get(#idx))
    };

    Ok(format_field_assignment(name, accessor))
}

/// Handle float fields (f32, f64)
fn handle_float_field(
    idx: i32,
    name: Option<syn::Ident>,
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

    Ok(format_field_assignment(name, accessor))
}

/// Handle text/string fields
fn handle_text_field(idx: i32, name: Option<syn::Ident>, is_optional: bool) -> Result<TokenStream> {
    let accessor = if is_optional {
        quote!(row.get::<Option<String>>(#idx))
    } else {
        quote!(row.get::<String>(#idx))
    };
    Ok(format_field_assignment(name, accessor))
}

/// Handle blob/Vec<u8> fields
fn handle_blob_field(idx: i32, name: Option<syn::Ident>, is_optional: bool) -> Result<TokenStream> {
    let accessor = if is_optional {
        quote!(row.get::<Option<Vec<u8>>>(#idx))
    } else {
        quote!(row.get::<Vec<u8>>(#idx))
    };
    Ok(format_field_assignment(name, accessor))
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
    base_type_str.contains("f32") || base_type_str.contains("f64")
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
