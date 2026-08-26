//! Arithmetic operations using `std::ops` traits.
//!
//! This module implements `Add`, `Sub`, `Mul`, `Div`, `Rem` for `SQLExpr`,
//! enabling natural Rust syntax for SQL arithmetic.

use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{AddOp, ArithmeticOutput, DivOp, MulOp, NegOutput, Numeric, RemOp, SubOp};

use super::{AggOr, AggregateKind, Expr, Nullability, ResolveArithmeticNullability, SQLExpr};

type ArithmeticNullable<'a, V, T, N, Rhs, Op> = <<T as ArithmeticOutput<
    <Rhs as Expr<'a, V>>::SQLType,
    Op,
>>::Nullability as ResolveArithmeticNullability<
    N,
    <Rhs as Expr<'a, V>>::Nullable,
>>::Output;

#[inline]
fn binary_op_sql<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
{
    left.into_expr_sql()
        .push(operator)
        .append(right.into_expr_sql())
}

// =============================================================================
// Addition
// =============================================================================

impl<'a, V, T, N, A, Rhs> Add<Rhs> for SQLExpr<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: ArithmeticOutput<Rhs::SQLType, AddOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, AddOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, AddOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, AddOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

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
    T: ArithmeticOutput<Rhs::SQLType, SubOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, SubOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, SubOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, SubOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

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
    T: ArithmeticOutput<Rhs::SQLType, MulOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, MulOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, MulOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, MulOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

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
    T: ArithmeticOutput<Rhs::SQLType, DivOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, DivOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, DivOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, DivOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

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
    T: ArithmeticOutput<Rhs::SQLType, RemOp>,
    N: Nullability,
    A: AggOr<Rhs::Aggregate>,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    <T as ArithmeticOutput<Rhs::SQLType, RemOp>>::Nullability:
        ResolveArithmeticNullability<N, Rhs::Nullable>,
{
    type Output = SQLExpr<
        'a,
        V,
        <T as ArithmeticOutput<Rhs::SQLType, RemOp>>::Output,
        ArithmeticNullable<'a, V, T, N, Rhs, RemOp>,
        <A as AggOr<Rhs::Aggregate>>::Output,
    >;

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
    T: Numeric + NegOutput,
    N: Nullability,
    A: AggregateKind,
{
    type Output = SQLExpr<'a, V, T::Output, N, A>;

    fn neg(self) -> Self::Output {
        SQLExpr::new(SQL::from(Token::MINUS).append(self.into_expr_sql().parens()))
    }
}
