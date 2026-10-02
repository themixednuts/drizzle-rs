//! Date and time functions.
//!
//! Arguments that hold a date or time must have a temporal SQL type (on
//! SQLite, text columns count as temporal, since SQLite stores dates as
//! text). Many functions exist on only one database and do not compile for
//! the others:
//!
//! - all dialects: [`current_date`], [`current_time`], [`current_timestamp`];
//! - SQLite: [`date`], [`time`], [`datetime`], [`strftime`], [`julianday`],
//!   [`unixepoch`], [`timediff`];
//! - PostgreSQL: [`now`], [`date_trunc`], [`extract`], [`age`], [`to_char`],
//!   [`to_timestamp`], [`to_date`], [`to_number`], [`date_bin`],
//!   [`make_date`], [`make_timestamp`], [`localtime`], [`localtimestamp`],
//!   [`clock_timestamp`].

use crate::dialect::DialectTypes;
use crate::dialect::{DialectSupports, feature};
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{DataType, Numeric, Temporal, Textual};
use crate::{PostgresDialect, SQLiteDialect};
use drizzle_types::postgres::types::{Timestamp as PgTimestamp, Timestamptz as PgTimestamptz};

use super::{AggregateKind, Expr, Nullability, SQLExpr, Scalar};

#[diagnostic::on_unimplemented(
    message = "DATE_TRUNC output type is not defined for `{Self}` on this dialect",
    label = "DATE_TRUNC accepts timestamp/timestamptz and preserves the timestamp flavor"
)]
/// Temporal types that [`date_trunc`] accepts on dialect `D`, and its result
/// type. On PostgreSQL, `timestamp` and `timestamptz` are accepted and keep
/// their type.
pub trait DateTruncPolicy<D>: Temporal {
    /// Result type of `DATE_TRUNC`.
    type Output: DataType;
}

impl DialectSupports<feature::SQLiteDateTime> for SQLiteDialect {}
impl DialectSupports<feature::PostgresDateTime> for PostgresDialect {}

impl DateTruncPolicy<PostgresDialect> for PgTimestamptz {
    type Output = Self;
}
impl DateTruncPolicy<PostgresDialect> for PgTimestamp {
    type Output = Self;
}

// =============================================================================
// CURRENT DATE/TIME (Cross-database)
// =============================================================================

/// The current date (`CURRENT_DATE`), on every dialect.
///
/// The result is the dialect's date type and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(current_date::<Value>().sql(), "CURRENT_DATE");
/// ```
#[must_use]
pub fn current_date<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Date, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_DATE"))
}

/// The current time (`CURRENT_TIME`), on every dialect.
///
/// The result is the dialect's time type and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(current_time::<Value>().sql(), "CURRENT_TIME");
/// ```
#[must_use]
pub fn current_time<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Time, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_TIME"))
}

/// The current date and time (`CURRENT_TIMESTAMP`), on every dialect.
///
/// The result is the dialect's timestamp-with-time-zone type (text on
/// SQLite, `timestamptz` on PostgreSQL, `TIMESTAMP` on MySQL) and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(current_timestamp::<Value>().sql(), "CURRENT_TIMESTAMP");
/// ```
#[must_use]
pub fn current_timestamp<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::TimestampTz, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_TIMESTAMP"))
}

// =============================================================================
// SQLite-specific DATE/TIME FUNCTIONS
// =============================================================================

/// The date part of a time value (`DATE`), on SQLite.
///
/// The argument must be temporal. The result is the dialect's date type and keeps the
/// argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(date(users.created_at).sql(), r#"DATE ("users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn date<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Date, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("DATE", expr.into_sql()))
}

/// The time part of a time value (`TIME`), on SQLite.
///
/// The argument must be temporal. The result is the dialect's time type and keeps the
/// argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(time(users.created_at).sql(), r#"TIME ("users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn time<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Time, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("TIME", expr.into_sql()))
}

/// A time value as `YYYY-MM-DD HH:MM:SS` (`DATETIME`), on SQLite.
///
/// The argument must be temporal. The result is the dialect's timestamp type and keeps the
/// argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(datetime(users.created_at).sql(), r#"DATETIME ("users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn datetime<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Timestamp,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("DATETIME", expr.into_sql()))
}

