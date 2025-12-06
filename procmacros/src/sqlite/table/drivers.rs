//! Shared driver infrastructure for SQLite row conversion.
//!
//! This module provides a unified approach to generating TryFrom implementations
//! for different SQLite drivers (rusqlite, libsql, turso), reducing code duplication
//! and ensuring consistent behavior.

use crate::sqlite::field::{FieldInfo, SQLiteType, TypeCategory};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Result;

use super::errors;

// =============================================================================
// Driver Configuration Trait
// =============================================================================

/// Configuration for a specific SQLite driver's row access patterns.
///
/// Each driver implements this to provide its specific syntax for:
/// - Row type path (e.g., `::rusqlite::Row<'_>`)
/// - Value extraction methods
/// - Optional wrapping for non-optional fields
pub(crate) trait DriverConfig {
    /// The row type path for this driver
    fn row_type() -> TokenStream;

    /// Generate accessor for integer column by index
    fn integer_accessor(idx: &TokenStream) -> TokenStream;

    /// Generate accessor for text column by index
    fn text_accessor(idx: &TokenStream) -> TokenStream;

    /// Generate accessor for blob column by index
    fn blob_accessor(idx: &TokenStream) -> TokenStream;

    /// Generate accessor for real column by index
    fn real_accessor(idx: &TokenStream) -> TokenStream;

    /// Wrap the conversion result for non-optional fields
    fn wrap_required(inner: TokenStream, field_name: &syn::Ident) -> TokenStream;

    /// Wrap the conversion result for optional fields
    fn wrap_optional(inner: TokenStream, field_name: &syn::Ident) -> TokenStream;
}

// =============================================================================
// Shared Field Conversion Logic
// =============================================================================

