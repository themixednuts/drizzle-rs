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
        quote!(serde_json::from_str(&row.get::<String>(#idx)?).map_err(Into::into))
    }

    fn error_type() -> TokenStream {
        quote!(drizzle_core::error::DrizzleError)
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
