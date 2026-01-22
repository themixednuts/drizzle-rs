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

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{Double, Numeric};

use super::{Expr, NullOr, Nullability, SQLExpr, Scalar};

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
pub fn abs<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("ABS", expr.to_sql()))
}

// =============================================================================
// ROUNDING FUNCTIONS
// =============================================================================

/// ROUND - rounds a number to the nearest integer (or specified precision).
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::round;
///
/// // SELECT ROUND(users.price)
/// let rounded = round(users.price);
/// ```
pub fn round<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("ROUND", expr.to_sql()))
}

/// ROUND with precision - rounds a number to specified decimal places.
///
/// Returns Double type, preserves nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::round_to;
///
/// // SELECT ROUND(users.price, 2)
/// let rounded = round_to(users.price, 2);
/// ```
pub fn round_to<'a, V, E, P>(expr: E, precision: P) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
    P: Expr<'a, V>,
{
    SQLExpr::new(SQL::func(
        "ROUND",
        expr.to_sql()
            .push(Token::COMMA)
            .append(precision.to_sql()),
    ))
}

/// CEIL / CEILING - rounds a number up to the nearest integer.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::ceil;
///
/// // SELECT CEIL(users.price)
/// let ceiling = ceil(users.price);
/// ```
pub fn ceil<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("CEIL", expr.to_sql()))
}

/// FLOOR - rounds a number down to the nearest integer.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::floor;
///
/// // SELECT FLOOR(users.price)
/// let floored = floor(users.price);
/// ```
pub fn floor<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("FLOOR", expr.to_sql()))
}

/// TRUNC - truncates a number towards zero.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::trunc;
///
/// // SELECT TRUNC(users.price)
/// let truncated = trunc(users.price);
/// ```
pub fn trunc<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("TRUNC", expr.to_sql()))
}

// =============================================================================
// POWER AND ROOT FUNCTIONS
// =============================================================================

/// SQRT - returns the square root of a number.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::sqrt;
///
/// // SELECT SQRT(users.area)
/// let root = sqrt(users.area);
/// ```
pub fn sqrt<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("SQRT", expr.to_sql()))
}

/// POWER - raises a number to a power.
///
/// Returns Double type. The result is nullable if either input is nullable.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::power;
///
/// // SELECT POWER(users.base, 2)
/// let squared = power(users.base, 2);
/// ```
pub fn power<'a, V, E1, E2>(
    base: E1,
    exponent: E2,
) -> SQLExpr<'a, V, Double, <E1::Nullable as NullOr<E2::Nullable>>::Output, Scalar>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
{
    SQLExpr::new(SQL::func(
        "POWER",
        base.to_sql()
            .push(Token::COMMA)
            .append(exponent.to_sql()),
    ))
}

// =============================================================================
// LOGARITHMIC AND EXPONENTIAL FUNCTIONS
// =============================================================================

/// EXP - returns e raised to the power of the argument.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::exp;
///
/// // SELECT EXP(users.rate)
/// let exponential = exp(users.rate);
/// ```
pub fn exp<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("EXP", expr.to_sql()))
}

/// LN - returns the natural logarithm of a number.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::ln;
///
/// // SELECT LN(users.value)
/// let natural_log = ln(users.value);
/// ```
pub fn ln<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("LN", expr.to_sql()))
}

/// LOG10 - returns the base-10 logarithm of a number.
///
/// Returns Double type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::log10;
///
/// // SELECT LOG10(users.value)
/// let log_base_10 = log10(users.value);
/// ```
pub fn log10<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("LOG10", expr.to_sql()))
}

/// LOG - returns the logarithm of a number with a specified base.
///
/// Returns Double type. The result is nullable if either input is nullable.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::log;
///
/// // SELECT LOG(2, users.value)
/// let log_base_2 = log(2, users.value);
/// ```
pub fn log<'a, V, E1, E2>(
    base: E1,
    value: E2,
) -> SQLExpr<'a, V, Double, <E1::Nullable as NullOr<E2::Nullable>>::Output, Scalar>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
{
    SQLExpr::new(SQL::func(
        "LOG",
        base.to_sql().push(Token::COMMA).append(value.to_sql()),
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
pub fn sign<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("SIGN", expr.to_sql()))
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
) -> SQLExpr<'a, V, E1::SQLType, <E1::Nullable as NullOr<E2::Nullable>>::Output, Scalar>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
{
    SQLExpr::new(
        dividend
            .to_sql()
            .push(Token::REM)
            .append(divisor.to_sql()),
    )
}
