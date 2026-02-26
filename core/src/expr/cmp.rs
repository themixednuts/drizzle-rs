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

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Compatible, DataType, Textual};

use super::{AggOr, AggregateKind, Expr, NonNull, SQLExpr};

// =============================================================================
// Internal Helper
// =============================================================================

fn binary_op<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    let left_sql = operand_sql(left);
    let right_sql = operand_sql(right);

    left_sql.push(operator).append(right_sql)
}

#[inline]
fn operand_sql<'a, V, T>(value: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    value.into_sql().parens_if_subquery()
}

/// Operand accepted by comparison helpers.
///
/// Enforces `Expected: Compatible<Rhs::SQLType>` for any expression-like operand.
pub trait ComparisonOperand<'a, V, Expected>: ToSQL<'a, V>
where
    V: SQLParam + 'a,
    Expected: DataType,
{
    type SQLType: DataType;
    type Aggregate: AggregateKind;
}

impl<'a, V, Expected, R> ComparisonOperand<'a, V, Expected> for R
where
    V: SQLParam + 'a,
    Expected: DataType + Compatible<R::SQLType>,
    R: Expr<'a, V>,
{
    type SQLType = R::SQLType;
    type Aggregate = R::Aggregate;
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
#[allow(clippy::type_complexity)]
pub fn eq<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(binary_op(left, Token::EQ, right))
}

/// Inequality comparison (`<>` or `!=`).
///
/// Requires both operands to have compatible SQL types.
#[allow(clippy::type_complexity)]
pub fn neq<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(binary_op(left, Token::NE, right))
}

// =============================================================================
// Ordering Comparisons
// =============================================================================

/// Greater-than comparison (`>`).
///
/// Requires both operands to have compatible SQL types.
#[allow(clippy::type_complexity)]
pub fn gt<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(binary_op(left, Token::GT, right))
}

/// Greater-than-or-equal comparison (`>=`).
///
/// Requires both operands to have compatible SQL types.
#[allow(clippy::type_complexity)]
pub fn gte<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(binary_op(left, Token::GE, right))
}

/// Less-than comparison (`<`).
///
/// Requires both operands to have compatible SQL types.
#[allow(clippy::type_complexity)]
pub fn lt<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(binary_op(left, Token::LT, right))
}

/// Less-than-or-equal comparison (`<=`).
///
/// Requires both operands to have compatible SQL types.
#[allow(clippy::type_complexity)]
pub fn lte<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
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
#[allow(clippy::type_complexity)]
pub fn like<'a, V, L, R>(
    left: L,
    pattern: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::SQLType: Textual,
    <R as ComparisonOperand<'a, V, L::SQLType>>::SQLType: Textual,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(
        operand_sql(left)
            .push(Token::LIKE)
            .append(operand_sql(pattern)),
    )
}

/// NOT LIKE pattern matching.
///
/// Requires both operands to be textual types (TEXT, VARCHAR).
#[allow(clippy::type_complexity)]
pub fn not_like<'a, V, L, R>(
    left: L,
    pattern: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L::SQLType>,
    L::SQLType: Textual,
    <R as ComparisonOperand<'a, V, L::SQLType>>::SQLType: Textual,
    L::Aggregate: AggOr<<R as ComparisonOperand<'a, V, L::SQLType>>::Aggregate>,
{
    SQLExpr::new(
        operand_sql(left)
            .push(Token::NOT)
            .push(Token::LIKE)
            .append(operand_sql(pattern)),
    )
}

// =============================================================================
// Range Comparisons
// =============================================================================

/// BETWEEN comparison.
///
/// Checks if expr is between low and high (inclusive).
/// Requires expr type to be compatible with both bounds.
#[allow(clippy::type_complexity)]
pub fn between<'a, V, E, L, H>(
    expr: E,
    low: L,
    high: H,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <<E::Aggregate as AggOr<<L as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>>::Output as AggOr<<H as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: ComparisonOperand<'a, V, E::SQLType>,
    H: ComparisonOperand<'a, V, E::SQLType>,
    E::Aggregate: AggOr<<L as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>,
    <E::Aggregate as AggOr<<L as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>>::Output:
        AggOr<<H as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(operand_sql(expr))
            .push(Token::BETWEEN)
            .append(operand_sql(low))
            .push(Token::AND)
            .append(operand_sql(high))
            .push(Token::RPAREN),
    )
}

/// NOT BETWEEN comparison.
///
/// Requires expr type to be compatible with both bounds.
#[allow(clippy::type_complexity)]
pub fn not_between<'a, V, E, L, H>(
    expr: E,
    low: L,
    high: H,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <<E::Aggregate as AggOr<<L as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>>::Output as AggOr<<H as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: ComparisonOperand<'a, V, E::SQLType>,
    H: ComparisonOperand<'a, V, E::SQLType>,
    E::Aggregate: AggOr<<L as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>,
    <E::Aggregate as AggOr<<L as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>>::Output:
        AggOr<<H as ComparisonOperand<'a, V, E::SQLType>>::Aggregate>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(operand_sql(expr))
            .push(Token::NOT)
            .push(Token::BETWEEN)
            .append(operand_sql(low))
            .push(Token::AND)
            .append(operand_sql(high))
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
pub fn is_null<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(operand_sql(expr).push(Token::IS).push(Token::NULL))
}

