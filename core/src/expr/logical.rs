//! Logical operators (AND, OR, NOT).
//!
//! This module provides both function-based and operator-based logical operations:
//!
//! ```ignore
//! // Function style
//! and2(condition1, condition2)
//! or2(condition1, condition2)
//! not(condition)
//!
//! // Operator style (via std::ops traits)
//! condition1 & condition2   // BitAnd
//! condition1 | condition2   // BitOr
//! !condition                 // Not
//! ```

use core::ops::{BitAnd, BitOr, Not};

use crate::sql::{SQL, SQLChunk, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::Bool;

use super::{AggregateKind, Expr, NonNull, Nullability, SQLExpr, Scalar};

// =============================================================================
// NOT
// =============================================================================

/// Logical NOT.
///
/// Negates a boolean expression.
pub fn not<'a, V, E>(expr: E) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
{
    let expr_sql = expr.to_sql();
    let needs_paren = expr_sql.chunks.len() > 1
        || (expr_sql.chunks.len() == 1
            && !matches!(expr_sql.chunks[0], SQLChunk::Raw(_) | SQLChunk::Ident(_)));

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

/// Logical AND of multiple conditions.
///
/// Returns a boolean expression that is true if all conditions are true.
/// Accepts any iterable of items that implement ToSQL.
pub fn and<'a, V, I, E>(conditions: I) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    I: IntoIterator<Item = E>,
    E: ToSQL<'a, V>,
{
    let mut iter = conditions.into_iter();

    let sql = match iter.next() {
        None => SQL::empty(),
        Some(first) => {
            let first_sql = first.to_sql();
            let Some(second) = iter.next() else {
                return SQLExpr::new(first_sql);
            };
            let all_conditions = core::iter::once(first_sql)
                .chain(core::iter::once(second.to_sql()))
                .chain(iter.map(|c| c.to_sql()));
            SQL::from(Token::LPAREN)
                .append(SQL::join(all_conditions, Token::AND))
                .push(Token::RPAREN)
        }
    };
    SQLExpr::new(sql)
}

/// Logical AND of two expressions.
pub fn and2<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(left.to_sql())
            .push(Token::AND)
            .append(right.to_sql())
            .push(Token::RPAREN),
    )
}

// =============================================================================
// OR
// =============================================================================

/// Logical OR of multiple conditions.
///
/// Returns a boolean expression that is true if any condition is true.
/// Accepts any iterable of items that implement ToSQL.
pub fn or<'a, V, I, E>(conditions: I) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    I: IntoIterator<Item = E>,
    E: ToSQL<'a, V>,
{
    let mut iter = conditions.into_iter();

    let sql = match iter.next() {
        None => SQL::empty(),
        Some(first) => {
            let first_sql = first.to_sql();
            let Some(second) = iter.next() else {
                return SQLExpr::new(first_sql);
            };
            let all_conditions = core::iter::once(first_sql)
                .chain(core::iter::once(second.to_sql()))
                .chain(iter.map(|c| c.to_sql()));
            SQL::from(Token::LPAREN)
                .append(SQL::join(all_conditions, Token::OR))
                .push(Token::RPAREN)
        }
    };
    SQLExpr::new(sql)
}

/// Logical OR of two expressions.
pub fn or2<'a, V, L, R>(left: L, right: R) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(left.to_sql())
            .push(Token::OR)
            .append(right.to_sql())
            .push(Token::RPAREN),
    )
}

// =============================================================================
// Operator Trait Implementations
// =============================================================================

/// Implements `!expr` for boolean expressions (SQL NOT).
///
/// # Example
///
/// ```ignore
/// let condition = eq(users.active, true);
/// let negated = !condition;  // NOT "users"."active" = TRUE
/// ```
impl<'a, V, N, A> Not for SQLExpr<'a, V, Bool, N, A>
where
    V: SQLParam + 'a,
    N: Nullability,
    A: AggregateKind,
{
    type Output = SQLExpr<'a, V, Bool, NonNull, Scalar>;

    fn not(self) -> Self::Output {
        not(self)
    }
}

/// Implements `expr1 & expr2` for boolean expressions (SQL AND).
///
/// # Example
///
/// ```ignore
/// let condition = eq(users.active, true) & gt(users.age, 18);
/// // ("users"."active" = TRUE AND "users"."age" > 18)
/// ```
impl<'a, V, N, A, Rhs> BitAnd<Rhs> for SQLExpr<'a, V, Bool, N, A>
where
    V: SQLParam + 'a,
    N: Nullability,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
{
    type Output = SQLExpr<'a, V, Bool, NonNull, Scalar>;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        and2(self, rhs)
    }
}

/// Implements `expr1 | expr2` for boolean expressions (SQL OR).
///
/// # Example
///
/// ```ignore
/// let condition = eq(users.role, "admin") | eq(users.role, "moderator");
/// // ("users"."role" = 'admin' OR "users"."role" = 'moderator')
/// ```
impl<'a, V, N, A, Rhs> BitOr<Rhs> for SQLExpr<'a, V, Bool, N, A>
where
    V: SQLParam + 'a,
    N: Nullability,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
{
    type Output = SQLExpr<'a, V, Bool, NonNull, Scalar>;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        or2(self, rhs)
    }
}
