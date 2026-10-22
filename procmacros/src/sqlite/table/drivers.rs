//! Shared index-based field decoding for `SQLite` driver rows.
//!
//! Every driver decodes a select-model field through
//! `DrizzleRowByIndex::get_column`, which dispatches the cell's storage class
//! to the field type's `FromSQLiteValue` codec. The drivers' own typed
//! getters are not used: libsql's hit `unreachable!` on a storage-class
//! mismatch (NUMERIC affinity storing `10.0` as INTEGER), and turso's
//! `as_integer()`/`as_real()` accessors return `None` for any other class,
//! which a nullable field would read as NULL.

use crate::sqlite::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use super::errors;

/// Generate `name: value,` for a select-model field read at `idx`, a `usize`
/// expression. `Option<T>` maps only SQL NULL to `None`; a value that fails to
/// decode is an error.
pub fn generate_field_conversion_with_index(
    idx: &TokenStream,
    info: &FieldInfo,
    is_optional: bool,
) -> Result<TokenStream> {
    let name = info.ident;
    let base_type = info.base_type;

    // Check for unsupported reference types
    if matches!(base_type, syn::Type::Reference(_)) {
        return Err(syn::Error::new_spanned(
            name,
            errors::conversion::REFERENCE_TYPE_UNSUPPORTED,
        ));
    }

    // JSON documents decode through `Json<Payload>`'s codec.
    if info.is_json_column() {
        let decode = super::json::row_decode(idx, info, is_optional);
        return Ok(quote! { #name: #decode, });
    }

    let target = if is_optional {
        quote!(::std::option::Option<#base_type>)
    } else {
        quote!(#base_type)
    };
    Ok(quote! {
        #name: {
            use drizzle::sqlite::traits::DrizzleRowByIndex;
            DrizzleRowByIndex::get_column::<#target>(row, #idx)?
        },
    })
}
