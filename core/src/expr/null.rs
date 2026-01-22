//! NULL propagation and handling.
//!
//! This module provides traits and functions for handling SQL NULL values
//! in a type-safe manner.
//!
//! # Type Safety
//!
//! - `coalesce`, `ifnull`: Require compatible types between expression and default
//! - `nullif`: Requires compatible types between the two arguments

use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::Compatible;

use super::{Expr, NonNull, Null, Nullability, SQLExpr, Scalar};

// =============================================================================
// Nullability Combination
// =============================================================================

/// Combine nullability: if either input is nullable, output is nullable.
///
/// This follows SQL's NULL propagation semantics where operations on
/// NULL values produce NULL results.
///
/// # Truth Table
///
/// | Left | Right | Output |
/// |------|-------|--------|
/// | NonNull | NonNull | NonNull |
/// | NonNull | Null | Null |
/// | Null | NonNull | Null |
/// | Null | Null | Null |
pub trait NullOr<Rhs: Nullability>: Nullability {
    /// The resulting nullability.
    type Output: Nullability;
}

impl NullOr<NonNull> for NonNull {
    type Output = NonNull;
}
impl NullOr<Null> for NonNull {
    type Output = Null;
}
impl NullOr<NonNull> for Null {
    type Output = Null;
}
impl NullOr<Null> for Null {
    type Output = Null;
}

// =============================================================================
// COALESCE Function
// =============================================================================

/// COALESCE - returns first non-null value.
///
/// Requires compatible types between the expression and default.
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Both are Text
/// coalesce(users.nickname, users.name);
///
/// // ✅ OK: Int with i32 literal
/// coalesce(users.age, 0);
///
/// // ❌ Compile error: Int not compatible with Text
/// coalesce(users.age, "unknown");
/// ```
pub fn coalesce<'a, V, E, D>(expr: E, default: D) -> SQLExpr<'a, V, E::SQLType, Null, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
{
    SQLExpr::new(SQL::func(
        "COALESCE",
        expr.to_sql().push(Token::COMMA).append(default.to_sql()),
    ))
}

/// COALESCE with multiple values.
///
/// Returns the first non-null value from the provided expressions.
/// Takes an explicit first argument to guarantee at least one value at compile time.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::coalesce_many;
///
/// // COALESCE(users.nickname, users.username, 'Anonymous')
/// let name = coalesce_many(users.nickname, [users.username, "Anonymous"]);
/// ```
pub fn coalesce_many<'a, V, E, I>(first: E, rest: I) -> SQLExpr<'a, V, E::SQLType, Null, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator,
    I::Item: Expr<'a, V>,
{
    let mut sql = first.to_sql();
    for value in rest {
        sql = sql.push(Token::COMMA).append(value.to_sql());
    }
    SQLExpr::new(SQL::func("COALESCE", sql))
}

// =============================================================================
// NULLIF Function
// =============================================================================

/// NULLIF - returns NULL if arguments are equal, else first argument.
///
/// Requires compatible types between the two arguments.
/// The result is always nullable since it can return NULL.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::nullif;
///
/// // Returns NULL if status is 'unknown', otherwise returns status
/// let status = nullif(item.status, "unknown");
/// ```
pub fn nullif<'a, V, E1, E2>(expr1: E1, expr2: E2) -> SQLExpr<'a, V, E1::SQLType, Null, Scalar>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E2: Expr<'a, V>,
    E1::SQLType: Compatible<E2::SQLType>,
{
    SQLExpr::new(SQL::func(
        "NULLIF",
        expr1.to_sql().push(Token::COMMA).append(expr2.to_sql()),
    ))
}

// =============================================================================
// IFNULL / NVL Function
// =============================================================================

/// IFNULL - SQLite/MySQL equivalent of COALESCE with two arguments.
///
/// Requires compatible types between the expression and default.
/// Returns the first argument if not NULL, otherwise returns the second.
pub fn ifnull<'a, V, E, D>(expr: E, default: D) -> SQLExpr<'a, V, E::SQLType, Null, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
{
    SQLExpr::new(SQL::func(
        "IFNULL",
        expr.to_sql().push(Token::COMMA).append(default.to_sql()),
    ))
}
