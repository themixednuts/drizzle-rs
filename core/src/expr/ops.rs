//! Arithmetic operations using std::ops traits.
//!
//! This module implements `Add`, `Sub`, `Mul`, `Div`, `Rem` for `SQLExpr`,
//! enabling natural Rust syntax for SQL arithmetic.

use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{ArithmeticOutput, Numeric};

use super::{AggregateKind, Expr, NullOr, Nullability, SQLExpr, Scalar};

#[inline]
fn binary_op_sql<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
{
    left.into_sql()
        .parens_if_subquery()
        .push(operator)
        .append(right.into_sql().parens_if_subquery())
}

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
        SQLExpr::new(binary_op_sql(self, Token::PLUS, rhs))
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
        SQLExpr::new(binary_op_sql(self, Token::MINUS, rhs))
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
        SQLExpr::new(binary_op_sql(self, Token::STAR, rhs))
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
        SQLExpr::new(binary_op_sql(self, Token::SLASH, rhs))
    }
}

// =============================================================================
// Remainder (Modulo)
// =============================================================================

impl<'a, V, T, N, A, Rhs> Rem<Rhs> for SQLExpr<'a, V, T, N, A>
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

    fn rem(self, rhs: Rhs) -> Self::Output {
        SQLExpr::new(binary_op_sql(self, Token::REM, rhs))
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
        SQLExpr::new(SQL::from(Token::MINUS).append(self.into_sql().parens()))
    }
}
