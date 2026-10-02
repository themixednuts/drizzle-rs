//! Canonical `DEFAULT` clause text for `MySQL` columns.
//!
//! The table macros, the schema parser, catalog introspection and the
//! migration renderer all store a column default as the SQL text that follows
//! `DEFAULT`. They agree on one spelling so snapshots compare equal and every
//! rendered clause is accepted by the server:
//!
//! - `TEXT`, `BLOB`, `JSON` and spatial columns only accept expression
//!   defaults, so even a literal is written as `('hello')`;
//! - any other non-literal default (a function call or operator expression)
//!   is parenthesized, as in `(UUID())`;
//! - the `CURRENT_TIMESTAMP` family stays bare on `DATETIME`/`TIMESTAMP`
//!   columns, where `MySQL` accepts and reports it that way.

use super::MySQLTypeCategory;
use crate::alloc_prelude::{String, ToString, format};

/// Whether `MySQL` only accepts an expression `DEFAULT ( ... )` for a column
/// of `sql_type`. A literal default on such a column fails with error 1101.
#[must_use]
pub fn requires_expression_default(sql_type: &str) -> bool {
    match MySQLTypeCategory::classify(sql_type) {
        MySQLTypeCategory::TinyText
        | MySQLTypeCategory::Text
        | MySQLTypeCategory::MediumText
        | MySQLTypeCategory::LongText
        | MySQLTypeCategory::TinyBlob
        | MySQLTypeCategory::Blob
        | MySQLTypeCategory::MediumBlob
        | MySQLTypeCategory::LongBlob
        | MySQLTypeCategory::Json => true,
        MySQLTypeCategory::Custom => is_spatial_type(sql_type),
        _ => false,
    }
}

fn is_spatial_type(sql_type: &str) -> bool {
    let name = sql_type
        .trim()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .next()
        .unwrap_or_default();
    [
        "geometry",
        "point",
        "linestring",
        "polygon",
        "multipoint",
        "multilinestring",
        "multipolygon",
        "geometrycollection",
        "geomcollection",
    ]
    .iter()
    .any(|spatial| name.eq_ignore_ascii_case(spatial))
}

/// Whether `value` is `CURRENT_TIMESTAMP`, `NOW()`, `LOCALTIME` or
/// `LOCALTIMESTAMP`, with or without empty parentheses or a fractional
/// seconds precision.
#[must_use]
pub fn is_current_timestamp(value: &str) -> bool {
    let value = value.trim();
    let name_end = value
        .find(|ch: char| !ch.is_ascii_alphabetic() && ch != '_')
        .unwrap_or(value.len());
    let (name, rest) = value.split_at(name_end);
    let known = ["current_timestamp", "now", "localtime", "localtimestamp"]
        .iter()
        .any(|keyword| name.eq_ignore_ascii_case(keyword));
    if !known {
        return false;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return !name.eq_ignore_ascii_case("now");
    }
    rest.strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .is_some_and(|precision| precision.trim().bytes().all(|byte| byte.is_ascii_digit()))
}

/// Whether `value` is a single SQL literal: a quoted string, a (signed)
/// number, `NULL`, `TRUE`, `FALSE`, or a bit/hex literal.
#[must_use]
pub fn is_literal_default(value: &str) -> bool {
    let value = value.trim();
    if ["null", "true", "false"]
        .iter()
        .any(|keyword| value.eq_ignore_ascii_case(keyword))
    {
        return true;
    }
    if is_number(value) {
        return true;
    }
    if let Some(digits) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    if let Some(digits) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        return !digits.is_empty() && digits.bytes().all(|byte| matches!(byte, b'0' | b'1'));
    }
    let quoted = match value.as_bytes().first() {
        Some(b'x' | b'X' | b'b' | b'B' | b'n' | b'N') => &value[1..],
        _ => value,
    };
    quoted_string_end(quoted) == Some(quoted.len())
}

fn is_number(value: &str) -> bool {
    let unsigned = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(position) => (&unsigned[..position], Some(&unsigned[position + 1..])),
        None => (unsigned, None),
    };
    let mut parts = mantissa.splitn(2, '.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !digits(whole) || !digits(fraction) {
        return false;
    }
    exponent.is_none_or(|exponent| {
        let exponent = exponent
            .strip_prefix('-')
            .or_else(|| exponent.strip_prefix('+'))
            .unwrap_or(exponent);
        !exponent.is_empty() && digits(exponent)
    })
}

/// Returns the byte length of the quoted string literal starting `value`
/// (`'...'` with `''` and backslash escapes), or `None` if `value` does not
/// start with a complete one.
fn quoted_string_end(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.first() != Some(&b'\'') {
        return None;
    }
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'\'' if bytes.get(index + 1) == Some(&b'\'') => index += 2,
            b'\'' => return Some(index + 1),
            _ => index += 1,
        }
    }
    None
}

