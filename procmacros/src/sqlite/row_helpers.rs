use proc_macro2::TokenStream;
use quote::quote;

/// Generate text value conversion based on Rust type
fn generate_text_conversion(rust_type_str: &str, is_optional: bool, field_name: &str) -> TokenStream {
    if rust_type_str.contains("String") || rust_type_str.contains("&str") {
        let converter = quote!(value.as_text().cloned());
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("Vec") && rust_type_str.contains("u8") {
        // Convert text to Vec<u8>
        let converter = quote!(value.as_text().map(|v| v.as_bytes().to_vec()));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("i64") || rust_type_str.contains("i32") || rust_type_str.contains("i16") || rust_type_str.contains("i8") ||
             rust_type_str.contains("u64") || rust_type_str.contains("u32") || rust_type_str.contains("u16") || rust_type_str.contains("u8") {
        // Parse text to integer
        let converter = quote!(value.as_text().map(|v| v.parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("f64") || rust_type_str.contains("f32") {
        // Parse text to float
        let converter = quote!(value.as_text().map(|v| v.parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("bool") {
        // Parse text to bool
        let converter = quote!(value.as_text().map(|v| v.parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    } else {
        // For custom types that support FromStr, try to parse
        // Note: Vec<u8> should not reach here as it's handled above
        let converter = quote!(value.as_text().map(|v| v.parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    }
}

/// Generate integer value conversion based on Rust type
fn generate_integer_conversion(rust_type_str: &str, is_optional: bool, field_name: &str) -> TokenStream {
    if rust_type_str.contains("bool") {
        let converter = quote!(value.as_integer().map(|&v| v != 0));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("i64") {
        let converter = quote!(value.as_integer().copied());
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("String") || rust_type_str.contains("&str") {
        // Convert integer to string
        let converter = quote!(value.as_integer().map(|&v| v.to_string()));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("Vec") && rust_type_str.contains("u8") {
        // Convert integer to Vec<u8> via string
        let converter = quote!(value.as_integer().map(|&v| v.to_string().into_bytes()));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("f64") {
        // Convert integer to f64
        let converter = quote!(value.as_integer().map(|&v| v as f64));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("f32") {
        // Convert integer to f32
        let converter = quote!(value.as_integer().map(|&v| v as f32));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("i32") || rust_type_str.contains("i16") || rust_type_str.contains("i8") || 
             rust_type_str.contains("u64") || rust_type_str.contains("u32") || rust_type_str.contains("u16") || rust_type_str.contains("u8") {
        // For other integer types, use try_into
        let converter = quote!(value.as_integer().map(|&v| v.try_into()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    } else {
        // For custom types, convert integer to string and parse
        // Note: Vec<u8> should not reach here as it's handled above
        let converter = quote!(value.as_integer().map(|&v| v.to_string().parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    }
}

/// Generate real/float value conversion based on Rust type
fn generate_real_conversion(rust_type_str: &str, is_optional: bool, field_name: &str) -> TokenStream {
    if rust_type_str.contains("f64") {
        let converter = quote!(value.as_real().cloned());
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("f32") {
        let converter = quote!(value.as_real().map(|&v| v as f32));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("String") || rust_type_str.contains("&str") {
        // Convert real to string
        let converter = quote!(value.as_real().map(|&v| v.to_string()));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("Vec") && rust_type_str.contains("u8") {
        // Convert real to Vec<u8> via string
        let converter = quote!(value.as_real().map(|&v| v.to_string().into_bytes()));
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("i64") || rust_type_str.contains("i32") || rust_type_str.contains("i16") || rust_type_str.contains("i8") ||
             rust_type_str.contains("u64") || rust_type_str.contains("u32") || rust_type_str.contains("u16") || rust_type_str.contains("u8") {
        // Convert real to integer
        let converter = quote!(value.as_real().map(|&v| v as i64).map(|v| v.try_into()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("bool") {
        // Convert real to bool (non-zero is true)
        let converter = quote!(value.as_real().map(|&v| v != 0.0));
        wrap_optional(converter, field_name, is_optional)
    } else {
        // For custom types, convert real to string and parse
        // Note: Vec<u8> should not reach here as it's handled above
        let converter = quote!(value.as_real().map(|&v| v.to_string().parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    }
}

/// Generate blob value conversion based on Rust type
fn generate_blob_conversion(rust_type_str: &str, is_optional: bool, field_name: &str) -> TokenStream {
    if rust_type_str.contains("Vec") && rust_type_str.contains("u8") || rust_type_str.contains("&[u8]") {
        let converter = quote!(value.as_blob().cloned());
        wrap_optional(converter, field_name, is_optional)
    } else if rust_type_str.contains("String") || rust_type_str.contains("&str") {
        // Convert blob to string
        let converter = quote!(value.as_blob().map(|v| String::from_utf8_lossy(v).to_string()));
        wrap_optional(converter, field_name, is_optional)
    } else {
        // For custom types, convert blob to string and parse
        let converter = quote!(value.as_blob().map(|v| String::from_utf8_lossy(v).parse()).transpose()?);
        wrap_optional(converter, field_name, is_optional)
    }
}

/// Helper function to wrap optional fields just like in table implementations
fn wrap_optional(inner: TokenStream, field_name: &str, is_optional: bool) -> TokenStream {
    if is_optional {
        quote! { #inner }
    } else {
        let error_msg = format!("Error converting required field `{}`", field_name);
        quote! {
            #inner.ok_or_else(|| ::drizzle_rs::error::DrizzleError::ConversionError(#error_msg.to_string()))?
        }
    }
}

/// Generates type-aware value extraction for Turso rows using runtime type checking
/// This uses .is_text(), .is_real(), .is_integer(), .is_blob() to determine actual SQLite types
pub fn generate_turso_value_extraction(idx: usize, rust_type_str: &str) -> TokenStream {
    let is_optional = rust_type_str.contains("Option");
    let field_name = format!("field_{}", idx);
    
    let text_conversion = generate_text_conversion(rust_type_str, is_optional, &field_name);
    let integer_conversion = generate_integer_conversion(rust_type_str, is_optional, &field_name);
    let real_conversion = generate_real_conversion(rust_type_str, is_optional, &field_name);
    let blob_conversion = generate_blob_conversion(rust_type_str, is_optional, &field_name);
    
    let null_handling = if is_optional {
        quote!(None)
    } else {
        let error_msg = format!("Error converting required field `{}`", field_name);
        quote!(return Err(::drizzle_rs::error::DrizzleError::ConversionError(#error_msg.to_string())))
    };
    
    // Use runtime type checking instead of guessing from Rust types
    quote! {
        {
            let value = row.get_value(#idx)?;
            if value.is_text() {
                #text_conversion
            } else if value.is_integer() {
                #integer_conversion
            } else if value.is_real() {
                #real_conversion
            } else if value.is_blob() {
                #blob_conversion
            } else {
                // Handle NULL values
                #null_handling
            }
        }
    }
}

/// Generates type-aware value extraction for libsql rows using runtime type checking
/// This uses .is_text(), .is_real(), .is_integer(), .is_blob() to determine actual SQLite types
pub fn generate_libsql_value_extraction(idx: i32, rust_type_str: &str) -> TokenStream {
    let is_optional = rust_type_str.contains("Option");
    let field_name = format!("field_{}", idx);
    
    let text_conversion = generate_text_conversion(rust_type_str, is_optional, &field_name);
    let integer_conversion = generate_integer_conversion(rust_type_str, is_optional, &field_name);
    let real_conversion = generate_real_conversion(rust_type_str, is_optional, &field_name);
    let blob_conversion = generate_blob_conversion(rust_type_str, is_optional, &field_name);
    
    let null_handling = if is_optional {
        quote!(None)
    } else {
        let error_msg = format!("Error converting required field `{}`", field_name);
        quote!(return Err(::drizzle_rs::error::DrizzleError::ConversionError(#error_msg.to_string())))
    };
    
    // Use runtime type checking instead of guessing from Rust types
    quote! {
        {
            let value = row.get_value(#idx)?;
            if value.is_text() {
                #text_conversion
            } else if value.is_integer() {
                #integer_conversion
            } else if value.is_real() {
                #real_conversion
            } else if value.is_blob() {
                #blob_conversion
            } else {
                // Handle NULL values
                #null_handling
            }
        }
    }
}