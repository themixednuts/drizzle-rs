//! Shared field assignment generation for libsql and turso FromRow derive.
//!
//! Both drivers use DrizzleRow::get_column for unified type conversion via FromSQLiteValue trait.
//! Only JSON handling differs between them.

use crate::paths;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// Driver-specific JSON accessor generation
#[allow(dead_code)]
pub(crate) trait DriverJsonAccessor {
    /// Generate the JSON field accessor for this driver
    fn json_accessor(idx: usize) -> TokenStream;

    /// Get the error type for this driver
    fn error_type() -> TokenStream;
}

/// Generate field assignment using the unified DrizzleRow::get_column interface.
///
/// This works for both libsql and turso since they both implement DrizzleRow.
pub(crate) fn generate_field_assignment<D: DriverJsonAccessor>(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    // Check for special attributes
    if has_json_attribute(field) {
        return handle_json_field::<D>(idx, field_name);
    }

    // All other types use DrizzleRow::get_column with FromSQLiteValue
    let drizzle_row = paths::sqlite::drizzle_row();
    let field_type = &field.ty;
    let accessor = quote! {
        {
            <_ as #drizzle_row>::get_column::<#field_type>(row, #idx)
        }
    };

    if let Some(field_name) = field_name {
        Ok(quote! {
            #field_name: #accessor?,
        })
    } else {
        Ok(quote! {
            #accessor?,
        })
    }
}

/// Handle JSON fields using driver-specific accessor
fn handle_json_field<D: DriverJsonAccessor>(
    idx: usize,
    name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    let accessor = D::json_accessor(idx);

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

/// Check if field has json attribute
pub(crate) fn has_json_attribute(field: &Field) -> bool {
    field.attrs.iter().any(|attr| {
        if attr.path().get_ident().is_some_and(|ident| ident == "json") {
            return true;
        }

        if !attr.path().is_ident("column") {
            return false;
        }

        match &attr.meta {
            syn::Meta::List(list) => {
                let tokens = list.tokens.to_string().to_ascii_lowercase();
                tokens.contains("json")
            }
            _ => false,
        }
    })
}