/// Formats a time value as text (`STRFTIME(format, expr)`), on SQLite.
///
/// `format` must be text and `expr` temporal. The result is text and keeps
/// `expr`'s nullability. Common format codes:
///
/// - `%Y` year, `%m` month (01-12), `%d` day (01-31)
/// - `%H` hour (00-23), `%M` minute, `%S` second
/// - `%s` Unix time, `%w` weekday (0-6, Sunday is 0), `%j` day of year
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let day = strftime("%Y-%m-%d", users.created_at);
/// assert_eq!(day.sql(), r#"STRFTIME (?, "users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn strftime<'a, V, F, E>(
    format: F,
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <F::Aggregate as AggregateKind>::Or<E::Aggregate>,
    (F::Sources, E::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func(
        "STRFTIME",
        format.into_sql().push(Token::COMMA).append(expr.into_sql()),
    ))
}

/// A time value as a Julian day number (`JULIANDAY`), on SQLite.
///
/// The argument must be temporal. The result is the dialect's double type and
/// keeps the argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(julianday(users.created_at).sql(), r#"JULIANDAY ("users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn julianday<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("JULIANDAY", expr.into_sql()))
}

/// A time value as seconds since 1970-01-01 (`UNIXEPOCH`), on SQLite 3.38+.
///
/// The argument must be temporal. The result is the dialect's big-integer
/// type and keeps the argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(unixepoch(users.created_at).sql(), r#"UNIXEPOCH ("users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn unixepoch<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("UNIXEPOCH", expr.into_sql()))
}

// =============================================================================
// PostgreSQL-specific DATE/TIME FUNCTIONS
// =============================================================================

/// The current date and time (`NOW()`), on PostgreSQL.
///
/// The result is `timestamptz` and never NULL. It is fixed for the whole
/// transaction; see [`clock_timestamp`] for the wall-clock time.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(now::<Value>().sql(), "NOW()");
/// ```
#[must_use]
pub fn now<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::TimestampTz, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
{
    SQLExpr::new(SQL::raw("NOW()"))
}

/// Truncates a timestamp to a unit (`DATE_TRUNC(unit, expr)`), on PostgreSQL.
///
/// `unit` is text such as `'hour'`, `'day'`, `'week'`, `'month'` or `'year'`.
/// `expr` must be `timestamp` or `timestamptz` (see [`DateTruncPolicy`]); the
/// result has the same type and keeps `expr`'s nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let month = date_trunc("month", users.created_at);
/// assert_eq!(month.sql(), r#"DATE_TRUNC ($1, "users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn date_trunc<'a, V, P, E>(
    precision: P,
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as DateTruncPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    <P::Aggregate as AggregateKind>::Or<E::Aggregate>,
    (P::Sources, E::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    E: Expr<'a, V>,
    E::SQLType: DateTruncPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        "DATE_TRUNC",
        precision
            .into_sql()
            .push(Token::COMMA)
            .append(expr.into_sql()),
    ))
}

/// One field of a time value (`EXTRACT(field FROM expr)`), on PostgreSQL.
///
/// `field` is written into the SQL as is, so pass a fixed field name such as
/// `"YEAR"`, `"MONTH"`, `"DAY"`, `"HOUR"`, `"DOW"` or `"EPOCH"`, never user
/// input. `expr` must be temporal. PostgreSQL 14+ returns `numeric`, so the
/// call is cast to `DOUBLE PRECISION`; the result is `float8` and keeps
/// `expr`'s nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let year = extract("YEAR", users.created_at);
/// assert_eq!(
///     year.sql(),
///     r#"CAST (EXTRACT( YEAR FROM "users"."created_at") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn extract<'a, 'f, V, E>(
    field: &'f str,
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate, E::Sources>
where
    'f: 'a,
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    // EXTRACT uses special syntax: EXTRACT(field FROM timestamp). PostgreSQL 14+
    // returns NUMERIC, so the result is cast to the declared double type.
    let extracted = SQL::raw("EXTRACT(")
        .append(SQL::raw(field))
        .append(SQL::raw(" FROM "))
        .append(expr.into_sql())
        .push(Token::RPAREN);
    SQLExpr::new(SQL::func(
        "CAST",
        extracted
            .push(Token::AS)
            .append(SQL::raw("DOUBLE PRECISION")),
    ))
}

