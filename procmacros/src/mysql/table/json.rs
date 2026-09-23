//! JSON fields of MySQL tables.
//!
//! A `JSON` field keeps its payload type in the generated models and converts
//! through `drizzle::core::Json<Payload>`, which implements
//! `DrizzleMySQLColumn` once in drizzle-mysql. The table macro therefore
//! implements nothing on payload types, so one payload type can back columns
//! in several tables, and foreign, path-qualified and generic payloads all
//! work.

use super::context::MacroContext;
use proc_macro2::TokenStream;
use syn::Result;

/// Rejects JSON payload fields when drizzle-macros was built without `serde`.
///
/// Returns no tokens: JSON conversions are emitted inline by the models, the
/// column definitions and the row decoders.
pub fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    if cfg!(feature = "serde") {
        return Ok(TokenStream::new());
    }
    match ctx.field_infos.iter().find(|field| field.is_json_payload()) {
        Some(field) => Err(syn::Error::new_spanned(
            &field.ident,
            "the `serde` feature is required for custom MySQL JSON fields",
        )),
        None => Ok(TokenStream::new()),
    }
}
