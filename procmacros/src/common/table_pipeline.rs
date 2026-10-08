//! Shared helpers for table macro pipelines.
//!
//! Setup steps every dialect's table macro runs the same way: the table name,
//! the struct's fields, primary-key counting and the insert model's
//! required-field pattern.

use heck::ToSnakeCase;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Result};

/// Resolve the SQL table name from the struct ident and optional name override.
pub fn table_name_from_attrs(struct_ident: &syn::Ident, name_override: Option<String>) -> String {
    name_override.unwrap_or_else(|| struct_ident.unraw().to_string().to_snake_case())
}

/// Extract struct fields for table macros, returning a helpful error for non-struct inputs.
pub fn struct_fields<'a>(input: &'a DeriveInput, macro_name: &str) -> Result<&'a Fields> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(_) => Ok(&data.fields),
            Fields::Unnamed(_) | Fields::Unit => Err(syn::Error::new(
                input.span(),
                format!("#[{macro_name}] requires a struct with named fields"),
            )),
        },
        _ => Err(syn::Error::new(
            input.span(),
            format!("#[{macro_name}] can only be applied to structs"),
        )),
    }
}

/// The struct's own attributes, to put back on the table handle: doc
/// comments, `cfg`, lint levels and other attribute macros.
///
/// The handle already derives `Default`, `Clone`, `Copy`, `Debug`,
/// `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord`, so a `derive` is an
/// error rather than a silent no-op, as are generics, which a table cannot
/// use.
pub fn forwarded_struct_attrs(
    input: &DeriveInput,
    macro_name: &str,
) -> Result<proc_macro2::TokenStream> {
    if let Some(param) = input.generics.params.first() {
        return Err(syn::Error::new(
            param.span(),
            format!("#[{macro_name}] tables cannot have generic parameters"),
        ));
    }
    if let Some(derive) = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("derive"))
    {
        return Err(syn::Error::new(
            derive.span(),
            format!(
                "#[{macro_name}] derives Default, Clone, Copy, Debug, PartialEq, Eq, Hash, \
                 PartialOrd and Ord on the table itself; remove this derive"
            ),
        ));
    }
    let attrs = &input.attrs;
    Ok(quote::quote! { #(#attrs)* })
}

/// Count primary keys using a caller-provided predicate.
pub fn count_primary_keys<F>(fields: &Fields, mut is_primary: F) -> Result<usize>
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
pub fn required_fields_pattern<T, F>(field_infos: &[T], mut is_optional: F) -> Vec<bool>
where
    F: FnMut(&T) -> bool,
{
    field_infos.iter().map(|info| !is_optional(info)).collect()
}
