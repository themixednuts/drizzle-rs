//! Type-safe math functions.
//!
//! These functions require `Numeric` types (SmallInt, Int, BigInt, Float, Double)
//! and provide compile-time enforcement of mathematical operations.
//!
//! # Type Safety
//!
//! - `abs`, `round`, `ceil`, `floor`: Require `Numeric` types
//! - `sqrt`, `power`, `log`, `exp`: Require `Numeric` types, return Double
//! - `mod_`: Modulo operation requiring `Numeric` types

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{DataType, Integral, Numeric};
use crate::{PostgresDialect, SQLiteDialect};
use drizzle_types::postgres::types::{Float4, Float8, Int2, Int4, Int8, Numeric as PgNumeric};
use drizzle_types::sqlite::types::{
    Integer as SqliteInteger, Numeric as SqliteNumeric, Real as SqliteReal,
};

use super::{AggOr, Expr, NullOr, Nullability, SQLExpr};

#[diagnostic::on_unimplemented(
    message = "no rounding policy for `{Self}` on this dialect",
    label = "round/ceil/floor/trunc return type is not defined for this SQL type/dialect"
)]
pub trait RoundingPolicy<D>: Numeric {
    type Output: DataType;
}

impl RoundingPolicy<SQLiteDialect> for SqliteInteger {
    type Output = SqliteReal;
}
impl RoundingPolicy<SQLiteDialect> for SqliteReal {
    type Output = SqliteReal;
}
impl RoundingPolicy<SQLiteDialect> for SqliteNumeric {
    type Output = SqliteReal;
}

impl RoundingPolicy<PostgresDialect> for Int2 {
    type Output = Float8;
}
impl RoundingPolicy<PostgresDialect> for Int4 {
    type Output = Float8;
}
impl RoundingPolicy<PostgresDialect> for Int8 {
    type Output = Float8;
}
impl RoundingPolicy<PostgresDialect> for Float4 {
    type Output = Float8;
}
impl RoundingPolicy<PostgresDialect> for Float8 {
    type Output = Float8;
}
impl RoundingPolicy<PostgresDialect> for PgNumeric {
    type Output = Float8;
}

// =============================================================================
// ABSOLUTE VALUE
// =============================================================================

/// ABS - returns the absolute value of a number.
///
/// Preserves the SQL type and nullability of the input expression.
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Int column
/// abs(users.balance);
///
/// // ❌ Compile error: Text is not Numeric
/// abs(users.name);
/// ```
pub fn abs<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("ABS", expr.into_sql()))
}

// =============================================================================
// ROUNDING FUNCTIONS
// =============================================================================

/// ROUND - rounds a number to the nearest integer (or specified precision).
///
/// Returns a dialect-aware float type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::round;
///
/// // SELECT ROUND(users.price)
/// let rounded = round(users.price);
/// ```
#[allow(clippy::type_complexity)]
pub fn round<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("ROUND", expr.into_sql()))
}

/// ROUND with precision - rounds a number to specified decimal places.
///
/// Returns a dialect-aware float type, preserves nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::round_to;
///
/// // SELECT ROUND(users.price, 2)
/// let rounded = round_to(users.price, 2);
/// ```
#[allow(clippy::type_complexity)]
pub fn round_to<'a, V, E, P>(
    expr: E,
    precision: P,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    <E::Nullable as NullOr<P::Nullable>>::Output,
    <E::Aggregate as AggOr<P::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
    P: Expr<'a, V>,
    P::SQLType: Integral,
    E::Nullable: NullOr<P::Nullable>,
    P::Nullable: Nullability,
    E::Aggregate: AggOr<P::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "ROUND",
        expr.into_sql()
            .push(Token::COMMA)
            .append(precision.into_sql()),
    ))
}

/// CEIL / CEILING - rounds a number up to the nearest integer.
///
/// Returns a dialect-aware float type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::ceil;
///
/// // SELECT CEIL(users.price)
/// let ceiling = ceil(users.price);
/// ```
#[allow(clippy::type_complexity)]
pub fn ceil<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("CEIL", expr.into_sql()))
}

/// FLOOR - rounds a number down to the nearest integer.
///
/// Returns a dialect-aware float type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::floor;
///
/// // SELECT FLOOR(users.price)
/// let floored = floor(users.price);
/// ```
#[allow(clippy::type_complexity)]
pub fn floor<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("FLOOR", expr.into_sql()))
}

