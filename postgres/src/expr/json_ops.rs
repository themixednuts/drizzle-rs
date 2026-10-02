//! `PostgreSQL` JSON and JSONB operators. Documented in [`crate::expr`].

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::PostgresValue;
use drizzle_core::expr::{AggregateKind, Expr, NonNull, Null, SQLExpr};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQL, SQLChunk, Token};

/// Wraps a containment operand in `CAST(operand AS JSONB)`.
///
/// A bound `serde_json::Value` is declared as `json` by the drivers and a text
/// literal as `text`; neither resolves `jsonb @> ...` without the cast.
fn jsonb_operand<'a>(operand: SQL<'a, PostgresValue<'a>>) -> SQL<'a, PostgresValue<'a>> {
    SQL::func("CAST", operand.push(Token::AS).append(SQL::raw("JSONB")))
}

/// Binds `value` as `CAST($n AS type_name)`.
///
/// PostgreSQL infers untyped parameters at prepare time and picks the `text`
/// overload of `->` / `->>` / `#>`; without the cast the driver binds an
/// integer or array where the server expects text and the query fails.
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
///
/// Implemented for `json`, `jsonb`, and untyped SQL ([`Any`], treated as
/// `json`). Text columns are rejected even when they hold JSON; cast them first.
///
/// # Type safety
///
/// ```compile_fail
/// use drizzle_core::expr::raw_non_null;
/// use drizzle_postgres::expr::json_get_text;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Text;
///
/// let raw = raw_non_null::<PostgresValue, Text>("raw");
/// let _ = json_get_text(raw, "name"); // `text` is not a JSON type
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a PostgreSQL JSON type",
    label = "JSON operators need a `json` or `jsonb` operand"
)]
pub trait JsonType {
    /// The SQL type `->` and `#>` return: `json` for `json`, `jsonb` for `jsonb`,
    /// and `json` for untyped SQL.
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

/// SQL types the JSONB-only operators (`@>`, `<@`, `?`, `?|`, `?&`) accept
/// as their left operand.
///
/// Implemented for `jsonb` and untyped SQL ([`Any`]). PostgreSQL has none of
/// these operators for `json`, so a `json` column is rejected.
///
/// # Type safety
///
/// ```compile_fail
/// use drizzle_core::expr::raw_non_null;
/// use drizzle_postgres::expr::jsonb_exists_key;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Json;
///
/// let data = raw_non_null::<PostgresValue, Json>("data");
/// let _ = jsonb_exists_key(data, "name"); // `json`, not `jsonb`
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not `jsonb`",
    label = "this operator exists only for `jsonb`",
    note = "PostgreSQL has no `@>`, `<@`, `?`, `?|` or `?&` for `json`; declare the column as `jsonb`"
)]
pub trait JsonbType {}

impl JsonbType for Jsonb {}
impl JsonbType for Any {}

/// SQL types accepted as the right operand of [`jsonb_contains`] (`@>`) and
/// [`jsonb_contained`] (`<@`).
///
/// The operand is rendered as `CAST(operand AS JSONB)`, so it may be `json`,
/// `jsonb`, JSON text (`text`, `varchar`, such as a `&str` literal), or
/// untyped SQL ([`Any`]). Other types, such as integers, are rejected.
///
/// # Type safety
///
/// ```compile_fail
/// use drizzle_core::expr::raw_non_null;
/// use drizzle_postgres::expr::jsonb_contains;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let _ = jsonb_contains(data, 42_i32); // an integer is not JSON
/// ```
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

/// Gets an object field by key (`->`), keeping the JSON type.
///
/// The operand must be `json` or `jsonb` ([`JsonType`]); the result has the
/// same type, so calls can be chained. The result is NULL when the key is
/// missing.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::json_get;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let address = json_get(data, "address"); // a `jsonb` expression
/// assert_eq!(address.to_sql().sql(), "data -> CAST ($1 AS TEXT)");
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

/// Gets an array element by zero-based index (`->`), keeping the JSON type.
///
/// Negative indexes count from the end. The operand must be `json` or `jsonb`
/// ([`JsonType`]). The result is NULL when the index is out of range.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::json_get_idx;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let first = json_get_idx(data, 0);
/// assert_eq!(first.to_sql().sql(), "data -> CAST ($1 AS INTEGER)");
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

/// Gets an object field by key as `text` (`->>`).
///
/// The operand must be `json` or `jsonb` ([`JsonType`]). The result is NULL
/// when the key is missing or the value is JSON `null`.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::json_get_text;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let name = json_get_text(data, "name"); // a `text` expression
/// assert_eq!(name.to_sql().sql(), "data ->> CAST ($1 AS TEXT)");
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

/// Gets an array element by zero-based index as `text` (`->>`).
///
/// The operand must be `json` or `jsonb` ([`JsonType`]). The result is NULL
/// when the index is out of range.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::json_get_text_idx;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let first = json_get_text_idx(data, 0);
/// assert_eq!(first.to_sql().sql(), "data ->> CAST ($1 AS INTEGER)");
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

/// Gets the value at a path (`#>`), keeping the JSON type.
///
/// `path` is a `PostgreSQL` text-array literal such as `"{address,city}"`;
/// array indexes go in the path as numbers (`"{tags,0}"`). The operand must be
/// `json` or `jsonb` ([`JsonType`]). The result is NULL when the path does not exist.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::json_get_path;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let city = json_get_path(data, "{address,city}");
/// assert_eq!(city.to_sql().sql(), "data #> CAST ($1 AS TEXT[])");
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

