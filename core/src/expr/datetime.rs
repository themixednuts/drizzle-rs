//! Type-safe date/time functions.
//!
//! These functions work with `Temporal` types (Date, Time, Timestamp, TimestampTz)
//! and provide compile-time enforcement of temporal operations.
//!
//! # Database Compatibility
//!
//! Some functions are database-specific:
//! - SQLite: `date()`, `time()`, `datetime()`, `strftime()`, `julianday()`
//! - PostgreSQL: `now()`, `date_trunc()`, `extract()`, `age()`
//!
//! Cross-database functions try to use compatible SQL where possible.

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{DataType, Numeric, Temporal};
use crate::{PostgresDialect, SQLiteDialect};
use drizzle_types::postgres::types::{Timestamp as PgTimestamp, Timestamptz as PgTimestamptz};

use super::{Expr, NullOr, Nullability, SQLExpr, Scalar};

#[diagnostic::on_unimplemented(
    message = "this date/time function is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
pub trait SQLiteDateTimeSupport {}

#[diagnostic::on_unimplemented(
    message = "this date/time function is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
pub trait PostgresDateTimeSupport {}

#[diagnostic::on_unimplemented(
    message = "DATE_TRUNC output type is not defined for `{Self}` on this dialect",
    label = "DATE_TRUNC accepts timestamp/timestamptz and preserves the timestamp flavor"
)]
pub trait DateTruncPolicy<D>: Temporal {
    type Output: DataType;
}

impl SQLiteDateTimeSupport for SQLiteDialect {}
impl PostgresDateTimeSupport for PostgresDialect {}

impl DateTruncPolicy<PostgresDialect> for PgTimestamp {
    type Output = PgTimestamp;
}
impl DateTruncPolicy<PostgresDialect> for PgTimestamptz {
    type Output = PgTimestamptz;
}

// =============================================================================
// CURRENT DATE/TIME (Cross-database)
// =============================================================================

/// CURRENT_DATE - returns the current date.
///
/// Works on both SQLite and PostgreSQL.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::current_date;
///
/// // SELECT CURRENT_DATE
/// let today = current_date::<SQLiteValue>();
/// ```
pub fn current_date<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Date, super::NonNull, Scalar>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_DATE"))
}

/// CURRENT_TIME - returns the current time.
///
/// Works on both SQLite and PostgreSQL.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::current_time;
///
/// // SELECT CURRENT_TIME
/// let now_time = current_time::<SQLiteValue>();
/// ```
pub fn current_time<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Time, super::NonNull, Scalar>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_TIME"))
}

/// CURRENT_TIMESTAMP - returns the current timestamp with time zone.
///
/// Works on both SQLite and PostgreSQL. Returns `TimestampTz` because
/// the SQL standard defines `CURRENT_TIMESTAMP` as `timestamp with time zone`.
/// On SQLite (without chrono) this maps to `String`; on PostgreSQL it maps
/// to `DateTime<Utc>` (requires the `chrono` feature).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::current_timestamp;
///
/// // SELECT CURRENT_TIMESTAMP
/// let now = current_timestamp::<SQLiteValue>();
/// ```
pub fn current_timestamp<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::TimestampTz, super::NonNull, Scalar>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("CURRENT_TIMESTAMP"))
}

// =============================================================================
// SQLite-specific DATE/TIME FUNCTIONS
// =============================================================================

/// DATE - extracts the date part from a temporal expression (SQLite).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::date;
///
/// // SELECT DATE(users.created_at)
/// let created_date = date(users.created_at);
/// ```
pub fn date<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Date, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("DATE", expr.into_sql()))
}

/// TIME - extracts the time part from a temporal expression (SQLite).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::time;
///
/// // SELECT TIME(users.created_at)
/// let created_time = time(users.created_at);
/// ```
pub fn time<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Time, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("TIME", expr.into_sql()))
}

/// DATETIME - creates a datetime from a temporal expression (SQLite).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::datetime;
///
/// // SELECT DATETIME(users.created_at)
/// let dt = datetime(users.created_at);
/// ```
pub fn datetime<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Timestamp, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("DATETIME", expr.into_sql()))
}

/// STRFTIME - formats a temporal expression as text (SQLite).
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
/// ```ignore
/// use drizzle_core::expr::strftime;
///
/// // SELECT STRFTIME('%Y-%m-%d', users.created_at)
/// let formatted = strftime("%Y-%m-%d", users.created_at);
/// ```
pub fn strftime<'a, V, F, E>(
    format: F,
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteDateTimeSupport,
    F: Expr<'a, V>,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func(
        "STRFTIME",
        format.into_sql().push(Token::COMMA).append(expr.into_sql()),
    ))
}

