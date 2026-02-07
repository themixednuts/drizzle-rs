//! Type-safe comparison functions.
//!
//! This module provides both function-based and method-based comparisons:
//!
//! ```ignore
//! // Function style
//! eq(users.id, 42)
//! gt(users.age, 18)
//!
//! // Method style (on SQLExpr)
//! users.id.eq(42)
//! users.age.gt(18)
//! ```
//!
//! # Type Safety
//!
//! - `eq`, `neq`, `gt`, `gte`, `lt`, `lte`: Require compatible types
//! - `like`, `not_like`: Require textual types on both sides
//! - `between`: Requires expr compatible with both bounds
//! - `is_null`, `is_not_null`: No type constraint (any type can be null-checked)

use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Bool, Compatible, Textual};

use super::{Expr, NonNull, SQLExpr, Scalar};

// =============================================================================
// Internal Helper
// =============================================================================

fn binary_op<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    let right_sql = right.into_sql();
    // Wrap subqueries (starting with SELECT) in parentheses
    let right_sql = if right_sql.is_subquery() {
        right_sql.parens()
    } else {
        right_sql
    };
    left.into_sql().push(operator).append(right_sql)
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
    SQLExpr::new(left.into_sql().push(Token::LIKE).append(pattern.into_sql()))
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
        left.into_sql()
            .push(Token::NOT)
            .push(Token::LIKE)
            .append(pattern.into_sql()),
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
            .append(expr.into_sql())
            .push(Token::BETWEEN)
            .append(low.into_sql())
            .push(Token::AND)
            .append(high.into_sql())
            .push(Token::RPAREN),
    )
}

/// NOT BETWEEN comparison.
///
/// Requires expr type to be compatible with both bounds.
pub fn not_between<'a, V, E, L, H>(
    expr: E,
    low: L,
    high: H,
) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: Expr<'a, V>,
    H: Expr<'a, V>,
    E::SQLType: Compatible<L::SQLType> + Compatible<H::SQLType>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(expr.into_sql())
            .push(Token::NOT)
            .push(Token::BETWEEN)
            .append(low.into_sql())
            .push(Token::AND)
            .append(high.into_sql())
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
    SQLExpr::new(expr.into_sql().push(Token::IS).push(Token::NULL))
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
        expr.into_sql()
            .push(Token::IS)
            .push(Token::NOT)
            .push(Token::NULL),
    )
}

// =============================================================================
// Method-based Comparison API (Extension Trait)
// =============================================================================

/// Extension trait providing method-based comparisons for any `Expr` type.
///
/// This trait is blanket-implemented for all types implementing `Expr`,
/// allowing method syntax on columns, literals, and expressions:
///
/// ```ignore
/// // Works on columns directly
/// users.id.eq(42)
/// users.age.gt(18)
///
/// // Chain with operators
/// users.id.eq(42) & users.age.gt(18)
/// ```
pub trait ExprExt<'a, V: SQLParam>: Expr<'a, V> + Sized {
    /// Equality comparison (`=`).
    ///
    /// ```ignore
    /// users.id.eq(42)  // "users"."id" = 42
    /// ```
    fn eq<R>(self, other: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        eq(self, other)
    }

    /// Inequality comparison (`<>`).
    ///
    /// ```ignore
    /// users.id.ne(42)  // "users"."id" <> 42
    /// ```
    fn ne<R>(self, other: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        neq(self, other)
    }

    /// Greater-than comparison (`>`).
    ///
    /// ```ignore
    /// users.age.gt(18)  // "users"."age" > 18
    /// ```
    fn gt<R>(self, other: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        gt(self, other)
    }

    /// Greater-than-or-equal comparison (`>=`).
    ///
    /// ```ignore
    /// users.age.ge(18)  // "users"."age" >= 18
    /// ```
    fn ge<R>(self, other: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        gte(self, other)
    }

    /// Less-than comparison (`<`).
    ///
    /// ```ignore
    /// users.age.lt(65)  // "users"."age" < 65
    /// ```
    fn lt<R>(self, other: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        lt(self, other)
    }

    /// Less-than-or-equal comparison (`<=`).
    ///
    /// ```ignore
    /// users.age.le(65)  // "users"."age" <= 65
    /// ```
    fn le<R>(self, other: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        lte(self, other)
    }

    /// LIKE pattern matching.
    ///
    /// ```ignore
    /// users.name.like("%Alice%")  // "users"."name" LIKE '%Alice%'
    /// ```
    fn like<R>(self, pattern: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Textual,
        R::SQLType: Textual,
    {
        like(self, pattern)
    }

    /// NOT LIKE pattern matching.
    ///
    /// ```ignore
    /// users.name.not_like("%Bot%")  // "users"."name" NOT LIKE '%Bot%'
    /// ```
    fn not_like<R>(self, pattern: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        R: Expr<'a, V>,
        Self::SQLType: Textual,
        R::SQLType: Textual,
    {
        not_like(self, pattern)
    }

    /// IS NULL check.
    ///
    /// ```ignore
    /// users.deleted_at.is_null()  // "users"."deleted_at" IS NULL
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_null(self) -> SQLExpr<'a, V, Bool, NonNull, Scalar> {
        is_null(self)
    }

    /// IS NOT NULL check.
    ///
    /// ```ignore
    /// users.email.is_not_null()  // "users"."email" IS NOT NULL
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_not_null(self) -> SQLExpr<'a, V, Bool, NonNull, Scalar> {
        is_not_null(self)
    }

    /// BETWEEN comparison.
    ///
    /// Checks if the value is between low and high (inclusive).
    ///
    /// ```ignore
    /// users.age.between(18, 65)  // ("users"."age" BETWEEN 18 AND 65)
    /// ```
    fn between<L, H>(self, low: L, high: H) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        L: Expr<'a, V>,
        H: Expr<'a, V>,
        Self::SQLType: Compatible<L::SQLType> + Compatible<H::SQLType>,
    {
        between(self, low, high)
    }

    /// NOT BETWEEN comparison.
    ///
    /// Checks if the value is NOT between low and high.
    ///
    /// ```ignore
    /// users.age.not_between(0, 17)  // ("users"."age" NOT BETWEEN 0 AND 17)
    /// ```
    fn not_between<L, H>(self, low: L, high: H) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        L: Expr<'a, V>,
        H: Expr<'a, V>,
        Self::SQLType: Compatible<L::SQLType> + Compatible<H::SQLType>,
    {
        not_between(self, low, high)
    }

    /// IN array check.
    ///
    /// Checks if the value is in the provided array.
    ///
    /// ```ignore
    /// users.role.in_array([Role::Admin, Role::Moderator])
    /// // "users"."role" IN ('admin', 'moderator')
    /// ```
    fn in_array<I, R>(self, values: I) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        I: IntoIterator<Item = R>,
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        crate::expr::in_array(self, values)
    }

    /// NOT IN array check.
    ///
    /// Checks if the value is NOT in the provided array.
    ///
    /// ```ignore
    /// users.role.not_in_array([Role::Banned, Role::Suspended])
    /// // "users"."role" NOT IN ('banned', 'suspended')
    /// ```
    fn not_in_array<I, R>(self, values: I) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
    where
        I: IntoIterator<Item = R>,
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        crate::expr::not_in_array(self, values)
    }
}

/// Blanket implementation for all `Expr` types.
impl<'a, V: SQLParam, E: Expr<'a, V>> ExprExt<'a, V> for E {}
