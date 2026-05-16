//! Helpers for resolving crate paths at macro expansion time.
//!
//! The drizzle macros generate code that references the `const_format` crate
//! for compile-time SQL string concatenation. Two contexts need this to work:
//!
//! 1. **End-user crates** that have `drizzle` (the umbrella) as a dependency.
//!    They don't usually declare `const_format` directly — it's a transitive
//!    dep, re-exported as `drizzle::const_format`. For these we emit
//!    `::drizzle::const_format::concatcp!(...)`.
//!
//! 2. **Inner crates** (`drizzle-postgres`, `drizzle-sqlite`) whose own doc
//!    tests exercise the macros. They can't depend on `drizzle` (cyclic), but
//!    they DO depend on `const_format` directly. For these we emit
//!    `::const_format::concatcp!(...)`.
//!
//! `proc-macro-crate` lets us look at the user's `Cargo.toml` and pick the
//! right path.
//!
//! For relative paths like `drizzle::core::traits::SQLTable`, no leading `::`
//! is used and they resolve naturally — either against the user's `drizzle`
//! extern crate, or against a `mod drizzle { ... }` shim in inner-crate doc
//! tests. Only the `::drizzle::const_format::` paths needed this dispatch.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::quote;

/// Path to the `const_format` crate, suitable for macro-generated code.
///
/// Resolution order:
/// 1. If the calling crate has `const_format` as a direct dep (e.g.
///    `drizzle-postgres`, `drizzle-sqlite`, or the `drizzle` umbrella itself
///    — they all do), emit `::<name>` and use it directly. This is the
///    cheapest path and works for every internal crate.
/// 2. If the calling crate has `drizzle` (umbrella) as a dep, fall back to
///    `::drizzle::const_format` (drizzle re-exports it). End-user crates
///    that just write `drizzle = "..."` in Cargo.toml take this path.
/// 3. Last resort: `::const_format`. Either it's there or it isn't — we
///    give up trying to be clever.
pub fn const_format() -> TokenStream {
    if let Ok(found) = crate_name("const_format") {
        return match found {
            FoundCrate::Itself => quote!(::const_format),
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
        };
    }
    if let Ok(FoundCrate::Name(name)) = crate_name("drizzle") {
        let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
        return quote!(::#ident::const_format);
    }
    quote!(::const_format)
}
