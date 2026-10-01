//! `PostgreSQL`-only operators: arrays, JSON and JSONB, `ILIKE`, and POSIX
//! regular expressions.
//!
//! Each operator is a free function and, through an extension trait
//! ([`ArrayExprExt`], [`JsonExprExt`], [`RegexExprExt`]), a method on any
//! `PostgreSQL` expression. Operand types are checked at compile time, as
//! described below. Portable operators (`eq`, `like`, `and`, ...) live in
//! `drizzle_core::expr`.
//!
//! The examples use `raw_non_null::<PostgresValue, T>("col")` to stand for a
//! non-null column of SQL type `T`; in real code, use a table column.
//!
//! # Arrays
//!
//! | Operator | Function / method | True when |
//! |---|---|---|
//! | `@>` | [`array_contains`] / [`ArrayExprExt::array_contains`] | the left array holds every element of the right |
//! | `<@` | [`array_contained`] / [`ArrayExprExt::array_contained`] | every element of the left array is in the right |
//! | `&&` | [`array_overlaps`] / [`ArrayExprExt::array_overlaps`] | the arrays share at least one element |
//!
//! ## Operand types
//!
//! Both operands must be arrays, checked through [`ArrayOperand`]:
//!
//! - An `Array<T>` column (for example a `text[]` column) accepts an
//!   `Array<U>` operand when `T` is [`Compatible`](drizzle_types::Compatible) with `U`.
//! - Bind a Rust list with [`PgArray`]: `PgArray(vec!["a", "b"])` is an
//!   `Array<Text>`. A bare `Vec` or a single value is not an array operand.
//! - A placeholder or untyped SQL (`SQL::raw`) is accepted on either side.
//!
//! ## Examples
//!
//! ```
//! use drizzle_core::{ToSQL, expr::raw_non_null};
//! use drizzle_postgres::expr::{ArrayExprExt, PgArray};
//! use drizzle_postgres::values::PostgresValue;
//! use drizzle_types::{Array, postgres::types::Text};
//!
//! // Stands in for a `tags text[] NOT NULL` column.
//! let tags = raw_non_null::<PostgresValue, Array<Text>>("tags");
//! let condition = tags.array_contains(PgArray(vec!["rust", "sql"]));
//! assert_eq!(condition.to_sql().sql(), "tags @> $1");
//! ```
//!
//! # JSON and JSONB
//!
//! Access operators work on `json` and `jsonb` (operand bound: [`JsonType`]):
//!
//! | Operator | Function | Returns |
//! |---|---|---|
//! | `->` key | [`json_get`] | the field, as the input type (`json` or `jsonb`) |
//! | `->` index | [`json_get_idx`] | the array element, as the input type |
//! | `->>` key | [`json_get_text`] | the field as `text` |
//! | `->>` index | [`json_get_text_idx`] | the array element as `text` |
//! | `#>` path | [`json_get_path`] | the value at the path, as the input type |
//! | `#>>` path | [`json_get_path_text`] | the value at the path as `text` |
//!
//! Access results are nullable: a missing key, index or path yields NULL.
//!
//! Containment and key operators exist only for `jsonb` (operand bound: [`JsonbType`]):
//!
//! | Operator | Function | True when |
//! |---|---|---|
//! | `@>` | [`jsonb_contains`] | the left value contains the right value |
//! | `<@` | [`jsonb_contained`] | the left value is contained in the right value |
//! | `?` | [`jsonb_exists_key`] | the key is a top-level key |
//! | `?\|` | [`jsonb_exists_any`] | any of the keys is a top-level key |
//! | `?&` | [`jsonb_exists_all`] | all of the keys are top-level keys |
//!
//! [`JsonExprExt`] offers most of these as methods.
//! Untyped SQL (`SQL::raw`) is accepted wherever a JSON operand is expected.
//!
//! ## Examples
//!
//! ```
//! use drizzle_core::{ToSQL, expr::raw_non_null};
//! use drizzle_postgres::expr::JsonExprExt;
//! use drizzle_postgres::values::PostgresValue;
//! use drizzle_types::postgres::types::Jsonb;
//!
//! // Stands in for a `profile jsonb NOT NULL` column.
//! let profile = raw_non_null::<PostgresValue, Jsonb>("profile");
//! let city = profile.json_get("address").json_get_text("city");
//! assert_eq!(
//!     city.to_sql().sql(),
//!     "profile -> CAST ($1 AS TEXT) ->> CAST ($2 AS TEXT)"
//! );
//! ```
//!
//! # Pattern matching
//!
//! [`ilike`] and [`not_ilike`] match a `LIKE` pattern ignoring case. Both
//! sides must be textual ([`Textual`](drizzle_types::Textual)): the left a
//! `text`, `varchar`, `char` or enum expression, the pattern a compatible
//! textual value such as a `&str` or a placeholder.
//!
//! The regex functions, also available as [`RegexExprExt`] methods:
//!
//! | Operator | Function / method | True when the text |
//! |---|---|---|
//! | `~` | [`regex_match`] | matches the pattern (case-sensitive) |
//! | `~*` | [`regex_match_ci`] | matches the pattern (case-insensitive) |
//! | `!~` | [`regex_not_match`] | does not match the pattern (case-sensitive) |
//! | `!~*` | [`regex_not_match_ci`] | does not match the pattern (case-insensitive) |
//!
//! The left operand must be textual, as for `ILIKE`. The pattern is a `&str` bound as
//! a `text` parameter. The pattern matches anywhere in the string unless it is
//! anchored with `^` or `$`. Results are NULL when the left operand is NULL.
//!
//! ## Examples
//!
//! ```
//! use drizzle_core::{ToSQL, expr::raw_non_null};
//! use drizzle_postgres::expr::RegexExprExt;
//! use drizzle_postgres::values::PostgresValue;
//! use drizzle_types::postgres::types::Text;
//!
//! let sku = raw_non_null::<PostgresValue, Text>("sku");
//! let cond = sku.regex_match("^[A-Z]{3}-[0-9]+$");
//! assert_eq!(cond.to_sql().sql(), "sku ~ $1");
//! ```
//!
//! ## Type safety
//!
//! ```compile_fail
//! use drizzle_core::expr::raw_non_null;
//! use drizzle_postgres::expr::regex_match;
//! use drizzle_postgres::values::PostgresValue;
//! use drizzle_types::postgres::types::Int8;
//!
//! let id = raw_non_null::<PostgresValue, Int8>("id");
//! let _ = regex_match(id, "^1"); // `int8` is not textual
//! ```

mod array_ops;
mod ilike;
mod json_ops;
mod regex;

pub use array_ops::*;
pub use ilike::*;
pub use json_ops::*;
pub use regex::*;
