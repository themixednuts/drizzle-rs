//! Turso field assignment generation for FromRow derive.
//!
//! Uses the shared DrizzleRow::get_column infrastructure for unified type conversion.

use super::shared::{self, DriverJsonAccessor};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// Turso-specific JSON accessor configuration
pub(crate) struct TursoDriver;

impl DriverJsonAccessor for TursoDriver {
    fn json_accessor(idx: usize) -> TokenStream {
        quote! {
            {
                let text = row.get_value(#idx)?.as_text()
                    .ok_or_else(|| drizzle::error::DrizzleError::ConversionError("Expected text for JSON field".into()))?;
                serde_json::from_str(text).map_err(|e| drizzle::error::DrizzleError::ConversionError(e.to_string().into()))
            }
        }
    }

    fn error_type() -> TokenStream {
        quote!(DrizzleError)
    }
}

/// Generate turso field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    shared::generate_field_assignment::<TursoDriver>(idx, field, field_name)
}
