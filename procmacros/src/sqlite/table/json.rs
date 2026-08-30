use super::context::MacroContext;
use crate::common::type_is_json_value;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use crate::sqlite::{field::FieldInfo, generators::generate_to_sql};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::BTreeMap;
use syn::{Result, Type, TypePath};

// Common SQLite documentation URLs for error messages and macro docs
const SQLITE_JSON_URL: &str = "https://sqlite.org/json1.html";

pub fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    // Create a filter for JSON fields
    let json_fields: Vec<_> = ctx
        .field_infos
        .iter()
        .filter(|info| info.is_json && !type_is_json_value(info.base_type))
        .collect();

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

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sqlite_value = sqlite_paths::sqlite_value();
    let expression = sqlite_paths::expr();

    let mut json_types: BTreeMap<String, &FieldInfo> = BTreeMap::new();
    for info in json_fields {
        let base_type_str = info.base_type.to_token_stream().to_string();
        json_types.entry(base_type_str).or_insert(info);
    }

    // Generate core SQLiteValue implementations (needed for all drivers)
    let core_impls = json_types
            .values()
            .map(|info| {
                let struct_name = info.base_type;
                Ok(quote! {
                    // Core TryInto implementation for SQLiteValue (needed for all drivers)
                    impl<'a> ::std::convert::TryInto<#sqlite_value<'a>> for #struct_name {
                        type Error = ::serde_json::Error;

                        fn try_into(self) -> ::std::result::Result<#sqlite_value<'a>, Self::Error> {
                            let json_data = ::serde_json::to_string(&self)?;
                            ::std::result::Result::Ok(#sqlite_value::Text(::std::borrow::Cow::Owned(json_data)))
                        }
                    }
                })
            })
            .collect::<Result<Vec<_>>>()?;

    let to_sql_impl = json_types.values().map(|f| {
        let Type::Path(TypePath { path, qself: None }) = f.base_type else {
            return quote! {};
        };

        let Some(struct_ident) = path.segments.last().map(|s| &s.ident) else {
            return quote! {};
        };
        generate_to_sql(
            struct_ident,
            &quote! {
                use ::std::borrow::Cow;
                ::serde_json::to_string(self)
                    .map(#sqlite_value::from)
                    .map(Cow::Owned)
                    .map(#sql::param)
                    .map(|sql| #expression::json(sql))
                    .expect("failed to serialize JSON value for SQLite JSON column")
            },
        )
    });

    // Generate rusqlite-specific implementations
    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = super::rusqlite::generate_json_impls(&json_types)?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls: Vec<TokenStream> = vec![];

    // Generate turso-specific implementations
    #[cfg(feature = "turso")]
    let turso_json_impls = super::turso::generate_json_impls(&json_types)?;

    #[cfg(not(feature = "turso"))]
    let turso_json_impls: Vec<TokenStream> = vec![];

    // Generate libsql-specific implementations
    #[cfg(feature = "libsql")]
    let libsql_json_impls = super::libsql::generate_json_impls(&json_types)?;

    #[cfg(not(feature = "libsql"))]
    let libsql_json_impls: Vec<TokenStream> = vec![];

    let json_types_impl = quote! {
        #(#core_impls)*
        #(#to_sql_impl)*
        #(#rusqlite_impls)*
        #(#turso_json_impls)*
        #(#libsql_json_impls)*
    };

    Ok(json_types_impl)
}
