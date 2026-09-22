//! JSON fields of `PostgreSQL` tables.
//!
//! A `json`/`jsonb` field keeps its payload type in the generated models and
//! converts through `drizzle::core::Json<Payload>`, whose codecs
//! drizzle-postgres implements once. The table macro therefore implements
//! nothing on payload types, so one payload type can back columns in several
//! tables, and foreign, path-qualified and generic payloads all work.

use super::context::MacroContext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Rejects JSON fields when drizzle-macros was built without `serde`.
///
/// Returns no tokens: JSON conversions are emitted inline by the models and
/// row decoders.
pub fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    if cfg!(feature = "serde") {
        return Ok(quote!());
    }

    match ctx.field_infos.iter().find(|info| info.is_json_payload()) {
        Some(field) => Err(syn::Error::new_spanned(
            &field.ident,
            "The 'serde' feature must be enabled to use JSON fields.\n\
             Add to Cargo.toml: drizzle = { version = \"*\", features = [\"serde\"] }",
        )),
        None => Ok(quote!()),
    }
}
