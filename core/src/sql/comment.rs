//! sqlcommenter helpers — attach trace/context metadata to queries.
//!
//! Mirrors upstream `drizzle-orm`'s `sql.comment()` / `sqlCommenter()` helpers
//! so that observability sidecars that parse SQL comments (Google Cloud SQL
//! Insights, Sqlcommenter, etc.) see the same on-the-wire format.
//!
//! - [`comment`] wraps a free-form string in `/* ... */`, sanitising any
//!   embedded `/*` or `*/` sequences so they can't terminate the comment.
//! - [`comment_tags`] URL-encodes each key/value (matching JS
//!   `encodeURIComponent`, plus a `'` → `\'` escape), sorts alphabetically,
//!   joins with `,`, and wraps in `/* ... */`.
//!
//! Both helpers return an empty [`SQL`] fragment when the input reduces to
//! nothing (empty string, or a map whose values are all empty/skipped).

use crate::prelude::*;
use crate::sql::SQL;
use crate::traits::SQLParam;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

/// Byte set to percent-encode for `encodeURIComponent` semantics.
///
/// `encodeURIComponent` preserves `A-Z a-z 0-9 - _ . ! ~ * ' ( )` and
/// percent-encodes every other byte of the UTF-8 encoding. `percent-encoding`
/// works inversely — you enumerate bytes *to* encode — so we start from
/// `CONTROLS` (0x00-0x1F + 0x7F) and add every printable ASCII byte that is
/// *not* in the unreserved set.
const COMPONENT: &AsciiSet = &CONTROLS
    // space + ASCII printables that are not in the unreserved set.
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// Attach a free-form sqlcommenter comment to a query.
///
/// The input is sanitised so it cannot terminate the enclosing comment — any
/// `/*` becomes `/ *` and any `*/` becomes `* /`. An empty input yields an
/// empty SQL fragment (no wrapper).
///
/// In driver code you'd typically reach for this via
/// `QueryBuilder::comment(...)` on the per-dialect builder rather than calling
/// this helper directly.
pub fn comment<'a, V: SQLParam>(text: impl AsRef<str>) -> SQL<'a, V> {
    let text = text.as_ref();
    if text.is_empty() {
        return SQL::empty();
    }
    let sanitized = sanitize_string_input(text);
    let mut out = String::with_capacity(sanitized.len() + 4);
    out.push_str("/*");
    out.push_str(&sanitized);
    out.push_str("*/");
    SQL::raw(out)
}

/// Attach a tag-style sqlcommenter comment to a query.
///
/// Each pair is URL-encoded using `encodeURIComponent` semantics (with an
/// additional `'` → `\'` escape) and formatted as `key='value'`. Pairs are
/// sorted alphabetically by their encoded representation and joined with `,`.
/// Pairs whose value is empty after encoding are skipped. An empty result
/// yields an empty SQL fragment.
pub fn comment_tags<'a, V, I, K, Val>(pairs: I) -> SQL<'a, V>
where
    V: SQLParam,
    I: IntoIterator<Item = (K, Val)>,
    K: AsRef<str>,
    Val: AsRef<str>,
{
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in pairs {
        let v = v.as_ref();
        if v.is_empty() {
            continue;
        }
        let ek = sanitize_object_element(k.as_ref());
        let ev = sanitize_object_element(v);
        let mut entry = String::with_capacity(ek.len() + ev.len() + 3);
        entry.push_str(&ek);
        entry.push_str("='");
        entry.push_str(&ev);
        entry.push('\'');
        parts.push(entry);
    }
    if parts.is_empty() {
        return SQL::empty();
    }
    parts.sort();

    let total: usize = parts.iter().map(std::string::String::len).sum::<usize>() + parts.len() + 3;
    let mut out = String::with_capacity(total);
    out.push_str("/*");
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(p);
    }
    out.push_str("*/");
    SQL::raw(out)
}

/// Sanitise a free-form comment string so it can't terminate the enclosing
/// `/* ... */` block. Replaces `/*` with `/ *` and `*/` with `* /`, in that
/// order — matching upstream JS behaviour byte-for-byte.
#[inline]
fn sanitize_string_input(input: &str) -> String {
    input.replace("/*", "/ *").replace("*/", "* /")
}

/// Sanitise a key or value for a tag-style comment. URL-encodes using
/// `encodeURIComponent` semantics via [`percent_encoding::utf8_percent_encode`]
/// against [`COMPONENT`], then escapes any remaining `'` as `\'` (since `'` is
/// in the unreserved set that `encodeURIComponent` preserves but it would
/// clash with the surrounding `'...'` wrapping).
#[inline]
fn sanitize_object_element(s: &str) -> String {
    let encoded = utf8_percent_encode(s, COMPONENT).to_string();
    if encoded.as_bytes().contains(&b'\'') {
        encoded.replace('\'', "\\'")
    } else {
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{Dialect, SQLiteDialect};

    // A minimal SQLParam we can use in tests without pulling in a driver dep.
    #[derive(Clone, Debug)]
    struct TestValue;

    impl SQLParam for TestValue {
        const DIALECT: Dialect = Dialect::SQLite;
        type DialectMarker = SQLiteDialect;
    }

    fn render(s: SQL<'_, TestValue>) -> String {
        s.sql()
    }

    #[test]
    fn empty_string_yields_empty_sql() {
        let s: SQL<'_, TestValue> = comment("");
        assert_eq!(render(s), "");
    }

    #[test]
    fn plain_string_is_wrapped() {
        let s: SQL<'_, TestValue> = comment("hello world");
        assert_eq!(render(s), "/*hello world*/");
    }

    #[test]
    fn string_input_sanitises_comment_terminators() {
        let s: SQL<'_, TestValue> = comment("/* nested */ end");
        assert_eq!(render(s), "/*/ * nested * / end*/");
    }

    #[test]
    fn tags_are_sorted_and_url_encoded() {
        let s: SQL<'_, TestValue> = comment_tags([("route", "/users/:id"), ("action", "update")]);
        assert_eq!(render(s), "/*action='update',route='%2Fusers%2F%3Aid'*/");
    }

    #[test]
    fn empty_values_are_skipped() {
        let s: SQL<'_, TestValue> = comment_tags([("a", ""), ("b", "ok")]);
        assert_eq!(render(s), "/*b='ok'*/");
    }

    #[test]
    fn all_empty_yields_empty_sql() {
        let s: SQL<'_, TestValue> = comment_tags([("a", ""), ("b", "")]);
        assert_eq!(render(s), "");
    }

    #[test]
    fn quote_in_value_is_escaped_after_url_encoding() {
        let s: SQL<'_, TestValue> = comment_tags([("k", "it's")]);
        assert_eq!(render(s), r"/*k='it\'s'*/");
    }

    #[test]
    fn multibyte_utf8_is_percent_encoded_per_byte() {
        // "é" is U+00E9 = C3 A9 in UTF-8.
        let s: SQL<'_, TestValue> = comment_tags([("name", "café")]);
        assert_eq!(render(s), "/*name='caf%C3%A9'*/");
    }

    #[test]
    fn unreserved_set_is_preserved() {
        // ALPHA/DIGIT plus -_.!~*'()
        let s: SQL<'_, TestValue> = comment_tags([("k", "abcXYZ012-_.!~*()")]);
        // The single-quote is not in the input; this just confirms unreserved
        // bytes are left alone.
        assert_eq!(render(s), "/*k='abcXYZ012-_.!~*()'*/");
    }
}
