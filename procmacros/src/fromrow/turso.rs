//! Turso field assignment generation for FromRow derive
//!
//! Uses DrizzleRow::get_column for unified type conversion via FromSQLiteValue trait.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// Generate turso field assignment for FromRow derive
///
/// All types are handled uniformly via DrizzleRow::get_column which uses the
/// FromSQLiteValue trait for type-safe conversion.
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    // Check for special attributes
    if has_json_attribute(field) {
        return handle_json_field(idx, field_name);
    }

    // All other types use DrizzleRow::get_column with FromSQLiteValue
    let field_type = &field.ty;
    let accessor = quote! {
        {
            use ::drizzle_sqlite::traits::DrizzleRow;
            <_ as DrizzleRow>::get_column::<#field_type>(row, #idx)
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

/// Handle JSON fields (special case requiring serde deserialization)
fn handle_json_field(idx: usize, name: Option<&syn::Ident>) -> Result<TokenStream> {
    let accessor = quote! {
        {
            let text = row.get_value(#idx)?.as_text()
                .ok_or_else(|| ::drizzle_core::error::DrizzleError::ConversionError("Expected text for JSON field".into()))?;
            serde_json::from_str(text).map_err(|e| ::drizzle_core::error::DrizzleError::ConversionError(e.to_string().into()))
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

/// Check if field has json attribute
fn has_json_attribute(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().get_ident().is_some_and(|ident| ident == "json"))
}