/// Whether the whole of `value` is enclosed by one matching pair of
/// parentheses, ignoring parentheses inside string literals and quoted
/// identifiers.
#[must_use]
pub fn is_parenthesized(value: &str) -> bool {
    let value = value.trim();
    if !value.starts_with('(') || !value.ends_with(')') {
        return false;
    }
    let bytes = value.as_bytes();
    let mut depth = 0usize;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            quote @ (b'\'' | b'"' | b'`') => {
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'\\' && quote != b'`' {
                        index += 2;
                        continue;
                    }
                    if bytes[index] == quote {
                        if bytes.get(index + 1) == Some(&quote) {
                            index += 2;
                            continue;
                        }
                        break;
                    }
                    index += 1;
                }
            }
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && index + 1 < bytes.len() {
                    return false;
                }
            }
            _ => {}
        }
        index += 1;
    }
    depth == 0
}

/// Returns the canonical `DEFAULT` text for a column of `sql_type`.
///
/// Literals stay bare unless the column type requires an expression
/// default; the `CURRENT_TIMESTAMP` family stays bare on `DATETIME` and
/// `TIMESTAMP` columns; everything else is wrapped in one pair of
/// parentheses (an already parenthesized expression is kept as is).
///
/// # Examples
///
/// ```
/// use drizzle_types::mysql::canonical_default;
///
/// assert_eq!(canonical_default("varchar(20)", "'hello'"), "'hello'");
/// assert_eq!(canonical_default("text", "'hello'"), "('hello')");
/// assert_eq!(canonical_default("varchar(36)", "UUID()"), "(UUID())");
/// assert_eq!(canonical_default("timestamp", "CURRENT_TIMESTAMP"), "CURRENT_TIMESTAMP");
/// assert_eq!(canonical_default("int", "-1"), "-1");
/// ```
#[must_use]
pub fn canonical_default(sql_type: &str, value: &str) -> String {
    let value = value.trim();
    if is_parenthesized(value) || value.eq_ignore_ascii_case("null") {
        return value.to_string();
    }
    let temporal = matches!(
        MySQLTypeCategory::classify(sql_type),
        MySQLTypeCategory::DateTime | MySQLTypeCategory::Timestamp
    );
    if temporal && is_current_timestamp(value) {
        return value.to_string();
    }
    if is_literal_default(value) && !requires_expression_default(sql_type) {
        return value.to_string();
    }
    format!("({value})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_literals_only_where_mysql_requires_expressions() {
        assert_eq!(canonical_default("TEXT", "'hello'"), "('hello')");
        assert_eq!(canonical_default("json", "'[]'"), "('[]')");
        assert_eq!(canonical_default("longblob", "X'AB'"), "(X'AB')");
        assert_eq!(canonical_default("POINT", "'x'"), "('x')");
        assert_eq!(canonical_default("varchar(10)", "'it''s'"), "'it''s'");
        assert_eq!(canonical_default("text", "NULL"), "NULL");
        assert_eq!(canonical_default("text", "('hello')"), "('hello')");
    }

    #[test]
    fn wraps_function_defaults_except_current_timestamp() {
        assert_eq!(canonical_default("varchar(36)", "UUID()"), "(UUID())");
        assert_eq!(canonical_default("json", "json_array()"), "(json_array())");
        assert_eq!(canonical_default("int", "1 + 2"), "(1 + 2)");
        assert_eq!(canonical_default("int", "(1) + (2)"), "((1) + (2))");
        assert_eq!(
            canonical_default("datetime(3)", "CURRENT_TIMESTAMP(3)"),
            "CURRENT_TIMESTAMP(3)"
        );
        assert_eq!(canonical_default("timestamp", "now()"), "now()");
        assert_eq!(canonical_default("date", "CURRENT_DATE"), "(CURRENT_DATE)");
        assert_eq!(
            canonical_default("varchar(30)", "CURRENT_TIMESTAMP"),
            "(CURRENT_TIMESTAMP)"
        );
    }

    #[test]
    fn keeps_scalar_literals_bare() {
        for literal in ["1", "-1", "1.5", "1e3", "TRUE", "false", "b'101'", "X'6869'", "0xab"] {
            assert_eq!(canonical_default("int", literal), literal);
        }
        assert!(!is_literal_default("'a' 'b'"));
        assert!(!is_literal_default("concat('a', 'b')"));
        assert!(!is_parenthesized("(a) + (b)"));
        assert!(is_parenthesized("(')(')"));
    }
}
