//! Internal wrapper types for column arithmetic operations.
//!
//! These types are implementation details that allow `column + 5` syntax
//! to work seamlessly. Users don't interact with these directly.

use core::marker::PhantomData;

use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{ArithmeticOutput, Numeric};

use super::{Expr, NullOr, Nullability, Scalar};

/// Binary operation result for column arithmetic.
///
/// This is an implementation detail - users see `column + 5` and it "just works".
#[derive(Debug, Clone, Copy)]
pub struct ColumnBinOp<Lhs, Rhs, Op> {
    lhs: Lhs,
    rhs: Rhs,
    _op: PhantomData<Op>,
}

impl<Lhs, Rhs, Op> ColumnBinOp<Lhs, Rhs, Op> {
    #[inline]
    pub fn new(lhs: Lhs, rhs: Rhs) -> Self {
        Self {
            lhs,
            rhs,
            _op: PhantomData,
        }
    }
}

/// Marker for addition
#[derive(Debug, Clone, Copy)]
pub struct OpAdd;

/// Marker for subtraction
#[derive(Debug, Clone, Copy)]
pub struct OpSub;

/// Marker for multiplication
#[derive(Debug, Clone, Copy)]
pub struct OpMul;

/// Marker for division
#[derive(Debug, Clone, Copy)]
pub struct OpDiv;

/// Marker for remainder/modulo
#[derive(Debug, Clone, Copy)]
pub struct OpRem;

/// Trait to get the token for an operation
pub trait BinOpToken {
    const TOKEN: Token;
}

impl BinOpToken for OpAdd {
    const TOKEN: Token = Token::PLUS;
}

impl BinOpToken for OpSub {
    const TOKEN: Token = Token::MINUS;
}

impl BinOpToken for OpMul {
    const TOKEN: Token = Token::STAR;
}

impl BinOpToken for OpDiv {
    const TOKEN: Token = Token::SLASH;
}

impl BinOpToken for OpRem {
    const TOKEN: Token = Token::REM;
}

impl<'a, V, Lhs, Rhs, Op> ToSQL<'a, V> for ColumnBinOp<Lhs, Rhs, Op>
where
    V: SQLParam,
    Lhs: ToSQL<'a, V>,
    Rhs: ToSQL<'a, V>,
    Op: BinOpToken,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.lhs.to_sql().push(Op::TOKEN).append(self.rhs.to_sql())
    }
}

impl<'a, V, Lhs, Rhs, Op> Expr<'a, V> for ColumnBinOp<Lhs, Rhs, Op>
where
    V: SQLParam,
    Lhs: Expr<'a, V>,
    Rhs: Expr<'a, V>,
    Lhs::SQLType: Numeric + ArithmeticOutput<Rhs::SQLType>,
    Rhs::SQLType: Numeric,
    Lhs::Nullable: NullOr<Rhs::Nullable>,
    Rhs::Nullable: Nullability,
    Op: BinOpToken,
{
    type SQLType = <Lhs::SQLType as ArithmeticOutput<Rhs::SQLType>>::Output;
    type Nullable = <Lhs::Nullable as NullOr<Rhs::Nullable>>::Output;
    type Aggregate = Scalar;
}

/// Negation result for column arithmetic.
#[derive(Debug, Clone, Copy)]
pub struct ColumnNeg<T> {
    inner: T,
}

impl<T> ColumnNeg<T> {
    #[inline]
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<'a, V, T> ToSQL<'a, V> for ColumnNeg<T>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw("-").append(self.inner.to_sql())
    }
}

impl<'a, V, T> Expr<'a, V> for ColumnNeg<T>
where
    V: SQLParam,
    T: Expr<'a, V>,
    T::SQLType: Numeric,
{
    type SQLType = T::SQLType;
    type Nullable = T::Nullable;
    type Aggregate = Scalar;
}
