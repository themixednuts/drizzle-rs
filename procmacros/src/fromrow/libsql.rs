//! libsql field assignment generation for FromRow derive.
//!
//! Uses the shared DrizzleRow::get_column infrastructure for unified type conversion.

use super::shared::{self, DriverJsonAccessor};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Result};

/// libsql-specific JSON accessor configuration
pub(crate) struct LibsqlDriver;

impl DriverJsonAccessor for LibsqlDriver {
    fn json_accessor(idx: usize) -> TokenStream {
        let idx = idx as i32;
        quote! {
            {
                let json_str: String = row.get::<String>(#idx)?;
                serde_json::from_str(&json_str)
                    .map_err(|e| drizzle::error::DrizzleError::ConversionError(e.to_string().into()))
            }
        }
    }

    fn error_type() -> TokenStream {
        quote!(DrizzleError)
    }

    fn supports_name_lookup() -> bool {
        true
    }
}

/// Generate libsql field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&syn::Ident>,
) -> Result<TokenStream> {
    shared::generate_field_assignment::<LibsqlDriver>(idx, field, field_name)
}