/// The interval between two timestamps (`AGE(a, b)`, that is `a - b`), on PostgreSQL.
///
/// Both arguments must be temporal. The result is `interval`, nullable if
/// either argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let account_age = age(now(), users.created_at);
/// assert_eq!(account_age.sql(), r#"AGE (NOW(), "users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn age<'a, V, E1, E2>(
    timestamp1: E1,
    timestamp2: E2,
) -> SQLExpr<
    'a,
    V,
    drizzle_types::postgres::types::Interval,
    <E1::Nullable as Nullability>::Or<E2::Nullable>,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    E1: Expr<'a, V>,
    E1::SQLType: Temporal,
    E2: Expr<'a, V>,
    E2::SQLType: Temporal,
    E2::Nullable: Nullability,
{
    SQLExpr::new(SQL::func(
        "AGE",
        timestamp1
            .into_sql()
            .push(Token::COMMA)
            .append(timestamp2.into_sql()),
    ))
}

/// Formats a time value as text (`TO_CHAR(expr, format)`), on PostgreSQL.
///
/// `expr` must be temporal and `format` text. The result is text and keeps
/// `expr`'s nullability. Common patterns: `YYYY`, `MM`, `DD`, `HH24`, `MI`,
/// `SS`, `Day`, `Month`.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let day = to_char(users.created_at, "YYYY-MM-DD");
/// assert_eq!(day.sql(), r#"TO_CHAR ("users"."created_at", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn to_char<'a, V, E, F>(
    expr: E,
    format: F,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <E::Aggregate as AggregateKind>::Or<F::Aggregate>,
    (E::Sources, F::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
    F: Expr<'a, V>,
    F::SQLType: Textual,
{
    SQLExpr::new(SQL::func(
        "TO_CHAR",
        expr.into_sql().push(Token::COMMA).append(format.into_sql()),
    ))
}

/// Converts Unix time in seconds to a timestamp (`TO_TIMESTAMP`), on PostgreSQL.
///
/// The argument must be numeric. The result is `timestamptz` and keeps the
/// argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(to_timestamp(users.age).sql(), r#"TO_TIMESTAMP ("users"."age")"#);
/// ```
pub fn to_timestamp<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, PgTimestamptz, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("TO_TIMESTAMP", expr.into_sql()))
}

// =============================================================================
// Additional PostgreSQL Formatting Functions
// =============================================================================

