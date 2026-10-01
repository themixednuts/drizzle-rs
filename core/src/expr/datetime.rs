//! Type-safe date/time functions.
//!
//! These functions work with `Temporal` types (Date, Time, Timestamp, `TimestampTz`)
//! and provide compile-time enforcement of temporal operations.
//!
//! # Database Compatibility
//!
//! Some functions are database-specific:
//! - `SQLite`: `date()`, `time()`, `datetime()`, `strftime()`, `julianday()`
//! - `PostgreSQL`: `now()`, `date_trunc()`, `extract()`, `age()`
//!
//! Cross-database functions try to use compatible SQL where possible.

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
pub trait DateTruncPolicy<D>: Temporal {
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

/// `CURRENT_DATE` - returns the current date.
///
/// Works on both `SQLite` and `PostgreSQL`.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::current_date;
///
/// // SELECT CURRENT_DATE
/// let today = current_date::<SQLiteValue>();
/// # "####;
/// ```
#[must_use]
pub fn current_date<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Date, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_DATE"))
}

/// `CURRENT_TIME` - returns the current time.
///
/// Works on both `SQLite` and `PostgreSQL`.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::current_time;
///
/// // SELECT CURRENT_TIME
/// let now_time = current_time::<SQLiteValue>();
/// # "####;
/// ```
#[must_use]
pub fn current_time<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Time, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_TIME"))
}

/// `CURRENT_TIMESTAMP` - returns the current timestamp with time zone.
///
/// Works on both `SQLite` and `PostgreSQL`. Returns `TimestampTz` because
/// the SQL standard defines `CURRENT_TIMESTAMP` as `timestamp with time zone`.
/// On `SQLite` (without chrono) this maps to `String`; on `PostgreSQL` it maps
/// to `DateTime<Utc>` (requires the `chrono` feature).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::current_timestamp;
///
/// // SELECT CURRENT_TIMESTAMP
/// let now = current_timestamp::<SQLiteValue>();
/// # "####;
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

/// DATE - extracts the date part from a temporal expression (`SQLite`).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::date;
///
/// // SELECT DATE(users.created_at)
/// let created_date = date(users.created_at);
/// # "####;
/// ```
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

/// TIME - extracts the time part from a temporal expression (`SQLite`).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::time;
///
/// // SELECT TIME(users.created_at)
/// let created_time = time(users.created_at);
/// # "####;
/// ```
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

/// DATETIME - creates a datetime from a temporal expression (`SQLite`).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::datetime;
///
/// // SELECT DATETIME(users.created_at)
/// let dt = datetime(users.created_at);
/// # "####;
/// ```
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

/// STRFTIME - formats a temporal expression as text (`SQLite`).
///
/// Returns Text type, preserves nullability of the time value.
///
/// # Format Specifiers (common)
///
/// - `%Y` - 4-digit year
/// - `%m` - month (01-12)
/// - `%d` - day of month (01-31)
/// - `%H` - hour (00-23)
/// - `%M` - minute (00-59)
/// - `%S` - second (00-59)
/// - `%s` - Unix timestamp
/// - `%w` - day of week (0-6, Sunday=0)
/// - `%j` - day of year (001-366)
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::strftime;
///
/// // SELECT STRFTIME('%Y-%m-%d', users.created_at)
/// let formatted = strftime("%Y-%m-%d", users.created_at);
/// # "####;
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

/// JULIANDAY - converts a temporal expression to Julian day number (`SQLite`).
///
/// Returns a dialect-aware double type, preserves nullability.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::julianday;
///
/// // SELECT JULIANDAY(users.created_at)
/// let julian = julianday(users.created_at);
/// # "####;
/// ```
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

/// UNIXEPOCH - converts a temporal expression to Unix timestamp (`SQLite` 3.38+).
///
/// Returns a dialect-aware `BigInt` type (seconds since 1970-01-01), preserves nullability.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::unixepoch;
///
/// // SELECT UNIXEPOCH(users.created_at)
/// let unix_ts = unixepoch(users.created_at);
/// # "####;
/// ```
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

/// NOW - returns the current timestamp with time zone (`PostgreSQL`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::now;
///
/// // SELECT NOW()
/// let current = now::<PostgresValue>();
/// # "####;
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

/// `DATE_TRUNC` - truncates a timestamp to specified precision (`PostgreSQL`).
///
/// Truncates the timestamp to the specified precision. Common values:
/// 'microseconds', 'milliseconds', 'second', 'minute', 'hour',
/// 'day', 'week', 'month', 'quarter', 'year', 'decade', 'century', 'millennium'
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::date_trunc;
///
/// // SELECT DATE_TRUNC('month', users.created_at)
/// let month_start = date_trunc("month", users.created_at);
/// # "####;
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

