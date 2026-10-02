//! Small helpers shared by the dialect diffing and code generation modules.

use std::collections::HashMap;

// =============================================================================
// Hash Function
// =============================================================================

const DICTIONARY: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Returns a deterministic `len`-character alphanumeric hash of `input`
/// (drizzle-kit's algorithm), used in generated constraint names.
#[must_use]
pub fn hash(input: &str, len: usize) -> String {
    let dict_len = DICTIONARY.len() as u128;
    let len_u32 = u32::try_from(len).unwrap_or(u32::MAX);
    let combinations_count = dict_len.pow(len_u32);
    let p: u128 = 53;
    let mut power: u128 = 1;
    let mut hash_val: u128 = 0;

    for ch in input.chars() {
        let code = u128::from(u32::from(ch));
        hash_val = (hash_val + (code * power)) % combinations_count;
        power = (power * p) % combinations_count;
    }

    let mut result = Vec::with_capacity(len);
    let mut index = hash_val;

    for _ in 0..len {
        let idx = usize::try_from(index % dict_len).unwrap_or(0);
        result.push(DICTIONARY[idx] as char);
        index /= dict_len;
    }

    result.into_iter().rev().collect()
}

// =============================================================================
// String Utilities
// =============================================================================

/// Removes every leading and trailing `c`.
#[must_use]
pub fn trim_char(s: &str, c: char) -> String {
    s.trim_start_matches(c).trim_end_matches(c).to_string()
}

/// Removes leading and trailing occurrences of each char in `chars`, one
/// char at a time in order.
#[must_use]
pub fn trim_chars(s: &str, chars: &[char]) -> String {
    let mut result = s.to_string();
    for c in chars {
        result = result
            .trim_start_matches(*c)
            .trim_end_matches(*c)
            .to_string();
    }
    result
}

/// Escapes backslashes and single quotes (and, for
/// [`EscapeMode::PgArray`], double quotes) for use in a SQL default.
#[must_use]
pub fn escape_for_sql_default(input: &str, mode: EscapeMode) -> String {
    let mut value = input.replace('\\', "\\\\").replace('\'', "''");
    if matches!(mode, EscapeMode::PgArray) {
        value = value.replace('"', "\\\"");
    }
    value
}

