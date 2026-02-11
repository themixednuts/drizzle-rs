//! JSON type implementations for PostgreSQL
//!
//! This module generates TryInto<PostgresValue> implementations for custom JSON types
//! (structs marked with #[column(json)] or #[column(jsonb)]).

use super::context::MacroContext;
use crate::common::type_is_json_value;
use crate::postgres::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::Result;

/// Generate TryInto<PostgresValue> implementations for custom JSON types.
///
/// This enables custom structs (that implement Serialize) to be used with the
/// blanket `From<T> for PostgresInsertValue` impl which requires `T: TryInto<PostgresValue>`.
pub(crate) fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    // Create a filter for JSON fields with custom types (not serde_json::Value)
    let json_fields: Vec<_> = ctx
        .field_infos
        .iter()
        .filter(|info| {
            if !info.is_json && !info.is_jsonb {
                return false;
            }
            // Keep only custom types (exclude serde_json::Value itself)
            !type_is_json_value(&info.base_type)
        })
        .collect();

    // If no custom JSON fields, return an empty TokenStream
    if json_fields.is_empty() {
        return Ok(quote!());
    }

    // Check that serde feature is enabled for JSON fields
    if !cfg!(feature = "serde") {
        let first_json_field = json_fields.first().unwrap();
        return Err(syn::Error::new_spanned(
            &first_json_field.ident,
            "The 'serde' feature must be enabled to use JSON fields.\n\
             Add to Cargo.toml: drizzle = { version = \"*\", features = [\"serde\"] }",
        ));
    }

    // Track JSON type to PostgreSQL storage type mapping and detect conflicts
    let mut json_type_storage: HashMap<String, (bool, &FieldInfo)> = HashMap::new();

    // Check for conflicts and build the mapping
    // bool = true means jsonb, false means json
    for info in &json_fields {
        let base_type_str = info.base_type.to_token_stream().to_string();
        let is_jsonb = info.is_jsonb;

        if let Some((existing_is_jsonb, existing_field)) = json_type_storage.get(&base_type_str) {
            // Check if the storage type conflicts
            if *existing_is_jsonb != is_jsonb {
                return Err(syn::Error::new_spanned(
                    &info.ident,
                    format!(
                        "JSON type '{}' is used with conflicting storage types. \
                         Field '{}' uses {}, but field '{}' uses {}. \
                         Each JSON type must use the same storage type (either JSON or JSONB) throughout the codebase.",
                        base_type_str,
                        existing_field.ident,
                        if *existing_is_jsonb { "JSONB" } else { "JSON" },
                        info.ident,
                        if is_jsonb { "JSONB" } else { "JSON" }
                    ),
                ));
            }
        } else {
            // First occurrence of this JSON type
            json_type_storage.insert(base_type_str, (is_jsonb, info));
        }
    }

    // Generate TryInto<PostgresValue> implementations for each unique JSON type
    let core_impls: Vec<TokenStream> = json_type_storage
        .iter()
        .map(|(_, (is_jsonb, info))| {
            let struct_name = &info.base_type;
            let variant = if *is_jsonb {
                quote!(Jsonb)
            } else {
                quote!(Json)
            };

            quote! {
                // TryInto<PostgresValue> implementation for custom JSON type
                // This enables the blanket From<T> for PostgresInsertValue to work
                impl<'a> ::std::convert::TryInto<PostgresValue<'a>> for #struct_name {
                    type Error = ::serde_json::Error;

                    fn try_into(self) -> ::std::result::Result<PostgresValue<'a>, Self::Error> {
                        let json_val = ::serde_json::to_value(&self)?;
                        Ok(PostgresValue::#variant(json_val))
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        #(#core_impls)*
    })
}
