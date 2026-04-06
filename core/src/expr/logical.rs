//! Logical operators (AND, OR, NOT).
//!
//! This module provides both function-based and operator-based logical operations:
//!
//! ```rust
//! # let _ = r####"
//! // Function style
//! and(condition1, condition2)
//! or(condition1, condition2)
//! not(condition)
//!
//! // Operator style (via std::ops traits)
//! condition1 & condition2   // BitAnd
//! condition1 | condition2   // BitOr
//! !condition                 // Not
//!
//! // Multiple conditions (iterator)
//! and_all([condition1, condition2, condition3])
//! or_all([condition1, condition2, condition3])
//! # "####;
//! ```

use core::ops::{BitAnd, BitOr, Not};

use crate::dialect::DialectTypes;
use crate::sql::{SQL, SQLChunk, Token};
use crate::traits::SQLParam;
use crate::types::BooleanLike;

use super::{AggOr, AggregateKind, Expr, NullOr, Nullability, SQLExpr};

#[inline]
fn operand_sql<'a, V, E>(value: E) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: BooleanLike,
{
    value.into_sql().parens_if_subquery()
}

#[inline]
fn binary_logical_op<'a, V, L, R>(left: L, token: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    L::SQLType: BooleanLike,
    R: Expr<'a, V>,
    R::SQLType: BooleanLike,
{
    SQL::from(Token::LPAREN)
        .append(operand_sql(left))
        .push(token)
        .append(operand_sql(right))
        .push(Token::RPAREN)
}

// =============================================================================
// NOT
// =============================================================================

/// Logical NOT.
///
/// Negates a boolean expression.
pub fn not<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: BooleanLike,
    E::Nullable: Nullability,
{
    let expr_sql: SQL<'a, V> = expr.into_sql().parens_if_subquery();
    let needs_paren = expr_sql.chunks.len() > 1
        || (expr_sql.chunks.len() == 1
            && !matches!(
                expr_sql.chunks[0],
                SQLChunk::Raw(_) | SQLChunk::Ident(_) | SQLChunk::Number(_)
            ));

    let sql = if needs_paren {
        SQL::from_iter([Token::NOT, Token::LPAREN])
            .append(expr_sql)
            .push(Token::RPAREN)
    } else {
        SQL::from(Token::NOT).append(expr_sql)
    };
    SQLExpr::new(sql)
}

// =============================================================================
// AND
// =============================================================================

/// Logical AND of two conditions.
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::{and, eq, gt};
///
/// and(eq(users.active, true), gt(users.age, 18))
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn and<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    <L::Nullable as NullOr<R::Nullable>>::Output,
    <L::Aggregate as AggOr<R::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    L::SQLType: BooleanLike,
    L::Nullable: NullOr<R::Nullable>,
    L::Aggregate: AggOr<R::Aggregate>,
    R: Expr<'a, V>,
    R::SQLType: BooleanLike,
    R::Nullable: Nullability,
{
    SQLExpr::new(binary_logical_op(left, Token::AND, right))
}

// =============================================================================
// OR
// =============================================================================

/// Logical OR of two conditions.
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::{or, eq};
///
/// or(eq(users.role, "admin"), eq(users.role, "moderator"))
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn or<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    <L::Nullable as NullOr<R::Nullable>>::Output,
    <L::Aggregate as AggOr<R::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    L::SQLType: BooleanLike,
    L::Nullable: NullOr<R::Nullable>,
    L::Aggregate: AggOr<R::Aggregate>,
    R: Expr<'a, V>,
    R::SQLType: BooleanLike,
    R::Nullable: Nullability,
{
    SQLExpr::new(binary_logical_op(left, Token::OR, right))
}

// =============================================================================
// Operator Trait Implementations
// =============================================================================

/// Implements `!expr` for boolean expressions (SQL NOT).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// let condition = eq(users.active, true);
/// let negated = !condition;  // NOT "users"."active" = TRUE
/// # "####;
/// ```
impl<'a, V, T, N, A> Not for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: BooleanLike,
    N: Nullability,
    A: AggregateKind,
{
    type Output = SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, N, A>;

    fn not(self) -> Self::Output {
        not(self)
    }
}

/// Implements `expr1 & expr2` for boolean expressions (SQL AND).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// let condition = eq(users.active, true) & gt(users.age, 18);
/// // ("users"."active" = TRUE AND "users"."age" > 18)
/// # "####;
/// ```
impl<'a, V, T, N, A, Rhs> BitAnd<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: BooleanLike,
    N: Nullability + NullOr<Rhs::Nullable>,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: BooleanLike,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        <N as NullOr<Rhs::Nullable>>::Output,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        and(self, rhs)
    }
}

/// Implements `expr1 | expr2` for boolean expressions (SQL OR).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// let condition = eq(users.role, "admin") | eq(users.role, "moderator");
/// // ("users"."role" = 'admin' OR "users"."role" = 'moderator')
/// # "####;
/// ```
impl<'a, V, T, N, A, Rhs> BitOr<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: BooleanLike,
    N: Nullability + NullOr<Rhs::Nullable>,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: BooleanLike,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        <N as NullOr<Rhs::Nullable>>::Output,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        or(self, rhs)
    }
}