/// JULIANDAY - converts a temporal expression to Julian day number (SQLite).
///
/// Returns a dialect-aware double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::julianday;
///
/// // SELECT JULIANDAY(users.created_at)
/// let julian = julianday(users.created_at);
/// ```
pub fn julianday<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("JULIANDAY", expr.into_sql()))
}

/// UNIXEPOCH - converts a temporal expression to Unix timestamp (SQLite 3.38+).
///
/// Returns a dialect-aware BigInt type (seconds since 1970-01-01), preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::unixepoch;
///
/// // SELECT UNIXEPOCH(users.created_at)
/// let unix_ts = unixepoch(users.created_at);
/// ```
pub fn unixepoch<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    SQLExpr::new(SQL::func("UNIXEPOCH", expr.into_sql()))
}

// =============================================================================
// PostgreSQL-specific DATE/TIME FUNCTIONS
// =============================================================================

/// NOW - returns the current timestamp with time zone (PostgreSQL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::now;
///
/// // SELECT NOW()
/// let current = now::<PostgresValue>();
/// ```
pub fn now<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::TimestampTz, super::NonNull, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresDateTimeSupport,
{
    SQLExpr::new(SQL::raw("NOW()"))
}

/// DATE_TRUNC - truncates a timestamp to specified precision (PostgreSQL).
///
/// Truncates the timestamp to the specified precision. Common values:
/// 'microseconds', 'milliseconds', 'second', 'minute', 'hour',
/// 'day', 'week', 'month', 'quarter', 'year', 'decade', 'century', 'millennium'
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::date_trunc;
///
/// // SELECT DATE_TRUNC('month', users.created_at)
/// let month_start = date_trunc("month", users.created_at);
/// ```
#[allow(clippy::type_complexity)]
pub fn date_trunc<'a, V, P, E>(
    precision: P,
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as DateTruncPolicy<V::DialectMarker>>::Output, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresDateTimeSupport,
    P: Expr<'a, V>,
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
/// ```ignore
/// use drizzle_core::expr::extract;
///
/// // SELECT EXTRACT(YEAR FROM users.created_at)
/// let year = extract("YEAR", users.created_at);
/// ```
pub fn extract<'a, 'f, V, E>(
    field: &'f str,
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, Scalar>
where
    'f: 'a,
    V: SQLParam + 'a,
    V::DialectMarker: PostgresDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
{
    // EXTRACT uses special syntax: EXTRACT(field FROM timestamp)
    SQLExpr::new(
        SQL::raw("EXTRACT(")
            .append(SQL::raw(field))
            .append(SQL::raw(" FROM "))
            .append(expr.into_sql())
            .push(Token::RPAREN),
    )
}

/// AGE - calculates the interval between two timestamps (PostgreSQL).
///
/// Returns Text (interval representation). The result is nullable if either input is nullable.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::age;
///
/// // SELECT AGE(NOW(), users.created_at)
/// let user_age = age(now(), users.created_at);
/// ```
#[allow(clippy::type_complexity)]
pub fn age<'a, V, E1, E2>(
    timestamp1: E1,
    timestamp2: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E1::Nullable as NullOr<E2::Nullable>>::Output,
    Scalar,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresDateTimeSupport,
    E1: Expr<'a, V>,
    E1::SQLType: Temporal,
    E2: Expr<'a, V>,
    E2::SQLType: Temporal,
    E1::Nullable: NullOr<E2::Nullable>,
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

/// TO_CHAR - formats a temporal expression as text (PostgreSQL).
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
/// ```ignore
/// use drizzle_core::expr::to_char;
///
/// // SELECT TO_CHAR(users.created_at, 'YYYY-MM-DD')
/// let formatted = to_char(users.created_at, "YYYY-MM-DD");
/// ```
pub fn to_char<'a, V, E, F>(
    expr: E,
    format: F,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Temporal,
    F: Expr<'a, V>,
{
    SQLExpr::new(SQL::func(
        "TO_CHAR",
        expr.into_sql().push(Token::COMMA).append(format.into_sql()),
    ))
}

/// TO_TIMESTAMP - converts a Unix timestamp to a timestamp (PostgreSQL).
///
/// Returns TimestampTz type. The input should be a numeric Unix timestamp.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::to_timestamp;
///
/// // SELECT TO_TIMESTAMP(users.created_unix)
/// let ts = to_timestamp(users.created_unix);
/// ```
pub fn to_timestamp<'a, V, E>(expr: E) -> SQLExpr<'a, V, PgTimestamptz, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresDateTimeSupport,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("TO_TIMESTAMP", expr.into_sql()))
}
