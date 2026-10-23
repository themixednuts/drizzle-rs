//! `#[cfg(...)]` / `#[cfg_attr(...)]` handling for the schema parser.
//!
//! The parser reads source before the compiler strips cfg-gated code, and it
//! cannot know which Cargo features a build enables. What it can know is how
//! a dialect's own entity is gated: an enum written
//!
//! ```ignore
//! #[cfg_attr(feature = "sqlite", derive(SQLiteEnum))]
//! #[cfg_attr(feature = "postgres", derive(PostgresEnum))]
//! enum Role { Member, Admin }
//! ```
//!
//! derives `PostgresEnum` exactly when `feature = "postgres"` holds. So the
//! parser reads each dialect's view of an item with its own predicate on and
//! the predicates gating the item's other dialects off, and leaves every
//! other predicate undecided. Attributes and fields gated by a predicate that
//! evaluates false are dropped; undecided ones are kept, as before.

use proc_macro2::{Delimiter, Group, Span, TokenStream};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{AttrStyle, Attribute, Meta, Token};

/// Three-valued truth of a cfg predicate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Truth {
    True,
    False,
    Unknown,
}

/// Predicates whose value is known for one dialect's view of an item, keyed
/// by their token text (`feature = "postgres"`).
#[derive(Default, Debug, Clone)]
pub(super) struct CfgContext {
    known: Vec<(String, bool)>,
}

impl CfgContext {
    /// The view of a dialect gated by `own` (`None`: not gated), on an item
    /// whose other dialects are gated by `others`.
    pub(super) fn for_dialect<'a>(
        own: Option<&str>,
        others: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut known = Vec::new();
        if let Some(own) = own {
            known.push((own.to_string(), true));
        }
        for other in others {
            if Some(other) != own && !known.iter().any(|(key, _)| key == other) {
                known.push((other.to_string(), false));
            }
        }
        Self { known }
    }

    pub(super) fn eval(&self, predicate: &Meta) -> Truth {
        let key = predicate_key(predicate);
        if let Some((_, value)) = self.known.iter().find(|(known, _)| *known == key) {
            return if *value { Truth::True } else { Truth::False };
        }
        let Meta::List(list) = predicate else {
            return Truth::Unknown;
        };
        let Ok(nested) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            return Truth::Unknown;
        };
        let values = nested.iter().map(|meta| self.eval(meta));
        if list.path.is_ident("not") && nested.len() == 1 {
            match self.eval(&nested[0]) {
                Truth::True => Truth::False,
                Truth::False => Truth::True,
                Truth::Unknown => Truth::Unknown,
            }
        } else if list.path.is_ident("all") {
            values.fold(Truth::True, |acc, value| match (acc, value) {
                (Truth::False, _) | (_, Truth::False) => Truth::False,
                (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
                _ => Truth::True,
            })
        } else if list.path.is_ident("any") {
            values.fold(Truth::False, |acc, value| match (acc, value) {
                (Truth::True, _) | (_, Truth::True) => Truth::True,
                (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
                _ => Truth::False,
            })
        } else {
            Truth::Unknown
        }
    }

    /// Whether code under these attributes may be compiled: false only when
    /// a `#[cfg(...)]` predicate is known to be false.
    pub(super) fn is_active(&self, attrs: &[Attribute]) -> bool {
        attrs.iter().all(|attr| {
            !attr.path().is_ident("cfg")
                || attr
                    .parse_args::<Meta>()
                    .map_or(true, |predicate| self.eval(&predicate) != Truth::False)
        })
    }

    /// `attrs` with every `#[cfg_attr(predicate, a, b)]` unrolled to `#[a]`
    /// and `#[b]` (recursively), or dropped when the predicate is false.
    pub(super) fn expand(&self, attrs: &[Attribute]) -> Vec<Attribute> {
        let mut out = Vec::with_capacity(attrs.len());
        for attr in attrs {
            let Some((predicate, metas)) = split_cfg_attr(attr) else {
                out.push(attr.clone());
                continue;
            };
            if self.eval(&predicate) == Truth::False {
                continue;
            }
            let unrolled: Vec<Attribute> = metas
                .into_iter()
                .map(|meta| synthetic_attribute(attr.style, meta))
                .collect();
            out.extend(self.expand(&unrolled));
        }
        out
    }
}