/// Generate field conversion code for any driver.
///
/// This is the main entry point that dispatches to type-specific handlers
/// based on the field's TypeCategory.
pub(crate) fn generate_field_conversion<D: DriverConfig>(
    idx: usize,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let name = info.ident;
    let idx_tokens = quote!(#idx);

    // Check for unsupported reference types
    let base_type_str = info.base_type.to_token_stream().to_string();
    if base_type_str.starts_with('&') {
        return Err(syn::Error::new_spanned(
            info.ident,
            errors::conversion::REFERENCE_TYPE_UNSUPPORTED,
        ));
    }

    // Dispatch based on type category
    let converted = match info.type_category() {
        TypeCategory::Json => generate_json_conversion::<D>(&idx_tokens, info, is_optional)?,
        TypeCategory::Uuid => generate_uuid_conversion::<D>(&idx_tokens, info, is_optional)?,
        TypeCategory::Enum => generate_enum_conversion::<D>(&idx_tokens, info, is_optional)?,
        TypeCategory::ArrayString => {
            generate_arraystring_conversion::<D>(&idx_tokens, info, is_optional)?
        }
        TypeCategory::ArrayVec => {
            generate_arrayvec_conversion::<D>(&idx_tokens, info, is_optional)?
        }
        TypeCategory::String => generate_string_conversion::<D>(&idx_tokens, info, is_optional)?,
        TypeCategory::Blob => generate_blob_conversion::<D>(&idx_tokens, info, is_optional)?,
        TypeCategory::Primitive => {
            generate_primitive_conversion::<D>(&idx_tokens, info, is_optional)?
        }
    };

    // Wrap with appropriate optional/required handling
    let wrapped = if is_optional {
        D::wrap_optional(converted, name)
    } else {
        D::wrap_required(converted, name)
    };

    Ok(wrapped)
}

// =============================================================================
// Type-Specific Conversion Generators
// =============================================================================

#[allow(dead_code)]
fn generate_json_conversion<D: DriverConfig>(
    idx: &TokenStream,
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
        SQLiteType::Text => D::text_accessor(idx),
        SQLiteType::Blob => D::blob_accessor(idx),
        _ => {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::json::INVALID_COLUMN_TYPE,
            ));
        }
    };

    let deserialize = match info.column_type {
        SQLiteType::Text => quote!(serde_json::from_str(v)),
        SQLiteType::Blob => quote!(serde_json::from_slice(v)),
        _ => unreachable!(),
    };

    // Both optional and non-optional JSON fields use the same pattern since
    // the JSON deserialization needs to handle the Option wrapper uniformly
    let _ = is_optional;
    Ok(quote!(#accessor.map(|v| #deserialize).transpose()?))
}

#[allow(dead_code)]
fn generate_uuid_conversion<D: DriverConfig>(
    idx: &TokenStream,
    info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    let accessor = match info.column_type {
        SQLiteType::Blob => D::blob_accessor(idx),
        SQLiteType::Text => D::text_accessor(idx),
        _ => {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::uuid::INVALID_COLUMN_TYPE,
            ));
        }
    };

    let parse = match info.column_type {
        SQLiteType::Blob => quote!(::uuid::Uuid::from_slice(v)),
        SQLiteType::Text => quote!(::uuid::Uuid::parse_str(v)),
        _ => unreachable!(),
    };

    Ok(quote!(#accessor.map(|v| #parse).transpose()?))
}

#[allow(dead_code)]
fn generate_enum_conversion<D: DriverConfig>(
    idx: &TokenStream,
    info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    match info.column_type {
        SQLiteType::Integer => {
            let accessor = D::integer_accessor(idx);
            Ok(quote!(#accessor.map(|&v| v.try_into()).transpose()?))
        }
        SQLiteType::Text => {
            let accessor = D::text_accessor(idx);
            Ok(quote!(#accessor.map(|v| v.try_into()).transpose()?))
        }
        _ => Err(syn::Error::new_spanned(
            info.ident,
            errors::enums::INVALID_COLUMN_TYPE,
        )),
    }
}

#[allow(dead_code)]
fn generate_arraystring_conversion<D: DriverConfig>(
    idx: &TokenStream,
    info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    let accessor = D::text_accessor(idx);
    let base_type = info.base_type;

    Ok(quote!(
        #accessor
            .map(|v| <#base_type as FromSQLiteValue>::from_sqlite_text(v))
            .transpose()?
    ))
}

#[allow(dead_code)]
fn generate_arrayvec_conversion<D: DriverConfig>(
    idx: &TokenStream,
    info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    let accessor = D::blob_accessor(idx);
    let base_type = info.base_type;

    Ok(quote!(
        #accessor
            .map(|v| <#base_type as FromSQLiteValue>::from_sqlite_blob(v))
            .transpose()?
    ))
}

#[allow(dead_code)]
fn generate_string_conversion<D: DriverConfig>(
    idx: &TokenStream,
    _info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    let accessor = D::text_accessor(idx);
    Ok(quote!(#accessor.cloned()))
}

#[allow(dead_code)]
fn generate_blob_conversion<D: DriverConfig>(
    idx: &TokenStream,
    _info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    let accessor = D::blob_accessor(idx);
    Ok(quote!(#accessor.cloned()))
}

#[allow(dead_code)]
fn generate_primitive_conversion<D: DriverConfig>(
    idx: &TokenStream,
    info: &FieldInfo,
    _is_optional: bool,
) -> Result<TokenStream> {
    let base_type_str = info.base_type.to_token_stream().to_string();

    match info.column_type {
        SQLiteType::Integer => {
            let accessor = D::integer_accessor(idx);
            let is_bool = base_type_str.contains("bool");
            let is_i64 = base_type_str.contains("i64");

            if is_bool {
                Ok(quote!(#accessor.map(|&v| v != 0)))
            } else if is_i64 {
                Ok(quote!(#accessor.copied()))
            } else {
                // Other integer types need conversion
                Ok(quote!(#accessor.map(|&v| v.try_into()).transpose()?))
            }
        }
        SQLiteType::Real => {
            let accessor = D::real_accessor(idx);
            let is_f32 = base_type_str.contains("f32");

            if is_f32 {
                Ok(quote!(#accessor.map(|&v| v as f32)))
            } else {
                Ok(quote!(#accessor.copied()))
            }
        }
        SQLiteType::Text => {
            let accessor = D::text_accessor(idx);
            Ok(quote!(#accessor.cloned()))
        }
        SQLiteType::Blob => {
            let accessor = D::blob_accessor(idx);
            Ok(quote!(#accessor.cloned()))
        }
        SQLiteType::Numeric => {
            // Treat as integer
            let accessor = D::integer_accessor(idx);
            Ok(quote!(#accessor.copied()))
        }
        SQLiteType::Any => {
            // Default to text
            let accessor = D::text_accessor(idx);
            Ok(quote!(#accessor.cloned()))
        }
    }
}