/// TRUNC - truncates a number towards zero.
///
/// Returns a dialect-aware float type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::trunc;
///
/// // SELECT TRUNC(users.price)
/// let truncated = trunc(users.price);
/// ```
#[allow(clippy::type_complexity)]
pub fn trunc<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("TRUNC", expr.into_sql()))
}

// =============================================================================
// POWER AND ROOT FUNCTIONS
// =============================================================================

/// SQRT - returns the square root of a number.
///
/// Returns a dialect-aware double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::sqrt;
///
/// // SELECT SQRT(users.area)
/// let root = sqrt(users.area);
/// ```
pub fn sqrt<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("SQRT", expr.into_sql()))
}

/// POWER - raises a number to a power.
///
/// Returns a dialect-aware double type. The result is nullable if either input is nullable.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::power;
///
/// // SELECT POWER(users.base, 2)
/// let squared = power(users.base, 2);
/// ```
#[allow(clippy::type_complexity)]
pub fn power<'a, V, E1, E2>(
    base: E1,
    exponent: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <E1::Nullable as NullOr<E2::Nullable>>::Output,
    <E1::Aggregate as AggOr<E2::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
    E1::Aggregate: AggOr<E2::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "POWER",
        base.into_sql()
            .push(Token::COMMA)
            .append(exponent.into_sql()),
    ))
}

// =============================================================================
// LOGARITHMIC AND EXPONENTIAL FUNCTIONS
// =============================================================================

/// EXP - returns e raised to the power of the argument.
///
/// Returns a dialect-aware double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::exp;
///
/// // SELECT EXP(users.rate)
/// let exponential = exp(users.rate);
/// ```
pub fn exp<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("EXP", expr.into_sql()))
}

/// LN - returns the natural logarithm of a number.
///
/// Returns a dialect-aware double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::ln;
///
/// // SELECT LN(users.value)
/// let natural_log = ln(users.value);
/// ```
pub fn ln<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("LN", expr.into_sql()))
}

/// LOG10 - returns the base-10 logarithm of a number.
///
/// Returns a dialect-aware double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::log10;
///
/// // SELECT LOG10(users.value)
/// let log_base_10 = log10(users.value);
/// ```
pub fn log10<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("LOG10", expr.into_sql()))
}

/// LOG - returns the logarithm of a number with a specified base.
///
/// Returns a dialect-aware double type. The result is nullable if either input is nullable.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::log;
///
/// // SELECT LOG(2, users.value)
/// let log_base_2 = log(2, users.value);
/// ```
#[allow(clippy::type_complexity)]
pub fn log<'a, V, E1, E2>(
    base: E1,
    value: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <E1::Nullable as NullOr<E2::Nullable>>::Output,
    <E1::Aggregate as AggOr<E2::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
    E1::Aggregate: AggOr<E2::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "LOG",
        base.into_sql().push(Token::COMMA).append(value.into_sql()),
    ))
}

// =============================================================================
// SIGN AND MODULO
// =============================================================================

/// SIGN - returns the sign of a number (-1, 0, or 1).
///
/// Returns the same numeric type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::sign;
///
/// // SELECT SIGN(users.balance)
/// let balance_sign = sign(users.balance);
/// ```
pub fn sign<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("SIGN", expr.into_sql()))
}

/// MOD - returns the remainder of division (using % operator).
///
/// Returns the same type as the dividend. The result is nullable if either input is nullable.
/// Named `mod_` to avoid conflict with Rust's `mod` keyword.
///
/// Note: Uses the `%` operator which works on both SQLite and PostgreSQL.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::mod_;
///
/// // SELECT users.value % 3
/// let remainder = mod_(users.value, 3);
/// ```
#[allow(clippy::type_complexity)]
pub fn mod_<'a, V, E1, E2>(
    dividend: E1,
    divisor: E2,
) -> SQLExpr<
    'a,
    V,
    E1::SQLType,
    <E1::Nullable as NullOr<E2::Nullable>>::Output,
    <E1::Aggregate as AggOr<E2::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
    E1::Aggregate: AggOr<E2::Aggregate>,
{
    SQLExpr::new(
        dividend
            .into_sql()
            .push(Token::REM)
            .append(divisor.into_sql()),
    )
}
