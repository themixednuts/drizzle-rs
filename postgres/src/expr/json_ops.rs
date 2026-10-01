//! `PostgreSQL` JSON/JSONB operators.
//!
//! Provides type-safe access to `PostgreSQL` JSON operators:
//! - `->` (get JSON object field by key, returns JSON)
//! - `->>` (get JSON object field by key, returns text)
//! - `#>` (get JSON object at path, returns JSON)
//! - `#>>` (get JSON object at path, returns text)
//! - `@>` (JSON contains)
//! - `?` (JSON key exists)

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::PostgresValue;
use drizzle_core::expr::{AggregateKind, Expr, NonNull, Null, SQLExpr};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQL, SQLChunk, Token};

/// `CAST($n AS type)` around an operator argument.
///
/// PostgreSQL infers untyped parameters at prepare time and picks the `text`
/// overload of `->` / `->>` / `#>`; without the cast the driver binds an
/// integer or array where the server expects text and the query fails.
/// `CAST(operand AS JSONB)` for containment operands.
///
/// A bound `serde_json::Value` is declared as `json` by the drivers and a text
/// literal as `text`; neither resolves `jsonb @> ...` without the cast.
fn jsonb_operand<'a>(operand: SQL<'a, PostgresValue<'a>>) -> SQL<'a, PostgresValue<'a>> {
    SQL::func("CAST", operand.push(Token::AS).append(SQL::raw("JSONB")))
}

fn typed_param<'a>(
    value: PostgresValue<'a>,
    type_name: &'static str,
) -> SQL<'a, PostgresValue<'a>> {
    SQL::func(
        "CAST",
        SQL::param(value)
            .push(Token::AS)
            .append(SQL::raw(type_name)),
    )
}

use drizzle_types::postgres::types::{Any, Boolean, Json, Jsonb, Text, Varchar};

/// SQL types the JSON access operators (`->`, `->>`, `#>`, `#>>`) accept.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a PostgreSQL JSON type",
    label = "JSON operators need a `json` or `jsonb` operand"
)]
pub trait JsonType {
    /// The type `->` and `#>` return: `json` for `json`, `jsonb` for `jsonb`.
    type Field: drizzle_types::DataType;
}

impl JsonType for Json {
    type Field = Json;
}
impl JsonType for Jsonb {
    type Field = Jsonb;
}
impl JsonType for Any {
    type Field = Json;
}

/// SQL types the JSONB-only operators (`@>`, `<@`, `?`, `?|`, `?&`) accept.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not `jsonb`",
    label = "this operator exists only for `jsonb`",
    note = "PostgreSQL has no `@>`, `<@`, `?`, `?|` or `?&` for `json`; declare the column as `jsonb`"
)]
pub trait JsonbType {}

impl JsonbType for Jsonb {}
impl JsonbType for Any {}

/// Right operand types of `@>` / `<@`. The operand is cast to `jsonb`, so JSON
/// text is accepted as well.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be cast to `jsonb` for a containment check",
    label = "expected a JSON value, JSON text, or untyped SQL"
)]
pub trait JsonbOperand {}

impl JsonbOperand for Json {}
impl JsonbOperand for Jsonb {}
impl JsonbOperand for Text {}
impl JsonbOperand for Varchar {}
impl JsonbOperand for Any {}

/// `PostgreSQL` `->` operator - get JSON object field by key, returns JSON.
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
pub fn json_get<'a, E>(
    expr: E,
    key: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, <E::SQLType as JsonType>::Field, Null, E::Aggregate, E::Sources>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->".into()))
            .append(typed_param(PostgresValue::Text(key.into()), "TEXT")),
    )
}

/// `PostgreSQL` `->` operator with integer index - get JSON array element.
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
) -> SQLExpr<'a, PostgresValue<'a>, <E::SQLType as JsonType>::Field, Null, E::Aggregate, E::Sources>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->".into()))
            .append(typed_param(PostgresValue::Integer(index), "INTEGER")),
    )
}

/// `PostgreSQL` `->>` operator - get JSON object field as text.
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
) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, E::Aggregate, E::Sources>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->>".into()))
            .append(typed_param(PostgresValue::Text(key.into()), "TEXT")),
    )
}

/// `PostgreSQL` `->>` operator with integer index - get JSON array element as text.
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
) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, E::Aggregate, E::Sources>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("->>".into()))
            .append(typed_param(PostgresValue::Integer(index), "INTEGER")),
    )
}

/// `PostgreSQL` `#>` operator - get JSON object at specified path, returns JSON.
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
) -> SQLExpr<'a, PostgresValue<'a>, <E::SQLType as JsonType>::Field, Null, E::Aggregate, E::Sources>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("#>".into()))
            .append(typed_param(PostgresValue::Text(path.into()), "TEXT[]")),
    )
}