/// Gets the value at a path as `text` (`#>>`).
///
/// `path` is a `PostgreSQL` text-array literal such as `"{address,city}"`.
/// The operand must be `json` or `jsonb` ([`JsonType`]). The result is NULL
/// when the path does not exist.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::json_get_path_text;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let city = json_get_path_text(data, "{address,city}");
/// assert_eq!(city.to_sql().sql(), "data #>> CAST ($1 AS TEXT[])");
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

/// Tests whether the left `jsonb` value contains the right one (`@>`).
///
/// The left operand must be `jsonb` ([`JsonbType`]). The right operand may be
/// `json`, `jsonb`, or JSON text ([`JsonbOperand`]); it is cast to `jsonb`.
/// The result is NULL when either operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::jsonb_contains;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let cond = jsonb_contains(data, r#"{"role": "admin"}"#);
/// assert_eq!(cond.to_sql().sql(), "data @> CAST ($1 AS JSONB)");
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

/// Tests whether the left `jsonb` value is contained in the right one (`<@`).
///
/// The left operand must be `jsonb` ([`JsonbType`]). The right operand may be
/// `json`, `jsonb`, or JSON text ([`JsonbOperand`]); it is cast to `jsonb`.
/// The result is NULL when either operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::jsonb_contained;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let cond = jsonb_contained(data, r#"{"role": "admin", "active": true}"#);
/// assert_eq!(cond.to_sql().sql(), "data <@ CAST ($1 AS JSONB)");
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

/// Tests whether a key exists at the top level of a `jsonb` value (`?`).
///
/// The operand must be `jsonb` ([`JsonbType`]). For a `jsonb` array, the
/// test matches string elements instead. The result is NULL when the operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::jsonb_exists_key;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let cond = jsonb_exists_key(data, "email");
/// assert_eq!(cond.to_sql().sql(), "data ? CAST ($1 AS TEXT)");
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

/// Tests whether any of the keys exists at the top level of a `jsonb` value (`?|`).
///
/// The operand must be `jsonb` ([`JsonbType`]). The keys are bound as one
/// `text[]` parameter. The result is NULL when the operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::jsonb_exists_any;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let cond = jsonb_exists_any(data, &["email", "phone"]);
/// assert_eq!(cond.to_sql().sql(), "data ?| $1");
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

/// Tests whether all of the keys exist at the top level of a `jsonb` value (`?&`).
///
/// The operand must be `jsonb` ([`JsonbType`]). The keys are bound as one
/// `text[]` parameter. The result is NULL when the operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::jsonb_exists_all;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let data = raw_non_null::<PostgresValue, Jsonb>("data");
/// let cond = jsonb_exists_all(data, &["name", "email"]);
/// assert_eq!(cond.to_sql().sql(), "data ?& $1");
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

/// Method forms of the JSON operators, available on every `PostgreSQL` expression.
///
/// Each method calls the free function of the same name and has the same
/// operand rules ([`JsonType`], [`JsonbType`], [`JsonbOperand`]).
/// `?|` and `?&` have no method form; use [`jsonb_exists_any`] and
/// [`jsonb_exists_all`].
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::JsonExprExt;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Jsonb;
///
/// let settings = raw_non_null::<PostgresValue, Jsonb>("settings");
/// let theme = settings.json_get_path_text("{ui,theme}");
/// assert_eq!(theme.to_sql().sql(), "settings #>> CAST ($1 AS TEXT[])");
/// ```
pub trait JsonExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// Gets an object field by key (`->`), keeping the JSON type. See [`json_get`].
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

    /// Gets an array element by index (`->`), keeping the JSON type. See [`json_get_idx`].
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

    /// Gets an object field by key as `text` (`->>`). See [`json_get_text`].
    fn json_get_text(
        self,
        key: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Self::Aggregate, Self::Sources>
    where
        Self::SQLType: JsonType,
    {
        json_get_text(self, key)
    }

    /// Gets an array element by index as `text` (`->>`). See [`json_get_text_idx`].
    fn json_get_text_idx(
        self,
        index: i32,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Self::Aggregate, Self::Sources>
    where
        Self::SQLType: JsonType,
    {
        json_get_text_idx(self, index)
    }

    /// Gets the value at a path (`#>`), keeping the JSON type. See [`json_get_path`].
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

    /// Gets the value at a path as `text` (`#>>`). See [`json_get_path_text`].
    fn json_get_path_text(
        self,
        path: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Text, Null, Self::Aggregate, Self::Sources>
    where
        Self::SQLType: JsonType,
    {
        json_get_path_text(self, path)
    }

    /// Tests whether `self` contains `other` (`@>`, `jsonb` only). See [`jsonb_contains`].
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

    /// Tests whether `self` is contained in `other` (`<@`, `jsonb` only). See [`jsonb_contained`].
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

    /// Tests whether a top-level key exists (`?`, `jsonb` only). See [`jsonb_exists_key`].
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

impl<'a, E: Expr<'a, PostgresValue<'a>>> JsonExprExt<'a> for E {}
