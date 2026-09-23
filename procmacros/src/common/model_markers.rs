//! Shared `PhantomData` marker-type generation for Insert models.
//!
//! Both `SQLiteTable` and `PostgresTable` Insert models track which fields
//! have been set at compile time via a `PhantomData<(F0Set | F0NotSet, F1Set
//! | F1NotSet, ...)>` tuple. The three helpers below were duplicated
//! byte-for-byte across `procmacros/src/{sqlite,postgres}/table/models/
//! insert.rs`; they live here once so adding a new dialect doesn't require
//! a third copy.
//!
//! The marker ZSTs are `{table}::{FieldPascal}{Set|NotSet}`, defined in the
//! table's column module (see [`super::column_types`]). They are
//! caller-visible because the Insert model's generic parameter shows up in
//! `new()` return types and constructor-pattern matches.

use super::column_types::{not_set_marker, set_marker};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate the `(F0Set | F0NotSet, ...)` tuple literal for a known
/// required-fields pattern. `bools[i] == true` ⇒ slot `i` resolves to
/// `{table}::{Field}Set`; otherwise `{table}::{Field}NotSet`.
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
            if is_required {
                set_marker(struct_ident, field_idents[i])
            } else {
                not_set_marker(struct_ident, field_idents[i])
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
        .map(|ident| not_set_marker(struct_ident, ident))
        .collect();
    quote! { (#(#elements),*) }
}
