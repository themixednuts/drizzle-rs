use super::context::MacroContext;
use crate::sqlite::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::HashMap;
use syn::Result;

// Common SQLite documentation URLs for error messages and macro docs
const SQLITE_JSON_URL: &str = "https://sqlite.org/json1.html";

/// Generates `FromSql` and `ToSql` impls for JSON fields.
pub(crate) fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    // Create a filter for JSON fields
    let json_fields: Vec<_> = ctx.field_infos.iter().filter(|info| info.is_json).collect();

    // If no JSON fields, return an empty TokenStream
    if json_fields.is_empty() {
        return Ok(quote!());
    }

    // Check that serde feature is enabled for JSON fields
    if !cfg!(feature = "serde") {
        let first_json_field = json_fields.first().unwrap();
        return Err(syn::Error::new_spanned(
            first_json_field.ident,
            format!(
                "The 'serde' feature must be enabled to use JSON fields.\n\
             Add to Cargo.toml: drizzle = {{ version = \"*\", features = [\"serde\"] }}\n\
             See: {SQLITE_JSON_URL}"
            ),
        ));
    }

    // Track JSON type to SQLite storage type mapping and detect conflicts
    use crate::sqlite::field::SQLiteType;

    let mut json_type_storage: HashMap<String, (SQLiteType, &FieldInfo)> = HashMap::new();

    // Check for conflicts and build the mapping
    for info in json_fields {
        let base_type_str = info.base_type.to_token_stream().to_string();

        if let Some((existing_storage, existing_field)) = json_type_storage.get(&base_type_str) {
            // Check if the storage type conflicts
            if *existing_storage != info.column_type {
                return Err(syn::Error::new_spanned(
                    info.ident,
                    format!(
                        "JSON type '{}' is used with conflicting storage types. \
                         Field '{}' uses {:?}, but field '{}' uses {:?}. \
                         Each JSON type must use the same storage type (either TEXT or BLOB) throughout the codebase.",
                        base_type_str,
                        existing_field.ident,
                        existing_storage,
                        info.ident,
                        info.column_type
                    ),
                ));
            }
        } else {
            // First occurrence of this JSON type
            json_type_storage.insert(base_type_str, (info.column_type.clone(), info));
        }
    }

    // Generate core SQLiteValue implementations (needed for all drivers)
    let core_impls = if json_type_storage.is_empty() {
        vec![]
    } else {
        json_type_storage.iter().map(|(_, (storage_type, info))| {
            let struct_name = info.base_type;
            let core_conversion = match storage_type {
                SQLiteType::Text => quote! {
                    let json = serde_json::to_string(&self)?;
                    Ok(::drizzle::sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Owned(json)))
                },
                SQLiteType::Blob => quote! {
                    let json = serde_json::to_vec(&self)?;
                    Ok(::drizzle::sqlite::values::SQLiteValue::Blob(::std::borrow::Cow::Owned(json)))
                },
                _ => return Err(syn::Error::new_spanned(
                    info.ident,
                    "JSON fields must use either TEXT or BLOB column types"
                )),
            };

            Ok(quote! {
                // Core TryInto implementation for SQLiteValue (needed for all drivers)
                impl<'a> ::std::convert::TryInto<::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_name {
                    type Error = serde_json::Error;

                    fn try_into(self) -> Result<::drizzle::sqlite::values::SQLiteValue<'a>, Self::Error> {
                        #core_conversion
                    }
                }
            })
        }).collect::<Result<Vec<_>>>()?
    };

    // Generate rusqlite-specific implementations
    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = super::rusqlite::generate_json_impls(&json_type_storage)?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls: Vec<TokenStream> = vec![];

    // Generate turso-specific implementations
    #[cfg(feature = "turso")]
    let turso_json_impls = super::turso::generate_json_impls(&json_type_storage)?;

    #[cfg(not(feature = "turso"))]
    let turso_json_impls: Vec<TokenStream> = vec![];

    // Generate libsql-specific implementations
    #[cfg(feature = "libsql")]
    let libsql_json_impls = super::libsql::generate_json_impls(&json_type_storage)?;

    #[cfg(not(feature = "libsql"))]
    let libsql_json_impls: Vec<TokenStream> = vec![];

    let json_types_impl = quote! {
        #(#core_impls)*
        #(#rusqlite_impls)*
        #(#turso_json_impls)*
        #(#libsql_json_impls)*
    };

    Ok(json_types_impl)
}