/// IS NOT NULL check.
///
/// Returns a boolean expression checking if the value is not NULL.
/// Any expression type can be null-checked.
pub fn is_not_null<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(
        operand_sql(expr)
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
    #[allow(clippy::type_complexity)]
    fn eq<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        eq(self, other)
    }

    /// Inequality comparison (`<>`).
    ///
    /// ```ignore
    /// users.id.ne(42)  // "users"."id" <> 42
    /// ```
    #[allow(clippy::type_complexity)]
    fn ne<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        neq(self, other)
    }

    /// Greater-than comparison (`>`).
    ///
    /// ```ignore
    /// users.age.gt(18)  // "users"."age" > 18
    /// ```
    #[allow(clippy::type_complexity)]
    fn gt<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        gt(self, other)
    }

    /// Greater-than-or-equal comparison (`>=`).
    ///
    /// ```ignore
    /// users.age.ge(18)  // "users"."age" >= 18
    /// ```
    #[allow(clippy::type_complexity)]
    fn ge<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        gte(self, other)
    }

    /// Less-than comparison (`<`).
    ///
    /// ```ignore
    /// users.age.lt(65)  // "users"."age" < 65
    /// ```
    #[allow(clippy::type_complexity)]
    fn lt<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        lt(self, other)
    }

    /// Less-than-or-equal comparison (`<=`).
    ///
    /// ```ignore
    /// users.age.le(65)  // "users"."age" <= 65
    /// ```
    #[allow(clippy::type_complexity)]
    fn le<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        lte(self, other)
    }

    /// LIKE pattern matching.
    ///
    /// ```ignore
    /// users.name.like("%Alice%")  // "users"."name" LIKE '%Alice%'
    /// ```
    #[allow(clippy::type_complexity)]
    fn like<R>(
        self,
        pattern: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::SQLType: Textual,
        <R as ComparisonOperand<'a, V, Self::SQLType>>::SQLType: Textual,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        like(self, pattern)
    }

    /// NOT LIKE pattern matching.
    ///
    /// ```ignore
    /// users.name.not_like("%Bot%")  // "users"."name" NOT LIKE '%Bot%'
    /// ```
    #[allow(clippy::type_complexity)]
    fn not_like<R>(
        self,
        pattern: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggOr<
            <R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        R: ComparisonOperand<'a, V, Self::SQLType>,
        Self::SQLType: Textual,
        <R as ComparisonOperand<'a, V, Self::SQLType>>::SQLType: Textual,
        Self::Aggregate:
            AggOr<<R as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
    {
        not_like(self, pattern)
    }

    /// IS NULL check.
    ///
    /// ```ignore
    /// users.deleted_at.is_null()  // "users"."deleted_at" IS NULL
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_null(
        self,
    ) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Self::Aggregate> {
        is_null(self)
    }

    /// IS NOT NULL check.
    ///
    /// ```ignore
    /// users.email.is_not_null()  // "users"."email" IS NOT NULL
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_not_null(
        self,
    ) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Self::Aggregate> {
        is_not_null(self)
    }

    /// BETWEEN comparison.
    ///
    /// Checks if the value is between low and high (inclusive).
    ///
    /// ```ignore
    /// users.age.between(18, 65)  // ("users"."age" BETWEEN 18 AND 65)
    /// ```
    #[allow(clippy::type_complexity)]
    fn between<L, H>(
        self,
        low: L,
        high: H,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <<Self::Aggregate as AggOr<
            <L as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output as AggOr<
            <H as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        L: ComparisonOperand<'a, V, Self::SQLType>,
        H: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<L as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
        <Self::Aggregate as AggOr<
            <L as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output:
            AggOr<<H as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
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
    #[allow(clippy::type_complexity)]
    fn not_between<L, H>(
        self,
        low: L,
        high: H,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <<Self::Aggregate as AggOr<
            <L as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output as AggOr<
            <H as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output,
    >
    where
        L: ComparisonOperand<'a, V, Self::SQLType>,
        H: ComparisonOperand<'a, V, Self::SQLType>,
        Self::Aggregate:
            AggOr<<L as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
        <Self::Aggregate as AggOr<
            <L as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate,
        >>::Output:
            AggOr<<H as ComparisonOperand<'a, V, Self::SQLType>>::Aggregate>,
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
    fn in_array<I, R>(
        self,
        values: I,
    ) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Self::Aggregate>
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
    fn not_in_array<I, R>(
        self,
        values: I,
    ) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Self::Aggregate>
    where
        I: IntoIterator<Item = R>,
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        crate::expr::not_in_array(self, values)
    }

    /// IN subquery check.
    fn in_subquery<S>(
        self,
        subquery: S,
    ) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Self::Aggregate>
    where
        S: Expr<'a, V>,
        Self::SQLType: Compatible<S::SQLType>,
    {
        crate::expr::in_subquery(self, subquery)
    }

    /// NOT IN subquery check.
    fn not_in_subquery<S>(
        self,
        subquery: S,
    ) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Self::Aggregate>
    where
        S: Expr<'a, V>,
        Self::SQLType: Compatible<S::SQLType>,
    {
        crate::expr::not_in_subquery(self, subquery)
    }
}

/// Blanket implementation for all `Expr` types.
impl<'a, V: SQLParam, E: Expr<'a, V>> ExprExt<'a, V> for E {}
