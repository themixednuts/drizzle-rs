//! Shared `PhantomData` marker-type generation for Insert models.
//!
//! Both `SQLiteTable` and `PostgresTable` Insert models track which fields
//! have been set at compile time via a `PhantomData<(F0Set | F0NotSet, F1Set
//! | F1NotSet, ...)>` tuple. The three helpers below were duplicated
//! byte-for-byte across `procmacros/src/{sqlite,postgres}/table/models/
//! insert.rs`; they live here once so adding a new dialect doesn't require
//! a third copy.
//!
//! The output ZSTs are named `{TableIdent}{FieldPascal}{Set|NotSet}` —
//! caller-visible because the Insert model's generic parameter shows up in
//! `new()` return types and constructor-pattern matches.

use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

/// Generate the `(F0Set | F0NotSet, ...)` tuple literal for a known
/// required-fields pattern. `bools[i] == true` ⇒ slot `i` resolves to
/// `{table}{field}Set`; otherwise `{table}{field}NotSet`.
///
/// Used by `Insert::new()` to declare the post-construction phantom type:
/// the constructor accepts only required fields and returns a model whose
/// `T` says "those fields are set, the rest are not."
pub fn generate_pattern_literal(
    struct_ident: &Ident,
    field_idents: &[&Ident],
    required_fields_pattern: &[bool],
) -> TokenStream {
    let pattern_values: Vec<_> = required_fields_pattern
        .iter()
        .enumerate()
        .map(|(i, &is_required)| {
            let pascal = field_idents[i].to_string().to_upper_camel_case();
            if is_required {
                format_ident!("{}{}Set", struct_ident, pascal)
            } else {
                format_ident!("{}{}NotSet", struct_ident, pascal)
            }
        })
        .collect();
    quote! { (#(#pattern_values),*) }
}

/// Generate the all-`NotSet` tuple — `Insert::default()`'s phantom type.
///
/// Every Insert model starts life with every field unset; the per-field
/// `.field(value)` setters flip the corresponding slot from `NotSet` to
/// `Set` at the type level.
pub fn generate_empty_pattern_tuple(struct_ident: &Ident, field_idents: &[&Ident]) -> TokenStream {
    let elements: Vec<_> = field_idents
        .iter()
        .map(|ident| {
            let pascal = ident.to_string().to_upper_camel_case();
            format_ident!("{}{}NotSet", struct_ident, pascal)
        })
        .collect();
    quote! { (#(#elements),*) }
}

/// Emit the `{table}{field}Set` / `{table}{field}NotSet` ZSTs themselves —
/// one pair per field. These are the elements of the phantom tuples
/// produced by [`generate_pattern_literal`] and
/// [`generate_empty_pattern_tuple`].
pub fn generate_marker_types(struct_ident: &Ident, field_idents: &[&Ident]) -> Vec<TokenStream> {
    field_idents
        .iter()
        .map(|ident| {
            let pascal = ident.to_string().to_upper_camel_case();
            let set_marker = format_ident!("{}{}Set", struct_ident, pascal);
            let not_set_marker = format_ident!("{}{}NotSet", struct_ident, pascal);

            quote! {
                pub struct #set_marker;
                pub struct #not_set_marker;
            }
        })
        .collect()
}
