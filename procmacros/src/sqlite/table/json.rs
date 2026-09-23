//! JSON fields of `SQLite` tables.
//!
//! A JSON field keeps its payload type in the generated models and converts
//! through `drizzle::core::Json<Payload>`, whose codecs drizzle-sqlite
//! implements once. The table macro therefore emits no JSON impls: it only
//! validates the feature set and emits the conversion expressions below.

use super::context::MacroContext;
use crate::sqlite::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

// Common SQLite documentation URLs for error messages and macro docs
const SQLITE_JSON_URL: &str = "https://sqlite.org/json1.html";

/// Rejects JSON fields when drizzle-macros was built without `serde`.
pub fn validate_json_fields(ctx: &MacroContext) -> Result<()> {
    if cfg!(feature = "serde") {
        return Ok(());
    }

    match ctx.field_infos.iter().find(|info| info.is_json_column()) {
        Some(field) => Err(syn::Error::new_spanned(
            field.ident,
            format!(
                "The 'serde' feature must be enabled to use JSON fields.\n\
             Add to Cargo.toml: drizzle = {{ version = \"*\", features = [\"serde\"] }}\n\
             See: {SQLITE_JSON_URL}"
            ),
        )),
        None => Ok(()),
    }
}

/// Reads a JSON field from a driver row at `idx` through `Json<Payload>`'s
/// `FromSQLiteValue` codec, the same path for every SQLite driver.
///
/// `row` must implement `DrizzleRowByIndex` and `idx` must be a `usize`
/// expression. The expression propagates decode errors with `?`.
pub fn row_decode(idx: &TokenStream, info: &FieldInfo, is_optional: bool) -> TokenStream {
    let base_type = info.base_type;
    if is_optional {
        quote! {{
            use drizzle::sqlite::traits::DrizzleRowByIndex;
            DrizzleRowByIndex::get_column::<
                ::std::option::Option<drizzle::core::Json<#base_type>>
            >(row, #idx)?
            .map(drizzle::core::Json::into_inner)
        }}
    } else {
        quote! {{
            use drizzle::sqlite::traits::DrizzleRowByIndex;
            DrizzleRowByIndex::get_column::<drizzle::core::Json<#base_type>>(row, #idx)?
                .into_inner()
        }}
    }
}

/// Reads a JSON field of a partial model, yielding `None` when the column is
/// absent, SQL `NULL`, or not a decodable document.
pub fn partial_row_decode(idx: &TokenStream, info: &FieldInfo) -> TokenStream {
    let base_type = info.base_type;
    quote! {{
        use drizzle::sqlite::traits::DrizzleRowByIndex;
        DrizzleRowByIndex::get_column::<
            ::std::option::Option<drizzle::core::Json<#base_type>>
        >(row, #idx)
        .ok()
        .flatten()
        .map(drizzle::core::Json::into_inner)
    }}
}

/// The Rust value type a JSON column binds and decodes through:
/// `Json<Payload>`, or `Option<Json<Payload>>` for a nullable column.
pub fn column_value_type(info: &FieldInfo) -> TokenStream {
    let base_type = info.base_type;
    if info.is_nullable {
        quote! { ::std::option::Option<drizzle::core::Json<#base_type>> }
    } else {
        quote! { drizzle::core::Json<#base_type> }
    }
}

/// Wraps a JSON column's `DEFAULT_FN`, which returns the payload (or
/// `Option<Payload>`), so it yields the column's `Json<Payload>` value type.
pub fn wrap_default_fn(func: &syn::Expr, is_nullable: bool) -> TokenStream {
    if is_nullable {
        quote! {
            ::std::option::Option::Some(|| {
                ::std::option::Option::map((#func)(), drizzle::core::Json)
            })
        }
    } else {
        quote! { ::std::option::Option::Some(|| drizzle::core::Json((#func)())) }
    }
}

/// Converts the payload expression `value` into the insert or update model
/// field that holds it. The field type selects the conversion.
pub fn model_value(value: &TokenStream) -> TokenStream {
    quote! {
        drizzle::core::json::JsonColumnValue::from_json(drizzle::core::Json(#value))
    }
}