/// Parses text into a date using a format (`TO_DATE(text, format)`), on PostgreSQL.
///
/// Both arguments must be text. The result is `date` and keeps the first
/// argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let d = to_date::<Value, _, _>("2024-01-15", "YYYY-MM-DD");
/// assert_eq!(d.sql(), "TO_DATE ($1, $2)");
/// ```
#[allow(clippy::type_complexity)]
pub fn to_date<'a, V, E, F>(
    expr: E,
    format: F,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Date,
    E::Nullable,
    <E::Aggregate as AggregateKind>::Or<F::Aggregate>,
    (E::Sources, F::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
{
    SQLExpr::new(SQL::func(
        "TO_DATE",
        expr.into_sql().push(Token::COMMA).append(format.into_sql()),
    ))
}

/// Parses text into a number using a format (`TO_NUMBER(text, format)`), on PostgreSQL.
///
/// Both arguments must be text. The result is `numeric` and keeps the first
/// argument's nullability.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let n = to_number(users.name, "9G999D99");
/// assert_eq!(n.sql(), r#"TO_NUMBER ("users"."name", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn to_number<'a, V, E, F>(
    expr: E,
    format: F,
) -> SQLExpr<
    'a,
    V,
    drizzle_types::postgres::types::Numeric,
    E::Nullable,
    <E::Aggregate as AggregateKind>::Or<F::Aggregate>,
    (E::Sources, F::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
{
    SQLExpr::new(SQL::func(
        "TO_NUMBER",
        expr.into_sql().push(Token::COMMA).append(format.into_sql()),
    ))
}

// =============================================================================
// DATE_BIN (PostgreSQL 14+)
// =============================================================================

/// Rounds a timestamp down into fixed-size buckets (`DATE_BIN`), on PostgreSQL 14+.
///
/// Buckets are `stride` wide (an interval as text, such as `'15 minutes'`) and
/// start at `origin`. `source` and `origin` must be temporal. The stride is
/// cast to `INTERVAL`. The result has `source`'s type and is nullable if any
/// argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let bucket = date_bin("15 minutes", users.created_at, localtimestamp());
/// assert_eq!(
///     bucket.sql(),
///     r#"DATE_BIN (CAST ($1 AS INTERVAL), "users"."created_at", LOCALTIMESTAMP)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn date_bin<'a, V, S, E, O>(
    stride: S,
    source: E,
    origin: O,
) -> SQLExpr<
    'a,
    V,
    E::SQLType,
    <<S::Nullable as Nullability>::Or<E::Nullable> as Nullability>::Or<O::Nullable>,
    <<S::Aggregate as AggregateKind>::Or<E::Aggregate> as AggregateKind>::Or<O::Aggregate>,
    (S::Sources, (E::Sources, O::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    S: Expr<'a, V>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
    O: Expr<'a, V>,
    O::SQLType: Temporal,
    E::Nullable: Nullability,
    O::Nullable: Nullability,
    O::Aggregate: super::AggregateKind,
{
    // The stride parameter binds as text; PostgreSQL resolves the overload at
    // prepare time, so the cast keeps the bound and inferred types aligned.
    SQLExpr::new(SQL::func(
        "DATE_BIN",
        super::math::pg_cast(stride.into_sql(), "INTERVAL")
            .push(Token::COMMA)
            .append(source.into_sql())
            .push(Token::COMMA)
            .append(origin.into_sql()),
    ))
}

// =============================================================================
// MAKE_DATE / MAKE_TIMESTAMP (PostgreSQL)
// =============================================================================

/// Builds a date from year, month and day (`MAKE_DATE`), on PostgreSQL.
///
/// All arguments must be numeric. The result is `date`, nullable if any
/// argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let d = make_date::<Value, _, _, _>(2024, 1, 15);
/// assert_eq!(d.sql(), "MAKE_DATE ($1, $2, $3)");
/// ```
#[allow(clippy::type_complexity)]
pub fn make_date<'a, V, Y, M, D>(
    year: Y,
    month: M,
    day: D,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Date,
    <<Y::Nullable as Nullability>::Or<M::Nullable> as Nullability>::Or<D::Nullable>,
    <<Y::Aggregate as AggregateKind>::Or<M::Aggregate> as AggregateKind>::Or<D::Aggregate>,
    (Y::Sources, (M::Sources, D::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    Y: Expr<'a, V>,
    Y::SQLType: Numeric,
    M: Expr<'a, V>,
    M::SQLType: Numeric,
    D: Expr<'a, V>,
    D::SQLType: Numeric,
    M::Nullable: Nullability,
    D::Nullable: Nullability,
    D::Aggregate: super::AggregateKind,
{
    SQLExpr::new(SQL::func(
        "MAKE_DATE",
        year.into_sql()
            .push(Token::COMMA)
            .append(month.into_sql())
            .push(Token::COMMA)
            .append(day.into_sql()),
    ))
}

/// Builds a timestamp from its parts (`MAKE_TIMESTAMP`), on PostgreSQL.
///
/// Takes year, month, day, hour, minute and seconds; all must be numeric
/// (seconds may have a fraction). The result is `timestamp`, nullable if any
/// argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let ts = make_timestamp::<Value, _, _, _, _, _, _>(2024, 1, 15, 10, 30, 0.0);
/// assert_eq!(ts.sql(), "MAKE_TIMESTAMP ($1, $2, $3, $4, $5, $6)");
/// ```
#[allow(clippy::type_complexity)]
pub fn make_timestamp<'a, V, Y, Mo, D, H, Mi, S>(
    year: Y,
    month: Mo,
    day: D,
    hour: H,
    minute: Mi,
    second: S,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Timestamp,
    <<<<Y::Nullable as Nullability>::Or<Mo::Nullable> as Nullability>::Or<D::Nullable> as Nullability>::Or<H::Nullable,> as Nullability>::Or<<Mi::Nullable as Nullability>::Or<S::Nullable>>,
    <<<<Y::Aggregate as AggregateKind>::Or<Mo::Aggregate> as AggregateKind>::Or<D::Aggregate> as AggregateKind>::Or<H::Aggregate,> as AggregateKind>::Or<<Mi::Aggregate as AggregateKind>::Or<S::Aggregate>>,
    (
        Y::Sources,
        (
            Mo::Sources,
            (D::Sources, (H::Sources, (Mi::Sources, S::Sources))),
        ),
    ),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
    Y: Expr<'a, V>,
    Y::SQLType: Numeric,
    Mo: Expr<'a, V>,
    Mo::SQLType: Numeric,
    D: Expr<'a, V>,
    D::SQLType: Numeric,
    H: Expr<'a, V>,
    H::SQLType: Numeric,
    Mi: Expr<'a, V>,
    Mi::SQLType: Numeric,
    S: Expr<'a, V>,
    S::SQLType: Numeric,
    H::Nullable: Nullability,
    D::Nullable: Nullability,
    Mo::Nullable: Nullability,
    S::Nullable: Nullability,
    H::Aggregate: super::AggregateKind,
    D::Aggregate: super::AggregateKind,
    S::Aggregate: super::AggregateKind,
{
    SQLExpr::new(SQL::func(
        "MAKE_TIMESTAMP",
        year.into_sql()
            .push(Token::COMMA)
            .append(month.into_sql())
            .push(Token::COMMA)
            .append(day.into_sql())
            .push(Token::COMMA)
            .append(hour.into_sql())
            .push(Token::COMMA)
            .append(minute.into_sql())
            .push(Token::COMMA)
            .append(second.into_sql()),
    ))
}

// =============================================================================
// Current Time (PostgreSQL-specific)
// =============================================================================

/// The current time without time zone (`LOCALTIME`), on PostgreSQL.
///
/// The result is `time` and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(localtime::<Value>().sql(), "LOCALTIME");
/// ```
#[must_use]
pub fn localtime<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Time, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
{
    SQLExpr::new(SQL::raw("LOCALTIME"))
}

/// The current date and time without time zone (`LOCALTIMESTAMP`), on PostgreSQL.
///
/// The result is `timestamp` and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(localtimestamp::<Value>().sql(), "LOCALTIMESTAMP");
/// ```
#[must_use]
pub fn localtimestamp<'a, V>() -> SQLExpr<'a, V, PgTimestamp, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
{
    SQLExpr::new(SQL::raw("LOCALTIMESTAMP"))
}

/// The actual wall-clock time (`CLOCK_TIMESTAMP()`), on PostgreSQL.
///
/// Unlike [`now`] and `CURRENT_TIMESTAMP`, the value changes during a
/// transaction. The result is `timestamptz` and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(clock_timestamp::<Value>().sql(), "CLOCK_TIMESTAMP()");
/// ```
#[must_use]
pub fn clock_timestamp<'a, V>() -> SQLExpr<'a, V, PgTimestamptz, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
{
    SQLExpr::new(SQL::raw("CLOCK_TIMESTAMP()"))
}

// =============================================================================
// TIMEDIFF (SQLite 3.43+)
// =============================================================================

/// The difference between two time values as text (`TIMEDIFF`), on SQLite 3.43+.
///
/// Both arguments must be temporal. The result is text in the form
/// `+YYYY-MM-DD HH:MM:SS.SSS`, nullable if either argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let since = timediff(current_timestamp(), users.created_at);
/// assert_eq!(since.sql(), r#"TIMEDIFF (CURRENT_TIMESTAMP, "users"."created_at")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn timediff<'a, V, E1, E2>(
    time1: E1,
    time2: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E1::Nullable as Nullability>::Or<E2::Nullable>,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteDateTime>,
    E1: Expr<'a, V>,
    E1::SQLType: Temporal,
    E2: Expr<'a, V>,
    E2::SQLType: Temporal,
    E2::Nullable: Nullability,
{
    SQLExpr::new(SQL::func(
        "TIMEDIFF",
        time1.into_sql().push(Token::COMMA).append(time2.into_sql()),
    ))
}
