//! Type-safe comparison functions.
//!
//! These functions enforce type compatibility at compile time using the
//! `Expr` trait and `Compatible` constraint. Comparing incompatible types
//! (e.g., `eq(int_column, "text")`) will fail at compile time.
//!
//! # Type Safety
//!
//! - `eq`, `neq`, `gt`, `gte`, `lt`, `lte`: Require compatible types
//! - `like`, `not_like`: Require textual types on both sides
//! - `between`: Requires expr compatible with both bounds
//! - `is_null`, `is_not_null`: No type constraint (any type can be null-checked)

use crate::sql::{Token, SQL};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Bool, Compatible, Textual};

use super::{Expr, NonNull, Scalar, SQLExpr};

// =============================================================================
// Internal Helper
// =============================================================================

fn binary_op<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    let right_sql = right.to_sql();
    // Wrap subqueries (starting with SELECT) in parentheses
    let right_sql = if right_sql.is_subquery() {
        right_sql.parens()
    } else {
        right_sql
    };
    left.to_sql().push(operator).append(right_sql)
}

// =============================================================================
// Equality Comparisons
// =============================================================================

/// Equality comparison (`=`).
///
/// Requires both operands to have compatible SQL types.
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Int compared with i32
/// eq(users.id, 10);
///
/// // ✅ OK: Int compared with BigInt (integer family)
/// eq(users.id, users.big_id);
///
/// // ❌ Compile error: Int cannot be compared with Text
/// eq(users.id, "hello");
/// ```
pub fn eq<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::EQ, right))
}

/// Inequality comparison (`<>` or `!=`).
///
/// Requires both operands to have compatible SQL types.
pub fn neq<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::NE, right))
}

// =============================================================================
// Ordering Comparisons
// =============================================================================

/// Greater-than comparison (`>`).
///
/// Requires both operands to have compatible SQL types.
pub fn gt<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::GT, right))
}

/// Greater-than-or-equal comparison (`>=`).
///
/// Requires both operands to have compatible SQL types.
pub fn gte<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::GE, right))
}

/// Less-than comparison (`<`).
///
/// Requires both operands to have compatible SQL types.
pub fn lt<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::LT, right))
}

/// Less-than-or-equal comparison (`<=`).
///
/// Requires both operands to have compatible SQL types.
pub fn lte<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::LE, right))
}

// =============================================================================
// Pattern Matching
// =============================================================================

/// LIKE pattern matching.
///
/// Requires both operands to be textual types (TEXT, VARCHAR).
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Text column with text pattern
/// like(users.name, "%Alice%");
///
/// // ❌ Compile error: Int is not Textual
/// like(users.id, "%123%");
/// ```
pub fn like<'a, V, L, R>(left: L, pattern: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Textual,
    R::SQLType: Textual,
{
    SQLExpr::new(left.to_sql().push(Token::LIKE).append(pattern.to_sql()))
}

/// NOT LIKE pattern matching.
///
/// Requires both operands to be textual types (TEXT, VARCHAR).
pub fn not_like<'a, V, L, R>(left: L, pattern: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Textual,
    R::SQLType: Textual,
{
    SQLExpr::new(
        left.to_sql()
            .push(Token::NOT)
            .push(Token::LIKE)
            .append(pattern.to_sql()),
    )
}

// =============================================================================
// Range Comparisons
// =============================================================================

/// BETWEEN comparison.
///
/// Checks if expr is between low and high (inclusive).
/// Requires expr type to be compatible with both bounds.
pub fn between<'a, V, E, L, H>(expr: E, low: L, high: H) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: Expr<'a, V>,
    H: Expr<'a, V>,
    E::SQLType: Compatible<L::SQLType> + Compatible<H::SQLType>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(expr.to_sql())
            .push(Token::BETWEEN)
            .append(low.to_sql())
            .push(Token::AND)
            .append(high.to_sql())
            .push(Token::RPAREN),
    )
}

/// NOT BETWEEN comparison.
///
/// Requires expr type to be compatible with both bounds.
pub fn not_between<'a, V, E, L, H>(expr: E, low: L, high: H) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: Expr<'a, V>,
    H: Expr<'a, V>,
    E::SQLType: Compatible<L::SQLType> + Compatible<H::SQLType>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(expr.to_sql())
            .push(Token::NOT)
            .push(Token::BETWEEN)
            .append(low.to_sql())
            .push(Token::AND)
            .append(high.to_sql())
            .push(Token::RPAREN),
    )
}

// =============================================================================
// NULL Checks
// =============================================================================

/// IS NULL check.
///
/// Returns a boolean expression checking if the value is NULL.
/// Any expression type can be null-checked.
pub fn is_null<'a, V, E>(expr: E) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(expr.to_sql().push(Token::IS).push(Token::NULL))
}

/// IS NOT NULL check.
///
/// Returns a boolean expression checking if the value is not NULL.
/// Any expression type can be null-checked.
pub fn is_not_null<'a, V, E>(expr: E) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(Token::IS)
            .push(Token::NOT)
            .push(Token::NULL),
    )
}
