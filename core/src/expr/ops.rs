//! Arithmetic operations using std::ops traits.
//!
//! This module implements `Add`, `Sub`, `Mul`, `Div` for `SQLExpr`,
//! enabling natural Rust syntax for SQL arithmetic.

use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::sql::{Token, SQL};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{ArithmeticOutput, Numeric};

use super::{AggregateKind, Expr, NullOr, Nullability, Scalar, SQLExpr};

// =============================================================================
// Addition
// =============================================================================

impl<'a, V, T, N, A, Rhs> Add<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType>,
    N: Nullability + NullOr<Rhs::Nullable>,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<'a, V, T::Output, <N as NullOr<Rhs::Nullable>>::Output, Scalar>;

    fn add(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(self.to_sql().push(Token::PLUS).append(rhs.to_sql()))
    }
}

// =============================================================================
// Subtraction
// =============================================================================

impl<'a, V, T, N, A, Rhs> Sub<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType>,
    N: Nullability + NullOr<Rhs::Nullable>,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<'a, V, T::Output, <N as NullOr<Rhs::Nullable>>::Output, Scalar>;

    fn sub(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(self.to_sql().push(Token::MINUS).append(rhs.to_sql()))
    }
}

// =============================================================================
// Multiplication
// =============================================================================

impl<'a, V, T, N, A, Rhs> Mul<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType>,
    N: Nullability + NullOr<Rhs::Nullable>,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<'a, V, T::Output, <N as NullOr<Rhs::Nullable>>::Output, Scalar>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(self.to_sql().push(Token::STAR).append(rhs.to_sql()))
    }
}

// =============================================================================
// Division
// =============================================================================

impl<'a, V, T, N, A, Rhs> Div<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType>,
    N: Nullability + NullOr<Rhs::Nullable>,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<'a, V, T::Output, <N as NullOr<Rhs::Nullable>>::Output, Scalar>;

    fn div(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(self.to_sql().push(Token::SLASH).append(rhs.to_sql()))
    }
}

// =============================================================================
// Negation
// =============================================================================

impl<'a, V, T, N, A> Neg for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: Numeric,
    N: Nullability,
    A: AggregateKind,
{
    type Output = SQLExpr<'a, V, T, N, Scalar>;

    fn neg(self) -> Self::Output {
        SQLExpr::new(SQL::from(Token::MINUS).append(self.to_sql().parens()))
    }
}