/// The predicate and attributes of a `#[cfg_attr(predicate, attrs...)]`.
pub(super) fn split_cfg_attr(attr: &Attribute) -> Option<(Meta, Vec<Meta>)> {
    if !attr.path().is_ident("cfg_attr") {
        return None;
    }
    let mut metas = attr
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .ok()?
        .into_iter();
    let predicate = metas.next()?;
    Some((predicate, metas.collect()))
}

/// Every attribute meta on an item, with the predicate key of the outermost
/// `cfg_attr` gating it (`None` when written plainly).
pub(super) fn gated_metas(attrs: &[Attribute]) -> Vec<(Meta, Option<String>)> {
    fn collect(meta: Meta, gate: Option<&str>, out: &mut Vec<(Meta, Option<String>)>) {
        if meta.path().is_ident("cfg_attr")
            && let Meta::List(list) = &meta
            && let Ok(nested) =
                list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        {
            let mut nested = nested.into_iter();
            let Some(predicate) = nested.next() else {
                return;
            };
            let key = predicate_key(&predicate);
            for inner in nested {
                collect(inner, Some(gate.unwrap_or(&key)), out);
            }
            return;
        }
        out.push((meta, gate.map(str::to_string)));
    }
    let mut out = Vec::new();
    for attr in attrs {
        collect(attr.meta.clone(), None, &mut out);
    }
    out
}

/// The token text that identifies a predicate.
pub(super) fn predicate_key(predicate: &Meta) -> String {
    predicate.to_token_stream().to_string()
}

/// `#[meta]` spanning only `meta`'s source, so its source text is the meta
/// as written inside the `cfg_attr`.
fn synthetic_attribute(style: AttrStyle, meta: Meta) -> Attribute {
    let span = syn::spanned::Spanned::span(&meta);
    Attribute {
        pound_token: Token![#](span),
        style,
        bracket_token: syn::token::Bracket(bracket_span(span)),
        meta,
    }
}

fn bracket_span(span: Span) -> proc_macro2::extra::DelimSpan {
    let mut group = Group::new(Delimiter::Bracket, TokenStream::new());
    group.set_span(span);
    group.delim_span()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(src: &str) -> Vec<Attribute> {
        let item: syn::ItemStruct = syn::parse_str(&format!("{src} struct S;")).unwrap();
        item.attrs
    }

    fn meta(src: &str) -> Meta {
        syn::parse_str(src).unwrap()
    }

    #[test]
    fn evaluates_known_predicates_and_combinators() {
        let cx =
            CfgContext::for_dialect(Some(r#"feature = "postgres""#), [r#"feature = "sqlite""#]);
        assert_eq!(cx.eval(&meta(r#"feature = "postgres""#)), Truth::True);
        assert_eq!(cx.eval(&meta(r#"feature = "sqlite""#)), Truth::False);
        assert_eq!(cx.eval(&meta(r#"not(feature = "sqlite")"#)), Truth::True);
        assert_eq!(cx.eval(&meta(r#"feature = "uuid""#)), Truth::Unknown);
        assert_eq!(
            cx.eval(&meta(r#"all(feature = "postgres", feature = "uuid")"#)),
            Truth::Unknown
        );
        assert_eq!(
            cx.eval(&meta(r#"all(feature = "sqlite", feature = "uuid")"#)),
            Truth::False
        );
        assert_eq!(
            cx.eval(&meta(r#"any(feature = "postgres", feature = "uuid")"#)),
            Truth::True
        );
    }

    #[test]
    fn expands_cfg_attr_and_drops_false_ones() {
        let cx =
            CfgContext::for_dialect(Some(r#"feature = "postgres""#), [r#"feature = "sqlite""#]);
        let expanded = cx.expand(&attrs(
            r#"#[cfg_attr(feature = "postgres", derive(PostgresEnum), postgres_enum(text))]
               #[cfg_attr(feature = "sqlite", derive(SQLiteEnum))]
               #[cfg_attr(feature = "uuid", cfg_attr(test, derive(Debug)))]
               #[repr(u8)]"#,
        ));
        let paths: Vec<String> = expanded
            .iter()
            .map(|a| a.path().to_token_stream().to_string())
            .collect();
        assert_eq!(paths, ["derive", "postgres_enum", "derive", "repr"]);
    }

    #[test]
    fn gated_metas_report_the_outer_predicate() {
        let gated = gated_metas(&attrs(
            r#"#[derive(Clone)]
               #[cfg_attr(feature = "pg", derive(PostgresEnum))]"#,
        ));
        assert_eq!(gated[0].1, None);
        assert_eq!(gated[1].1.as_deref(), Some(r#"feature = "pg""#));
    }
}