/// EXTRACT - extracts a component from a temporal expression (PostgreSQL/Standard SQL).
///
/// Returns a dialect-aware double type. Common fields:
/// 'year', 'month', 'day', 'hour', 'minute', 'second',
/// 'dow' (day of week), 'doy' (day of year), 'epoch' (Unix timestamp)
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::extract;
///
/// // SELECT EXTRACT(YEAR FROM users.created_at)
/// let year = extract("YEAR", users.created_at);
/// # "####;
/// ```
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

/// AGE - calculates the interval between two timestamps (`PostgreSQL`).
///
/// Returns `PostgreSQL` INTERVAL. The result is nullable if either input is nullable.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::age;
///
/// // SELECT AGE(NOW(), users.created_at)
/// let user_age = age(now(), users.created_at);
/// # "####;
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

/// `TO_CHAR` - formats a temporal expression as text (`PostgreSQL`).
///
/// Returns Text type, preserves nullability of the input expression.
///
/// # Common Format Patterns
///
/// - `YYYY` - 4-digit year
/// - `MM` - month (01-12)
/// - `DD` - day of month (01-31)
/// - `HH24` - hour (00-23)
/// - `MI` - minute (00-59)
/// - `SS` - second (00-59)
/// - `Day` - full day name
/// - `Month` - full month name
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::to_char;
///
/// // SELECT TO_CHAR(users.created_at, 'YYYY-MM-DD')
/// let formatted = to_char(users.created_at, "YYYY-MM-DD");
/// # "####;
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

/// `TO_TIMESTAMP` - converts a Unix timestamp to a timestamp (`PostgreSQL`).
///
/// Returns `TimestampTz` type. The input should be a numeric Unix timestamp.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::to_timestamp;
///
/// // SELECT TO_TIMESTAMP(users.created_unix)
/// let ts = to_timestamp(users.created_unix);
/// # "####;
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

/// `TO_DATE` - parses a date from text using a format pattern (`PostgreSQL`).
///
/// Returns Date type, preserves nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::to_date;
///
/// // SELECT TO_DATE('2024-01-15', 'YYYY-MM-DD')
/// let d = to_date("2024-01-15", "YYYY-MM-DD");
/// # "####;
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

/// `TO_NUMBER` - parses a number from text using a format pattern (`PostgreSQL`).
///
/// Returns Numeric type, preserves nullability of the input expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::to_number;
///
/// // SELECT TO_NUMBER('1,234.56', '9G999D99')
/// let n = to_number("1,234.56", "9G999D99");
/// # "####;
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

/// `DATE_BIN` - bins timestamps into intervals (`PostgreSQL` 14+).
///
/// Rounds a timestamp down to the nearest multiple of `stride` from `origin`.
/// Useful for time-series bucketing.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::date_bin;
///
/// // SELECT DATE_BIN('15 minutes', events.created_at, TIMESTAMP '2001-01-01')
/// let bucketed = date_bin("15 minutes", events.created_at, "2001-01-01");
/// # "####;
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

/// `MAKE_DATE` - constructs a date from year, month, day (`PostgreSQL`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::make_date;
///
/// // SELECT MAKE_DATE(2024, 1, 15)
/// let d = make_date(2024, 1, 15);
/// # "####;
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

/// `MAKE_TIMESTAMP` - constructs a timestamp from components (`PostgreSQL`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::make_timestamp;
///
/// // SELECT MAKE_TIMESTAMP(2024, 1, 15, 10, 30, 0.0)
/// let ts = make_timestamp(2024, 1, 15, 10, 30, 0.0);
/// # "####;
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

/// LOCALTIME - returns the current time without time zone (`PostgreSQL`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::localtime;
///
/// // SELECT LOCALTIME
/// let now_time = localtime::<PostgresValue>();
/// # "####;
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

/// LOCALTIMESTAMP - returns the current timestamp without time zone (`PostgreSQL`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::localtimestamp;
///
/// // SELECT LOCALTIMESTAMP
/// let now_ts = localtimestamp::<PostgresValue>();
/// # "####;
/// ```
#[must_use]
pub fn localtimestamp<'a, V>() -> SQLExpr<'a, V, PgTimestamp, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresDateTime>,
{
    SQLExpr::new(SQL::raw("LOCALTIMESTAMP"))
}

/// `CLOCK_TIMESTAMP` - returns the actual wall-clock time (`PostgreSQL`).
///
/// Unlike `NOW()` or `CURRENT_TIMESTAMP`, this changes during a transaction.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::clock_timestamp;
///
/// // SELECT CLOCK_TIMESTAMP()
/// let wall_clock = clock_timestamp::<PostgresValue>();
/// # "####;
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

/// TIMEDIFF - computes the difference between two temporal values (`SQLite` 3.43+).
///
/// Returns a text representation of the time difference.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::timediff;
///
/// // SELECT TIMEDIFF(events.end_time, events.start_time)
/// let duration = timediff(events.end_time, events.start_time);
/// # "####;
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
