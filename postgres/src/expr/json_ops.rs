//! PostgreSQL JSON/JSONB operators.
//!
//! Provides type-safe access to PostgreSQL JSON operators:
//! - `->` (get JSON object field by key, returns JSON)
//! - `->>` (get JSON object field by key, returns text)
//! - `#>` (get JSON object at path, returns JSON)
//! - `#>>` (get JSON object at path, returns text)
//! - `@>` (JSON contains)
//! - `?` (JSON key exists)

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::PostgresValue;
use drizzle_core::ToSQL;
use drizzle_core::expr::{Expr, NonNull, Null, SQLExpr, Scalar};
use drizzle_core::sql::{SQL, SQLChunk};
use drizzle_types::postgres::types::{Boolean, Json, Text};

/// PostgreSQL `->` operator - get JSON object field by key, returns JSON.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::json_get;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let field = json_get(data, "name");
/// assert!(field.to_sql().sql().contains("->"));
/// ```
pub fn json_get<'a, E>(expr: E, key: &'a str) -> SQLExpr<'a, PostgresValue<'a>, Json, Null, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->".into()))
            .append(SQL::param(PostgresValue::Text(key.into()))),
    )
}

/// PostgreSQL `->` operator with integer index - get JSON array element.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::json_get_idx;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let elem = json_get_idx(data, 0);
/// assert!(elem.to_sql().sql().contains("->"));
/// ```
pub fn json_get_idx<'a, E>(
    expr: E,
    index: i32,
) -> SQLExpr<'a, PostgresValue<'a>, Json, Null, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->".into()))
            .append(SQL::param(PostgresValue::Integer(index))),
    )
}

/// PostgreSQL `->>` operator - get JSON object field as text.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::json_get_text;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let name = json_get_text(data, "name");
/// assert!(name.to_sql().sql().contains("->>"));
/// ```
pub fn json_get_text<'a, E>(
    expr: E,
    key: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->>".into()))
            .append(SQL::param(PostgresValue::Text(key.into()))),
    )
}

/// PostgreSQL `->>` operator with integer index - get JSON array element as text.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::json_get_text_idx;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let elem = json_get_text_idx(data, 0);
/// assert!(elem.to_sql().sql().contains("->>"));
/// ```
pub fn json_get_text_idx<'a, E>(
    expr: E,
    index: i32,
) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->>".into()))
            .append(SQL::param(PostgresValue::Integer(index))),
    )
}

/// PostgreSQL `#>` operator - get JSON object at specified path, returns JSON.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::json_get_path;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let nested = json_get_path(data, "{a,b}");
/// assert!(nested.to_sql().sql().contains("#>"));
/// ```
pub fn json_get_path<'a, E>(
    expr: E,
    path: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Json, Null, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("#>".into()))
            .append(SQL::param(PostgresValue::Text(path.into()))),
    )
}

/// PostgreSQL `#>>` operator - get JSON object at specified path as text.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::json_get_path_text;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let nested = json_get_path_text(data, "{a,b}");
/// assert!(nested.to_sql().sql().contains("#>>"));
/// ```
pub fn json_get_path_text<'a, E>(
    expr: E,
    path: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("#>>".into()))
            .append(SQL::param(PostgresValue::Text(path.into()))),
    )
}

/// PostgreSQL `@>` operator for JSONB - left JSON contains right JSON.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::jsonb_contains;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let cond = jsonb_contains(data, r#"{"key": "value"}"#);
/// assert!(cond.to_sql().sql().contains("@>"));
/// ```
pub fn jsonb_contains<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: ToSQL<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("@>".into()))
            .append(right.to_sql()),
    )
}

/// PostgreSQL `<@` operator for JSONB - left JSON is contained by right JSON.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::jsonb_contained;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let cond = jsonb_contained(data, r#"{"key": "value", "other": 1}"#);
/// assert!(cond.to_sql().sql().contains("<@"));
/// ```
pub fn jsonb_contained<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: ToSQL<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("<@".into()))
            .append(right.to_sql()),
    )
}