/// Escapes backslashes and double quotes for a Rust `"..."` literal in
/// generated code.
#[must_use]
pub fn escape_for_rust_literal(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Rust keywords (strict, reserved, and the 2024 edition's `gen`), which a
/// generated identifier must not be.
const RUST_KEYWORDS: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Turns `name` into a Rust identifier for generated code: characters that
/// cannot appear in one become `_`, a leading digit gets a `_` prefix, and a
/// keyword gets a `_` suffix (`type` → `type_`).
pub(crate) fn rust_ident(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|ch| {
            if ch == '_' || ch.is_ascii_alphanumeric() {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|ch: char| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if RUST_KEYWORDS.contains(&out.as_str()) {
        out.push('_');
    }
    out
}

/// Whether the schema macros derive `sql_name` from the Rust identifier
/// `ident` on their own. Tables, columns and views default to the
/// identifier in `snake_case`; when that differs, generated code must spell
/// out `name = "..."`.
pub(crate) fn macro_default_name_matches(ident: &str, sql_name: &str) -> bool {
    use heck::ToSnakeCase;
    ident.to_snake_case() == sql_name
}

/// One output column of a view's `SELECT` list (see [`view_select_columns`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ViewSelectItem {
    /// A column reference, optionally qualified (`"t"."c"`), possibly
    /// renamed with `AS`.
    Column {
        output: String,
        table: Option<String>,
        column: String,
    },
    /// Any other expression with an `AS` alias.
    Expression { output: String, expression: String },
    /// `*` or `"t".*`.
    Star { table: Option<String> },
}

/// The output columns of a view definition and the first table after
/// `FROM`, read from the SQL text.
///
/// drizzle-kit snapshots do not record view columns, but the view macros
/// need fields. This reads the simple `select ... from ...` shape
/// drizzle-kit writes for query-builder views; anything it cannot follow
/// (no `FROM`, an item without a name, unbalanced quotes) gives `None`.
pub(crate) fn view_select_columns(
    definition: &str,
) -> Option<(Option<String>, Vec<ViewSelectItem>)> {
    let text = definition.trim().trim_end_matches(';');
    let lowered = text.to_ascii_lowercase();
    let mut rest = lowered.strip_prefix("select")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let mut start = text.len() - rest.len();
    rest = rest.trim_start();
    if let Some(after) = rest.strip_prefix("distinct")
        && after.starts_with(char::is_whitespace)
    {
        start = text.len() - after.len();
    }

    // `select 1 as one` has no FROM.
    let items_end = find_top_level_keyword(text, start, "from");
    let mut items = Vec::new();
    for item in split_top_level(&text[start..items_end.unwrap_or(text.len())], ',') {
        items.push(parse_select_item(item.trim())?);
    }
    if items.is_empty() {
        return None;
    }

    let from_table = match items_end {
        Some(end) => {
            let from = text[end + "from".len()..].trim_start();
            read_qualified_identifier(from).map(|(parts, _)| parts.last().cloned())?
        }
        None => None,
    };
    Some((from_table, items))
}

/// Index of `keyword` as a whole word outside quotes and parentheses.
fn find_top_level_keyword(text: &str, start: usize, keyword: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0_i32;
    let mut quote: Option<u8> = None;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
        } else {
            match b {
                b'\'' | b'"' | b'`' => quote = Some(b),
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ if depth == 0
                    && text[i..]
                        .get(..keyword.len())
                        .is_some_and(|word| word.eq_ignore_ascii_case(keyword))
                    && (i == 0 || !is_word_byte(bytes[i - 1]))
                    && bytes
                        .get(i + keyword.len())
                        .is_none_or(|next| !is_word_byte(*next)) =>
                {
                    return Some(i);
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

const fn is_word_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// Splits on `separator` outside quotes and parentheses.
fn split_top_level(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut quote: Option<char> = None;
    let mut last = 0;
    for (i, ch) in text.char_indices() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c == separator && depth == 0 => {
                parts.push(&text[last..i]);
                last = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[last..]);
    parts
}

/// Reads `ident` or `ident.ident...` (each part bare or quoted with `"` or
/// `` ` ``) from the start of `text`; returns the parts and the rest.
fn read_qualified_identifier(text: &str) -> Option<(Vec<String>, &str)> {
    let mut parts = Vec::new();
    let mut rest = text;
    loop {
        let (part, tail) = read_identifier(rest)?;
        parts.push(part);
        rest = tail;
        match rest.strip_prefix('.') {
            Some(tail) => rest = tail,
            None => return Some((parts, rest)),
        }
    }
}

fn read_identifier(text: &str) -> Option<(String, &str)> {
    let mut chars = text.char_indices();
    let (_, first) = chars.next()?;
    if first == '"' || first == '`' {
        let close = text[1..].find(first)? + 1;
        return Some((text[1..close].to_string(), &text[close + 1..]));
    }
    if !(first == '_' || first.is_alphabetic()) {
        return None;
    }
    let end = text
        .char_indices()
        .find(|(_, c)| !(*c == '_' || c.is_alphanumeric() || *c == '$'))
        .map_or(text.len(), |(i, _)| i);
    Some((text[..end].to_string(), &text[end..]))
}

fn parse_select_item(item: &str) -> Option<ViewSelectItem> {
    if item == "*" {
        return Some(ViewSelectItem::Star { table: None });
    }
    // `expr AS alias` (the alias is the last word of the item).
    if let Some(as_pos) = find_last_top_level_as(item) {
        let expression = item[..as_pos].trim();
        let (alias, tail) = read_identifier(item[as_pos + 2..].trim_start())?;
        if !tail.trim().is_empty() {
            return None;
        }
        return Some(match read_qualified_identifier(expression) {
            Some((mut parts, tail)) if tail.trim().is_empty() && parts.len() <= 2 => {
                let column = parts.pop()?;
                ViewSelectItem::Column {
                    output: alias,
                    table: parts.pop(),
                    column,
                }
            }
            _ => ViewSelectItem::Expression {
                output: alias,
                expression: expression.to_string(),
            },
        });
    }
    if let Some(table) = item.strip_suffix(".*") {
        let (parts, tail) = read_qualified_identifier(table)?;
        return tail.is_empty().then(|| ViewSelectItem::Star {
            table: parts.last().cloned(),
        });
    }
    let (mut parts, tail) = read_qualified_identifier(item)?;
    if !tail.trim().is_empty() || parts.len() > 2 {
        return None;
    }
    let column = parts.pop()?;
    Some(ViewSelectItem::Column {
        output: column.clone(),
        table: parts.pop(),
        column,
    })
}

/// Position of the last top-level ` as ` keyword in a select item.
fn find_last_top_level_as(item: &str) -> Option<usize> {
    let mut found = None;
    let mut from = 0;
    while let Some(pos) = find_top_level_keyword(item, from, "as") {
        found = Some(pos);
        from = pos + 2;
    }
    found
}

/// Convert a database-reported SQL default into the Rust-like expression
/// accepted by `#[column(DEFAULT = ...)]`.
///
/// SQL string literals become Rust string literals so the schema macro can
/// distinguish them from unquoted SQL keywords and function calls. PostgreSQL
/// casts are removed because the column type already supplies that context.
pub(crate) fn default_expression(sql: &str) -> Option<String> {
    default_expression_with(sql, false)
}

/// [`default_expression`] for a dialect whose string literals also use
/// backslash escapes (MySQL: `'back\\slash'` is the value `back\slash`).
pub(crate) fn default_expression_with(sql: &str, backslash_escapes: bool) -> Option<String> {
    let sql = sql.trim();
    let mut rust = String::with_capacity(sql.len());
    let mut rest = sql;

    while let Some(ch) = rest.chars().next() {
        if ch == '\'' {
            let (value, tail) = sql_string_literal(rest, backslash_escapes)?;
            rust.push_str(&format!("{value:?}"));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("::") {
            rest = skip_cast_type(tail);
        } else {
            rust.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }

    syn::parse_str::<syn::Expr>(&rust).ok().map(|_| rust)
}

/// Comment emitted in generated schema code when a database default cannot be
/// rendered as `#[column(default = ...)]`, so the output still compiles and the
/// omission is visible next to the field.
pub(crate) fn unsupported_default_comment(indent: &str, sql: &str) -> String {
    let sql = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "{indent}// TODO: default `{sql}` cannot be expressed as `#[column(default = ...)]`; add it manually.\n"
    )
}

/// Split a leading SQL string literal (`'it''s'`) into its unescaped value and
/// the remaining input. Returns `None` when the literal is unterminated.
fn sql_string_literal(input: &str, backslash_escapes: bool) -> Option<(String, &str)> {
    let mut value = String::new();
    let mut rest = input.strip_prefix('\'')?;
    loop {
        if backslash_escapes {
            let end = rest.find(['\'', '\\'])?;
            if rest[end..].starts_with('\\') {
                value.push_str(&rest[..end]);
                let mut escaped = rest[end + 1..].chars();
                let character = escaped.next()?;
                value.push(match character {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    other => other,
                });
                rest = escaped.as_str();
                continue;
            }
        }
        let end = rest.find('\'')?;
        value.push_str(&rest[..end]);
        rest = &rest[end + 1..];
        if let Some(tail) = rest.strip_prefix('\'') {
            value.push('\'');
            rest = tail;
        } else {
            return Some((value, rest));
        }
    }
}

/// Words that continue a multi-word PostgreSQL type name after its first word
/// (`character varying`, `double precision`, `timestamp without time zone`,
/// `interval year to month`).
const TYPE_NAME_CONTINUATIONS: &[&str] = &[
    "varying",
    "precision",
    "with",
    "without",
    "time",
    "zone",
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "to",
];

/// Skip the type name following a `::` cast, including schema qualification,
/// quoted identifiers, type modifiers (`numeric(10,2)`), array suffixes and
/// multi-word spellings such as `timestamp(3) without time zone`.
fn skip_cast_type(input: &str) -> &str {
    let mut rest = input.trim_start();

    // Possibly schema-qualified, possibly quoted first word.
    loop {
        rest = skip_identifier(rest);
        match rest.strip_prefix('.') {
            Some(tail) => rest = tail,
            None => break,
        }
    }
    rest = skip_type_suffixes(rest);

    loop {
        let candidate = rest.trim_start();
        let end = candidate
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(candidate.len());
        let word = &candidate[..end];
        let word_terminated = candidate[end..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        if word.is_empty()
            || !word_terminated
            || !TYPE_NAME_CONTINUATIONS.contains(&word.to_ascii_lowercase().as_str())
        {
            return rest;
        }
        rest = skip_type_suffixes(&candidate[end..]);
    }
}

fn skip_identifier(input: &str) -> &str {
    if let Some(mut rest) = input.strip_prefix('"') {
        loop {
            let Some(end) = rest.find('"') else {
                return "";
            };
            rest = &rest[end + 1..];
            match rest.strip_prefix('"') {
                Some(tail) => rest = tail,
                None => return rest,
            }
        }
    }
    let end = input
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(input.len());
    &input[end..]
}

/// Skip a type modifier list and any array dimension suffixes.
fn skip_type_suffixes(input: &str) -> &str {
    let mut rest = input;
    if rest.starts_with('(') {
        let mut depth = 0usize;
        let mut close = rest.len();
        for (index, ch) in rest.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = index + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        rest = &rest[close..];
    }
    while let Some(tail) = rest.strip_prefix('[') {
        let Some(end) = tail.find(']') else {
            return "";
        };
        rest = &tail[end + 1..];
    }
    rest
}

/// Reverses [`escape_for_sql_default`]. `''` is kept as-is in
/// [`EscapeMode::Array`].
#[must_use]
pub fn unescape_from_sql_default(input: &str, mode: EscapeMode) -> String {
    let mut res = input.replace("\\\"", "\"").replace("\\\\", "\\");
    if !matches!(mode, EscapeMode::Array) {
        res = res.replace("''", "'");
    }
    res
}

/// Context a SQL default value is escaped for.
#[derive(Debug, Clone, Copy)]
pub enum EscapeMode {
    /// A plain scalar default.
    Default,
    /// An element of an array literal.
    Array,
    /// An element of a PostgreSQL array literal (also escapes `"`).
    PgArray,
}

/// Returns `input` as a JSON/TypeScript string literal.
#[must_use]
pub fn escape_for_ts_literal(input: &str) -> String {
    // JSON.stringify equivalent
    serde_json::to_string(input).unwrap_or_else(|_| format!("\"{input}\""))
}

/// Renders a numeric default for TypeScript: a plain number when it fits in
/// `i64`, a `123n` bigint when larger, or a `` sql`...` `` template when it
/// is not a number.
#[must_use]
pub fn number_for_ts(value: &str) -> (NumberMode, String) {
    // `i64::MAX` is not exactly representable in f64 (it rounds up to 2^63),
    // so use literal bounds directly to avoid lossy `as f64` casts.
    const I64_MIN_F64: f64 = -9_223_372_036_854_775_808.0;
    const I64_MAX_F64: f64 = 9_223_372_036_854_775_807.0;

    value.parse::<f64>().map_or_else(
        |_| (NumberMode::Number, format!("sql`{value}`")),
        |num| {
            if num.is_nan() {
                (NumberMode::Number, format!("sql`{value}`"))
            } else if (I64_MIN_F64..=I64_MAX_F64).contains(&num) {
                (NumberMode::Number, value.to_string())
            } else {
                (NumberMode::BigInt, format!("{value}n"))
            }
        },
    )
}

/// TypeScript number kind chosen by [`number_for_ts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberMode {
    /// A plain `number`.
    Number,
    /// A `bigint` literal.
    BigInt,
}

/// Returns the parameters of a SQL type, e.g. `["10", "2"]` for
/// `numeric(10,2)`; empty when there are none.
#[must_use]
pub fn parse_params(type_str: &str) -> Vec<String> {
    if let Some(start) = type_str.find('(')
        && let Some(end) = type_str.find(')')
    {
        let params = &type_str[start + 1..end];
        return params.split(',').map(|s| s.trim().to_string()).collect();
    }
    Vec::new()
}

// =============================================================================
// Resolver Types
// =============================================================================

/// Created, deleted, and renamed items from a rename resolver.
#[derive(Debug, Clone)]
pub struct ResolverResult<T> {
    /// Items only in the new schema.
    pub created: Vec<T>,
    /// Items only in the old schema.
    pub deleted: Vec<T>,
    /// Items matched as renames or moves.
    pub renamed_or_moved: Vec<Rename<T>>,
}

/// One rename: `from` in the old schema became `to` in the new one.
#[derive(Debug, Clone)]
pub struct Rename<T> {
    /// Old item.
    pub from: T,
    /// New item.
    pub to: T,
}

/// A resolver that never detects renames: everything is a create or delete.
#[must_use]
pub const fn simple_resolver<T: Clone>(created: Vec<T>, deleted: Vec<T>) -> ResolverResult<T> {
    ResolverResult {
        created,
        deleted,
        renamed_or_moved: Vec::new(),
    }
}

// =============================================================================
// Inspect Utility
// =============================================================================

/// Formats a map as `{ k: 'v', ... }` for debug output (empty string when
/// empty).
#[must_use]
pub fn inspect<K, V, S>(map: &HashMap<K, V, S>) -> String
where
    K: std::fmt::Display,
    V: std::fmt::Display,
    S: std::hash::BuildHasher,
{
    if map.is_empty() {
        return String::new();
    }

    let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{k}: '{v}'")).collect();

    format!("{{ {} }}", pairs.join(", "))
}

// =============================================================================
// Migration Rename Tracking
// =============================================================================

/// Encodes renames as `table:from:to` and `column:table:from:to` strings
/// for a snapshot's `renames` list.
#[must_use]
pub fn prepare_migration_renames<T>(
    table_renames: &[(String, String)],
    column_renames: &[(String, String, String)], // (table, from, to)
) -> Vec<String> {
    let mut renames = Vec::new();

    for (from, to) in table_renames {
        renames.push(format!("table:{from}:{to}"));
    }

    for (table, from, to) in column_renames {
        renames.push(format!("column:{table}:{from}:{to}"));
    }

    renames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_ident_escapes_keywords_and_digits() {
        assert_eq!(rust_ident("type"), "type_");
        assert_eq!(rust_ident("user_id"), "user_id");
        assert_eq!(rust_ident("2fa"), "_2fa");
        assert_eq!(rust_ident("a-b c"), "a_b_c");
        assert!(macro_default_name_matches("created_at", "created_at"));
        assert!(!macro_default_name_matches("created_at", "createdAt"));
        // The macros snake_case `type_` back to `type`.
        assert!(macro_default_name_matches("type_", "type"));
    }

    #[test]
    fn view_select_columns_reads_drizzle_kit_views() {
        let (from, items) = view_select_columns(
            r#"select "author_id", count(*) as "total" from "posts" group by "posts"."author_id""#,
        )
        .expect("readable");
        assert_eq!(from.as_deref(), Some("posts"));
        assert_eq!(
            items,
            vec![
                ViewSelectItem::Column {
                    output: "author_id".into(),
                    table: None,
                    column: "author_id".into()
                },
                ViewSelectItem::Expression {
                    output: "total".into(),
                    expression: "count(*)".into()
                },
            ]
        );

        let (from, items) = view_select_columns(
            "SELECT DISTINCT `u`.`id`, `u`.`email` AS `mail`, coalesce(a, ',') AS c FROM `app`.`users` AS `u`",
        )
        .expect("readable");
        assert_eq!(from.as_deref(), Some("users"));
        assert_eq!(
            items[1],
            ViewSelectItem::Column {
                output: "mail".into(),
                table: Some("u".into()),
                column: "email".into()
            }
        );
        assert_eq!(
            items[2],
            ViewSelectItem::Expression {
                output: "c".into(),
                expression: "coalesce(a, ',')".into()
            }
        );

        assert_eq!(
            view_select_columns("select * from users").map(|(_, items)| items),
            Some(vec![ViewSelectItem::Star { table: None }])
        );
        assert_eq!(
            view_select_columns("select 1 as one"),
            Some((
                None,
                vec![ViewSelectItem::Expression {
                    output: "one".into(),
                    expression: "1".into()
                }]
            ))
        );
        // Unnamed expressions and non-SELECT definitions are not guessed.
        assert!(view_select_columns("select count(*) from users").is_none());
        assert!(view_select_columns("select 2").is_none());
        assert!(view_select_columns("values (1)").is_none());
    }

    #[test]
    fn test_hash() {
        let h1 = hash("test", 12);
        let h2 = hash("test", 12);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 12);

        let h3 = hash("different", 12);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_trim_char() {
        assert_eq!(trim_char("'hello'", '\''), "hello");
        assert_eq!(trim_char("hello", '\''), "hello");
    }

    #[test]
    fn test_parse_params() {
        assert_eq!(parse_params("varchar(255)"), vec!["255"]);
        assert_eq!(parse_params("numeric(10,2)"), vec!["10", "2"]);
        assert!(parse_params("text").is_empty());
    }

    #[test]
    fn test_number_for_ts() {
        let (mode, val) = number_for_ts("123");
        assert_eq!(mode, NumberMode::Number);
        assert_eq!(val, "123");

        // Use a value clearly outside f64's representation of i64 range
        // 1e20 is definitely greater than i64::MAX (~9.2e18)
        let (mode, val) = number_for_ts("100000000000000000000");
        assert_eq!(mode, NumberMode::BigInt);
        assert!(val.ends_with('n'));
    }

    #[test]
    fn test_escape_for_sql_default() {
        assert_eq!(
            escape_for_sql_default("it's a test", EscapeMode::Default),
            "it''s a test"
        );
        assert_eq!(
            escape_for_sql_default("path\\to\\file", EscapeMode::Default),
            "path\\\\to\\\\file"
        );
    }

    #[test]
    fn test_escape_for_rust_literal() {
        // Basic escaping
        assert_eq!(escape_for_rust_literal("hello"), "hello");

        // Escape double quotes
        assert_eq!(
            escape_for_rust_literal(r#"say "hello""#),
            r#"say \"hello\""#
        );

        // Escape backslashes
        assert_eq!(escape_for_rust_literal(r"path\to\file"), r"path\\to\\file");

        // Escape both
        assert_eq!(
            escape_for_rust_literal(r#"a "quoted" path\to\file"#),
            r#"a \"quoted\" path\\to\\file"#
        );

        // SQL query with quotes (typical view definition)
        assert_eq!(
            escape_for_rust_literal(r#"SELECT * FROM "users" WHERE name = 'test'"#),
            r#"SELECT * FROM \"users\" WHERE name = 'test'"#
        );
    }

    #[test]
    fn database_defaults_become_rust_expressions() {
        assert_eq!(
            default_expression("CURRENT_TIMESTAMP").as_deref(),
            Some("CURRENT_TIMESTAMP")
        );
        assert_eq!(default_expression("now()").as_deref(), Some("now()"));
        assert_eq!(
            default_expression("strftime('%s','now')").as_deref(),
            Some(r#"strftime("%s","now")"#)
        );
        assert_eq!(
            default_expression("nextval('users_id_seq'::regclass)").as_deref(),
            Some(r#"nextval("users_id_seq")"#)
        );
        assert_eq!(
            default_expression("'it''s'::text").as_deref(),
            Some(r#""it's""#)
        );
    }

    #[test]
    fn postgres_casts_are_stripped() {
        for (sql, expected) in [
            ("'guest'::character varying", r#""guest""#),
            ("'guest'::character varying(20)[]", r#""guest""#),
            ("'1.5'::double precision", r#""1.5""#),
            (
                "'2020-01-01 00:00:00'::timestamp without time zone",
                r#""2020-01-01 00:00:00""#,
            ),
            (
                "'2020-01-01 00:00:00+00'::timestamp(3) with time zone",
                r#""2020-01-01 00:00:00+00""#,
            ),
            ("'12:00:00'::time with time zone", r#""12:00:00""#),
            ("'1 year'::interval year to month", r#""1 year""#),
            ("'{}'::text[]", r#""{}""#),
            ("'{}'::jsonb", r#""{}""#),
            ("'active'::public.\"Status\"", r#""active""#),
            ("'1'::numeric(10,2) + 1", r#""1" + 1"#),
            ("('a'::text || 'b'::text)", r#"("a" || "b")"#),
            ("(now() + '1 day'::interval)", r#"(now() + "1 day")"#),
        ] {
            assert_eq!(default_expression(sql).as_deref(), Some(expected), "{sql}");
        }
    }

    #[test]
    fn untranslatable_defaults_are_rejected() {
        assert_eq!(default_expression("'unterminated"), None);
        assert_eq!(default_expression("x'00'"), None);
        assert_eq!(default_expression("ARRAY[]::text[]"), None);
        assert_eq!(default_expression("CAST(1 AS int)"), None);
    }
}
