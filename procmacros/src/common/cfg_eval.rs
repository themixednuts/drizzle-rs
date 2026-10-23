//! Field `#[cfg]` / `#[cfg_attr]` for the table and view attribute macros.
//!
//! An attribute macro receives its struct before the compiler evaluates the
//! `cfg`s on the fields: a field behind a false `#[cfg]` is still there, and
//! `#[cfg_attr(feature = "x", column(primary))]` is not a `column`
//! attribute. A derive, though, receives its input with every `cfg`
//! evaluated. So a table or view macro whose fields carry a `cfg` hands the
//! struct to the hidden `__DrizzleCfgEval` derive on a carrier struct, and
//! the derive runs the macro on the evaluated fields.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, Meta, Result, Token};

/// Field attributes the table and view macros read; the `__DrizzleCfgEval`
/// derive registers each as a helper so the carrier can hold them. Besides
/// `column`, these are the SQLite column-type shorthands (`#[text]`, ...).
#[allow(dead_code)]
pub const FIELD_HELPERS: &[&str] = &[
    "column", "integer", "text", "blob", "real", "number", "numeric", "boolean", "any", "INTEGER",
    "TEXT", "BLOB", "REAL", "NUMBER", "NUMERIC", "BOOLEAN", "ANY",
];

/// Whether a field carries a `cfg` or `cfg_attr` the macro would receive
/// unevaluated. Generic structs are left to the macro, which rejects them.
pub fn has_field_cfg(input: &DeriveInput) -> bool {
    let Data::Struct(data) = &input.data else {
        return false;
    };
    input.generics.params.is_empty()
        && data.fields.iter().any(|field| {
            field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
        })
}

/// Re-emit `input` as a carrier struct deriving `__DrizzleCfgEval`, which
/// runs `macro_name` with `args` on the struct once its `cfg`s are
/// evaluated. The struct's own attributes travel with the arguments; the
/// carrier fields keep only `cfg`, `doc` and the attributes the macros read.
pub fn defer(macro_name: &str, args: TokenStream, input: &DeriveInput) -> TokenStream {
    let macro_name = format_ident!("{macro_name}");
    let ident = &input.ident;
    let carrier = format_ident!("__DrizzleCfgEval{ident}");
    let attrs = &input.attrs;
    let vis = &input.vis;
    let Data::Struct(data) = &input.data else {
        unreachable!("has_field_cfg only accepts structs");
    };
    let mut fields = data.fields.clone();
    for field in &mut fields {
        field.attrs = field.attrs.iter().filter_map(carrier_attr).collect();
    }
    let body = match &fields {
        Fields::Named(named) => quote!(#named),
        Fields::Unnamed(unnamed) => quote!(#unnamed;),
        Fields::Unit => quote!(;),
    };
    quote! {
        #[derive(drizzle::__DrizzleCfgEval)]
        #[drizzle_cfg_eval(#macro_name, #ident, (#args), [#(#attrs)*])]
        #[allow(dead_code, non_camel_case_types)]
        #vis struct #carrier #body
    }
}

/// The macro name, its arguments and the original struct, from a carrier
/// the derive received with its `cfg`s evaluated.
pub fn undefer(mut input: DeriveInput) -> Result<(Ident, TokenStream, DeriveInput)> {
    let helper = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("drizzle_cfg_eval"))
        .ok_or_else(|| {
            syn::Error::new_spanned(&input.ident, "__DrizzleCfgEval is internal to drizzle")
        })?;
    let (macro_name, ident, args, attrs) = helper.parse_args_with(|stream: ParseStream| {
        let macro_name: Ident = stream.parse()?;
        stream.parse::<Token![,]>()?;
        let ident: Ident = stream.parse()?;
        stream.parse::<Token![,]>()?;
        let args;
        syn::parenthesized!(args in stream);
        let args: TokenStream = args.parse()?;
        stream.parse::<Token![,]>()?;
        let attrs;
        syn::bracketed!(attrs in stream);
        let attrs = Attribute::parse_outer(&attrs)?;
        Ok((macro_name, ident, args, attrs))
    })?;
    input.ident = ident;
    input.attrs = attrs;
    Ok((macro_name, args, input))
}

/// A field attribute as the carrier keeps it: `cfg`, `doc` and the macros'
/// own attributes as they are, a `cfg_attr` cut down to those, anything else
/// dropped (the macros ignore it, and the carrier could not hold it).
fn carrier_attr(attr: &Attribute) -> Option<Attribute> {
    let path = attr.path();
    if path.is_ident("cfg") || path.is_ident("doc") || is_helper(path) {
        return Some(attr.clone());
    }
    if !path.is_ident("cfg_attr") {
        return None;
    }
    let meta = carrier_cfg_attr(&attr.meta)?;
    Some(Attribute {
        meta,
        ..attr.clone()
    })
}

fn carrier_cfg_attr(meta: &Meta) -> Option<Meta> {
    let Meta::List(list) = meta else {
        return None;
    };
    let nested = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?;
    let mut nested = nested.into_iter();
    let predicate = nested.next()?;
    let kept: Vec<Meta> = nested
        .filter_map(|inner| {
            let path = inner.path();
            if path.is_ident("doc") || is_helper(path) {
                Some(inner)
            } else if path.is_ident("cfg_attr") {
                carrier_cfg_attr(&inner)
            } else {
                None
            }
        })
        .collect();
    if kept.is_empty() {
        return None;
    }
    let path = &list.path;
    Some(syn::parse_quote!(#path(#predicate, #(#kept),*)))
}

fn is_helper(path: &syn::Path) -> bool {
    path.get_ident()
        .is_some_and(|ident| FIELD_HELPERS.iter().any(|helper| ident == helper))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_keeps_only_what_the_macros_read() {
        let input: DeriveInput = syn::parse_quote! {
            /// Users.
            pub struct Users {
                #[cfg_attr(feature = "pg", column(primary), serde(rename = "key"))]
                pub id: i64,
                #[cfg(feature = "pg")]
                #[serde(skip)]
                /// Notes.
                pub note: String,
            }
        };
        assert!(has_field_cfg(&input));
        let carrier: DeriveInput =
            syn::parse2(defer("PostgresTable", quote!(RLS), &input)).unwrap();
        let Data::Struct(data) = &carrier.data else {
            panic!()
        };
        let attrs: Vec<String> = data
            .fields
            .iter()
            .flat_map(|f| f.attrs.iter().map(|a| quote!(#a).to_string()))
            .collect();
        assert_eq!(
            attrs,
            [
                r#"# [cfg_attr (feature = "pg" , column (primary))]"#,
                r#"# [cfg (feature = "pg")]"#,
                "# [doc = r\" Notes.\"]",
            ]
        );

        let (macro_name, args, original) = undefer(carrier).unwrap();
        assert_eq!(macro_name, "PostgresTable");
        assert_eq!(args.to_string(), "RLS");
        assert_eq!(original.ident, "Users");
        assert_eq!(original.attrs.len(), 1);
    }
}