/// PostgreSQL `?` operator for JSONB - does the key exist in the JSON object?
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::jsonb_exists_key;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let cond = jsonb_exists_key(data, "name");
/// assert!(cond.to_sql().sql().contains("?"));
/// ```
pub fn jsonb_exists_key<'a, E>(
    expr: E,
    key: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("?".into()))
            .append(SQL::param(PostgresValue::Text(key.into()))),
    )
}

/// PostgreSQL `?|` operator for JSONB - do any of the keys exist?
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::jsonb_exists_any;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let cond = jsonb_exists_any(data, &["name", "email"]);
/// assert!(cond.to_sql().sql().contains("?|"));
/// ```
pub fn jsonb_exists_any<'a, E>(
    expr: E,
    keys: &[&'a str],
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    let arr: Vec<PostgresValue<'a>> = keys
        .iter()
        .map(|k| PostgresValue::Text((*k).into()))
        .collect();
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("?|".into()))
            .append(SQL::param(PostgresValue::Array(arr))),
    )
}

/// PostgreSQL `?&` operator for JSONB - do all of the keys exist?
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::jsonb_exists_all;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let data = SQL::<PostgresValue>::raw("data");
/// let cond = jsonb_exists_all(data, &["name", "email"]);
/// assert!(cond.to_sql().sql().contains("?&"));
/// ```
pub fn jsonb_exists_all<'a, E>(
    expr: E,
    keys: &[&'a str],
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    let arr: Vec<PostgresValue<'a>> = keys
        .iter()
        .map(|k| PostgresValue::Text((*k).into()))
        .collect();
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("?&".into()))
            .append(SQL::param(PostgresValue::Array(arr))),
    )
}

/// Extension trait providing method-based JSON operators for PostgreSQL expressions.
pub trait JsonExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// Get JSON object field by key (`->` operator), returns JSON.
    fn json_get(self, key: &'a str) -> SQLExpr<'a, PostgresValue<'a>, Json, Null, Scalar> {
        json_get(self, key)
    }

    /// Get JSON array element by index (`->` operator), returns JSON.
    fn json_get_idx(self, index: i32) -> SQLExpr<'a, PostgresValue<'a>, Json, Null, Scalar> {
        json_get_idx(self, index)
    }

    /// Get JSON object field as text (`->>` operator).
    fn json_get_text(self, key: &'a str) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Scalar> {
        json_get_text(self, key)
    }

    /// Get JSON array element as text (`->>` operator).
    fn json_get_text_idx(self, index: i32) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Scalar> {
        json_get_text_idx(self, index)
    }

    /// Get JSON object at path (`#>` operator), returns JSON.
    fn json_get_path(self, path: &'a str) -> SQLExpr<'a, PostgresValue<'a>, Json, Null, Scalar> {
        json_get_path(self, path)
    }

    /// Get JSON object at path as text (`#>>` operator).
    fn json_get_path_text(
        self,
        path: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Scalar> {
        json_get_path_text(self, path)
    }

    /// JSONB contains (`@>` operator).
    fn jsonb_contains<R>(self, other: R) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
    where
        R: ToSQL<'a, PostgresValue<'a>>,
    {
        jsonb_contains(self, other)
    }

    /// JSONB is contained by (`<@` operator).
    fn jsonb_contained<R>(
        self,
        other: R,
    ) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
    where
        R: ToSQL<'a, PostgresValue<'a>>,
    {
        jsonb_contained(self, other)
    }

    /// JSONB key exists (`?` operator).
    fn jsonb_exists_key(
        self,
        key: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar> {
        jsonb_exists_key(self, key)
    }
}

/// Blanket implementation for all PostgreSQL `Expr` types.
impl<'a, E: Expr<'a, PostgresValue<'a>>> JsonExprExt<'a> for E {}