/// `PostgreSQL` `#>>` operator - get JSON object at specified path as text.
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
) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, E::Aggregate, E::Sources>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("#>>".into()))
            .append(typed_param(PostgresValue::Text(path.into()), "TEXT[]")),
    )
}

/// `PostgreSQL` `@>` operator for JSONB - left JSON contains right JSON.
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
#[allow(clippy::type_complexity)]
pub fn jsonb_contains<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    L::SQLType: JsonbType,
    R: Expr<'a, PostgresValue<'a>>,
    R::SQLType: JsonbOperand,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("@>".into()))
            .append(jsonb_operand(right.to_sql())),
    )
}

/// `PostgreSQL` `<@` operator for JSONB - left JSON is contained by right JSON.
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
#[allow(clippy::type_complexity)]
pub fn jsonb_contained<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    L::SQLType: JsonbType,
    R: Expr<'a, PostgresValue<'a>>,
    R::SQLType: JsonbOperand,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("<@".into()))
            .append(jsonb_operand(right.to_sql())),
    )
}

/// `PostgreSQL` `?` operator for JSONB - does the key exist in the JSON object?
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
#[allow(clippy::type_complexity)]
pub fn jsonb_exists_key<'a, E>(
    expr: E,
    key: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonbType,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("?".into()))
            .append(typed_param(PostgresValue::Text(key.into()), "TEXT")),
    )
}

/// `PostgreSQL` `?|` operator for JSONB - do any of the keys exist?
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
#[allow(clippy::type_complexity)]
pub fn jsonb_exists_any<'a, E>(
    expr: E,
    keys: &[&'a str],
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonbType,
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

/// `PostgreSQL` `?&` operator for JSONB - do all of the keys exist?
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
#[allow(clippy::type_complexity)]
pub fn jsonb_exists_all<'a, E>(
    expr: E,
    keys: &[&'a str],
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: JsonbType,
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

/// Extension trait providing method-based JSON operators for `PostgreSQL` expressions.
pub trait JsonExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// Get JSON object field by key (`->` operator), returns JSON.
    fn json_get(
        self,
        key: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        <Self::SQLType as JsonType>::Field,
        Null,
        Self::Aggregate,
        Self::Sources,
    >
    where
        Self::SQLType: JsonType,
    {
        json_get(self, key)
    }

    /// Get JSON array element by index (`->` operator), returns JSON.
    fn json_get_idx(
        self,
        index: i32,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        <Self::SQLType as JsonType>::Field,
        Null,
        Self::Aggregate,
        Self::Sources,
    >
    where
        Self::SQLType: JsonType,
    {
        json_get_idx(self, index)
    }

    /// Get JSON object field as text (`->>` operator).
    fn json_get_text(
        self,
        key: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Self::Aggregate, Self::Sources>
    where
        Self::SQLType: JsonType,
    {
        json_get_text(self, key)
    }

    /// Get JSON array element as text (`->>` operator).
    fn json_get_text_idx(
        self,
        index: i32,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Self::Aggregate, Self::Sources>
    where
        Self::SQLType: JsonType,
    {
        json_get_text_idx(self, index)
    }

    /// Get JSON object at path (`#>` operator), returns JSON.
    fn json_get_path(
        self,
        path: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        <Self::SQLType as JsonType>::Field,
        Null,
        Self::Aggregate,
        Self::Sources,
    >
    where
        Self::SQLType: JsonType,
    {
        json_get_path(self, path)
    }

    /// Get JSON object at path as text (`#>>` operator).
    fn json_get_path_text(
        self,
        path: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Self::Aggregate, Self::Sources>
    where
        Self::SQLType: JsonType,
    {
        json_get_path_text(self, path)
    }

    /// JSONB contains (`@>` operator).
    #[allow(clippy::type_complexity)]
    fn jsonb_contains<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<R::Aggregate>,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        Self::SQLType: JsonbType,
        R: Expr<'a, PostgresValue<'a>>,
        R::SQLType: JsonbOperand,
    {
        jsonb_contains(self, other)
    }

    /// JSONB is contained by (`<@` operator).
    #[allow(clippy::type_complexity)]
    fn jsonb_contained<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<R::Aggregate>,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        Self::SQLType: JsonbType,
        R: Expr<'a, PostgresValue<'a>>,
        R::SQLType: JsonbOperand,
    {
        jsonb_contained(self, other)
    }

    /// JSONB key exists (`?` operator).
    #[allow(clippy::type_complexity)]
    fn jsonb_exists_key(
        self,
        key: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        Self::Aggregate,
        Arg<Self::Nullable, Self::Sources>,
    >
    where
        Self::SQLType: JsonbType,
    {
        jsonb_exists_key(self, key)
    }
}

/// Blanket implementation for all `PostgreSQL` `Expr` types.
impl<'a, E: Expr<'a, PostgresValue<'a>>> JsonExprExt<'a> for E {}
