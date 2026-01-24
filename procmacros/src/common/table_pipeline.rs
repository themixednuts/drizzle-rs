//! Shared helpers for table macro pipelines.
//!
//! This module centralizes common setup steps used by SQLite and PostgreSQL
//! table macros to reduce duplication and keep behavior consistent.

use heck::ToSnakeCase;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Result};

/// Resolve the SQL table name from the struct ident and optional name override.
pub(crate) fn table_name_from_attrs(
    struct_ident: &syn::Ident,
    name_override: Option<String>,
) -> String {
    name_override.unwrap_or_else(|| struct_ident.to_string().to_snake_case())
}

/// Extract struct fields for table macros, returning a helpful error for non-struct inputs.
pub(crate) fn struct_fields<'a>(input: &'a DeriveInput, macro_name: &str) -> Result<&'a Fields> {
    match &input.data {
        Data::Struct(data) => Ok(&data.fields),
        _ => Err(syn::Error::new(
            input.span(),
            format!(
                "The #[{}] attribute can only be applied to struct definitions.\n",
                macro_name
            ),
        )),
    }
}

/// Count primary keys using a caller-provided predicate.
pub(crate) fn count_primary_keys<F>(fields: &Fields, mut is_primary: F) -> Result<usize>
where
    F: FnMut(&syn::Field) -> Result<bool>,
{
    let mut count = 0;
    for field in fields {
        if is_primary(field)? {
            count += 1;
        }
    }
    Ok(count)
}

/// Build a required-fields pattern for insert model const generics.
pub(crate) fn required_fields_pattern<T, F>(field_infos: &[T], mut is_optional: F) -> Vec<bool>
where
    F: FnMut(&T) -> bool,
{
    field_infos.iter().map(|info| !is_optional(info)).collect()
}
