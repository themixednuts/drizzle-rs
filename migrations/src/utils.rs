//! Common utilities for schema diffing and migration generation
//!
//! This module provides shared utilities used across SQLite and PostgreSQL dialects.

use std::collections::HashMap;

// =============================================================================
// Hash Function
// =============================================================================

const DICTIONARY: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Generate a hash string from input, used for naming constraints
pub fn hash(input: &str, len: usize) -> String {
    let dict_len = DICTIONARY.len() as u128;
    let combinations_count = dict_len.pow(len as u32);
    let p: u128 = 53;
    let mut power: u128 = 1;
    let mut hash_val: u128 = 0;

    for ch in input.chars() {
        let code = ch as u128;
        hash_val = (hash_val + (code * power)) % combinations_count;
        power = (power * p) % combinations_count;
    }

    let mut result = Vec::with_capacity(len);
    let mut index = hash_val;

    for _ in 0..len {
        let idx = (index % dict_len) as usize;
        result.push(DICTIONARY[idx] as char);
        index /= dict_len;
    }

    result.into_iter().rev().collect()
}

// =============================================================================
// String Utilities
// =============================================================================

/// Trim a specific character from both ends of a string
pub fn trim_char(s: &str, c: char) -> String {
    s.trim_start_matches(c).trim_end_matches(c).to_string()
}

/// Trim multiple characters from both ends of a string
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

/// Escape a string for SQL default value
pub fn escape_for_sql_default(input: &str, mode: EscapeMode) -> String {
    let mut value = input.replace('\\', "\\\\").replace('\'', "''");
    if matches!(mode, EscapeMode::PgArray) {
        value = value.replace('"', "\\\"");
    }
    value
}

/// Unescape a string from SQL default value
pub fn unescape_from_sql_default(input: &str, mode: EscapeMode) -> String {
    let mut res = input.replace("\\\"", "\"").replace("\\\\", "\\");
    if !matches!(mode, EscapeMode::Array) {
        res = res.replace("''", "'");
    }
    res
}

/// Escape mode for SQL values
#[derive(Debug, Clone, Copy)]
pub enum EscapeMode {
    Default,
    Array,
    PgArray,
}

/// Escape a string for TypeScript literal
pub fn escape_for_ts_literal(input: &str) -> String {
    // JSON.stringify equivalent
    serde_json::to_string(input).unwrap_or_else(|_| format!("\"{}\"", input))
}

/// Parse number for TypeScript representation
pub fn number_for_ts(value: &str) -> (NumberMode, String) {
    match value.parse::<f64>() {
        Ok(num) => {
            if num.is_nan() {
                (NumberMode::Number, format!("sql`{}`", value))
            } else if num >= i64::MIN as f64 && num <= i64::MAX as f64 {
                (NumberMode::Number, value.to_string())
            } else {
                (NumberMode::BigInt, format!("{}n", value))
            }
        }
        Err(_) => (NumberMode::Number, format!("sql`{}`", value)),
    }
}

/// Number mode for TypeScript
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberMode {
    Number,
    BigInt,
}

/// Parse type parameters from SQL type like "varchar(255)" or "numeric(10,2)"
pub fn parse_params(type_str: &str) -> Vec<String> {
    if let Some(start) = type_str.find('(') {
        if let Some(end) = type_str.find(')') {
            let params = &type_str[start + 1..end];
            return params.split(',').map(|s| s.trim().to_string()).collect();
        }
    }
    Vec::new()
}

// =============================================================================
// Diff Grouping
// =============================================================================

/// Diff type for entity changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    Create,
    Drop,
    Alter,
}

/// A grouped row of diffs for the same table
#[derive(Debug, Clone, Default)]
pub struct GroupedRow<T> {
    pub schema: Option<String>,
    pub table: String,
    pub inserted: Vec<T>,
    pub deleted: Vec<T>,
    pub updated: Vec<T>,
}

/// Trait for items that can be grouped by table
pub trait Groupable {
    fn diff_type(&self) -> DiffType;
    fn schema(&self) -> Option<&str>;
    fn table(&self) -> Option<&str>;
}

/// Group diff items by table
pub fn group_diffs<T: Groupable + Clone>(arr: &[T]) -> Vec<GroupedRow<T>> {
    if arr.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<GroupedRow<T>> = Vec::new();

    for item in arr {
        let table = match item.table() {
            Some(t) => t.to_string(),
            None => continue,
        };
        let schema = item.schema().map(|s| s.to_string());

        // Find existing group
        let idx = result
            .iter()
            .position(|g| g.table == table && g.schema == schema);

        let group = if let Some(idx) = idx {
            &mut result[idx]
        } else {
            result.push(GroupedRow {
                schema: schema.clone(),
                table: table.clone(),
                inserted: Vec::new(),
                deleted: Vec::new(),
                updated: Vec::new(),
            });
            result.last_mut().unwrap()
        };

        match item.diff_type() {
            DiffType::Create => group.inserted.push(item.clone()),
            DiffType::Drop => group.deleted.push(item.clone()),
            DiffType::Alter => group.updated.push(item.clone()),
        }
    }

    result
}

// =============================================================================
// Resolver Types
// =============================================================================

/// Result from a rename resolver
#[derive(Debug, Clone)]
pub struct ResolverResult<T> {
    pub created: Vec<T>,
    pub deleted: Vec<T>,
    pub renamed_or_moved: Vec<Rename<T>>,
}

/// A rename operation
#[derive(Debug, Clone)]
pub struct Rename<T> {
    pub from: T,
    pub to: T,
}

/// Simple resolver that doesn't detect renames (everything is create/delete)
pub fn simple_resolver<T: Clone>(created: Vec<T>, deleted: Vec<T>) -> ResolverResult<T> {
    ResolverResult {
        created,
        deleted,
        renamed_or_moved: Vec::new(),
    }
}

// =============================================================================
// Inspect Utility
// =============================================================================

/// Inspect an object for debugging (simplified version)
pub fn inspect<K, V>(map: &HashMap<K, V>) -> String
where
    K: std::fmt::Display,
    V: std::fmt::Display,
{
    if map.is_empty() {
        return String::new();
    }

    let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{}: '{}'", k, v)).collect();

    format!("{{ {} }}", pairs.join(", "))
}

// =============================================================================
// Migration Rename Tracking
// =============================================================================

/// Prepare migration rename strings for storage
pub fn prepare_migration_renames<T>(
    table_renames: &[(String, String)],
    column_renames: &[(String, String, String)], // (table, from, to)
) -> Vec<String> {
    let mut renames = Vec::new();

    for (from, to) in table_renames {
        renames.push(format!("table:{}:{}", from, to));
    }

    for (table, from, to) in column_renames {
        renames.push(format!("column:{}:{}:{}", table, from, to));
    }

    renames
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

