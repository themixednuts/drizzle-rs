//! Internal wrapper types for column arithmetic operations.
//!
//! These types are implementation details that allow `column + 5` syntax
//! to work seamlessly. Users don't interact with these directly.

use core::marker::PhantomData;

use crate::ValueTypeForDialect;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{AlwaysNullable, ArithmeticOutput, NegOutput, Numeric, PropagateNullability};

use super::{AggOr, AggregateKind, Expr, NonNull, NullOr, Nullability};

/// Binary operation result for column arithmetic.
///
/// This is an implementation detail - users see `column + 5` and it "just works".
#[derive(Debug, Clone, Copy)]
pub struct ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable> {
    lhs: Lhs,
    rhs: Rhs,
    _type: PhantomData<(Op, D, SQLType, Nullable)>,
}

impl<Lhs, Rhs, Op, D, SQLType, Nullable> ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable> {
    #[inline]
    pub const fn new(lhs: Lhs, rhs: Rhs) -> Self {
        Self {
            lhs,
            rhs,
            _type: PhantomData,
        }
    }
}

#[doc(hidden)]
pub use crate::types::{
    AddOp as OpAdd, DivOp as OpDiv, MulOp as OpMul, RemOp as OpRem, SubOp as OpSub,
};

/// Trait to get the token for an operation
pub trait BinOpToken {
    const TOKEN: Token;
}

#[doc(hidden)]
pub trait ResolveArithmeticNullability<Lhs, Rhs> {
    type Output: Nullability;
}

/// Lifetime-independent type metadata for the right-hand side of generated
/// column arithmetic operators.
#[doc(hidden)]
pub trait ArithmeticRhs<D> {
    type SQLType: Numeric;
    type Nullable: Nullability;
}

/// Computes and constructs the result of a generated column arithmetic
/// operator without tying right-hand-side metadata to an expression lifetime.
#[doc(hidden)]
pub trait BuildColumnArithmetic<Lhs, Rhs, Op, D, LhsSQLType, LhsNullable> {
    type Output;

    fn build(lhs: Lhs, rhs: Rhs) -> Self::Output;
}

impl<Lhs, Rhs, Op, D, LhsSQLType, LhsNullable>
    BuildColumnArithmetic<Lhs, Rhs, Op, D, LhsSQLType, LhsNullable> for ()
where
    Rhs: ArithmeticRhs<D>,
    LhsSQLType: ArithmeticOutput<Rhs::SQLType, Op>,
    <LhsSQLType as ArithmeticOutput<Rhs::SQLType, Op>>::Nullability:
        ResolveArithmeticNullability<LhsNullable, Rhs::Nullable>,
{
    type Output = ColumnBinOp<
        Lhs,
        Rhs,
        Op,
        D,
        <LhsSQLType as ArithmeticOutput<Rhs::SQLType, Op>>::Output,
        <<LhsSQLType as ArithmeticOutput<Rhs::SQLType, Op>>::Nullability as ResolveArithmeticNullability<
            LhsNullable,
            Rhs::Nullable,
        >>::Output,
    >;

    fn build(lhs: Lhs, rhs: Rhs) -> Self::Output {
        ColumnBinOp::new(lhs, rhs)
    }
}

macro_rules! arithmetic_value_rhs {
    ($dialect:ty; $($value:ty),+ $(,)?) => {
        $(
            impl ArithmeticRhs<$dialect> for $value
            where
                <$value as ValueTypeForDialect<$dialect>>::SQLType: Numeric,
            {
                type SQLType = <$value as ValueTypeForDialect<$dialect>>::SQLType;
                type Nullable = NonNull;
            }
        )+
    };
}

arithmetic_value_rhs!(
    crate::SQLiteDialect;
    i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool, f32, f64
);
arithmetic_value_rhs!(
    crate::PostgresDialect;
    i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64
);
arithmetic_value_rhs!(
    crate::MySQLDialect;
    i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64
);

#[cfg(feature = "rust-decimal")]
arithmetic_value_rhs!(crate::PostgresDialect; rust_decimal::Decimal);
#[cfg(feature = "rust-decimal")]
arithmetic_value_rhs!(crate::MySQLDialect; rust_decimal::Decimal);

impl<D, T> ArithmeticRhs<D> for Option<T>
where
    T: ValueTypeForDialect<D>,
    T::SQLType: Numeric,
{
    type SQLType = T::SQLType;
    type Nullable = super::Null;
}

impl<D, T> ArithmeticRhs<D> for &T
where
    T: ArithmeticRhs<D> + ?Sized,
{
    type SQLType = T::SQLType;
    type Nullable = T::Nullable;
}

impl<D, V, T, N, A> ArithmeticRhs<D> for super::SQLExpr<'_, V, T, N, A>
where
    V: SQLParam<DialectMarker = D>,
    T: Numeric,
    N: Nullability,
    A: AggregateKind,
{
    type SQLType = T;
    type Nullable = N;
}

impl<Lhs, Rhs> ResolveArithmeticNullability<Lhs, Rhs> for PropagateNullability
where
    Lhs: Nullability + NullOr<Rhs>,
    Rhs: Nullability,
{
    type Output = <Lhs as NullOr<Rhs>>::Output;
}

impl<Lhs: Nullability, Rhs: Nullability> ResolveArithmeticNullability<Lhs, Rhs> for AlwaysNullable {
    type Output = super::Null;
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

impl<'a, V, Lhs, Rhs, Op, D, SQLType, Nullable> ToSQL<'a, V>
    for ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
where
    V: SQLParam,
    Lhs: ToSQL<'a, V>,
    Rhs: ToSQL<'a, V>,
    Op: BinOpToken,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.lhs.to_sql().push(Op::TOKEN).append(self.rhs.to_sql())
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.lhs
            .into_sql()
            .push(Op::TOKEN)
            .append(self.rhs.into_sql())
    }
}

impl<'a, V, Lhs, Rhs, Op, D, SQLType, Nullable> Expr<'a, V>
    for ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
where
    V: SQLParam<DialectMarker = D>,
    Lhs: Expr<'a, V>,
    Rhs: Expr<'a, V>,
    Lhs::SQLType: Numeric + ArithmeticOutput<Rhs::SQLType, Op, Output = SQLType>,
    Rhs::SQLType: Numeric,
    Rhs::Nullable: Nullability,
    Lhs::Aggregate: AggOr<Rhs::Aggregate>,
    Rhs::Aggregate: AggregateKind,
    Op: BinOpToken,
    SQLType: crate::types::DataType,
    Nullable: Nullability,
    <Lhs::SQLType as ArithmeticOutput<Rhs::SQLType, Op>>::Nullability:
        ResolveArithmeticNullability<Lhs::Nullable, Rhs::Nullable, Output = Nullable>,
{
    type SQLType = SQLType;
    type Nullable = Nullable;
    type Aggregate = <Lhs::Aggregate as AggOr<Rhs::Aggregate>>::Output;
}

impl<Lhs, Rhs, Op, D, SQLType, Nullable> super::HasAggStatus
    for ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
where
    Lhs: super::HasAggStatus,
    Rhs: super::HasAggStatus,
    Lhs::Status: super::CombineAggStatus<Rhs::Status>,
{
    type Status = <Lhs::Status as super::CombineAggStatus<Rhs::Status>>::Output;
}

impl<Lhs, Rhs, Op, D, SQLType, Nullable> ArithmeticRhs<D>
    for ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
where
    SQLType: Numeric,
    Nullable: Nullability,
{
    type SQLType = SQLType;
    type Nullable = Nullable;
}

impl<Lhs, Rhs, Op, D, SQLType, Nullable> crate::row::ExprValueType
    for ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
where
    SQLType: crate::types::DataType + crate::row::SQLTypeToRust<D>,
    Nullable:
        Nullability + crate::row::WrapNullable<<SQLType as crate::row::SQLTypeToRust<D>>::RustType>,
{
    type ValueType = <Nullable as crate::row::WrapNullable<
        <SQLType as crate::row::SQLTypeToRust<D>>::RustType,
    >>::Output;
}

impl<Lhs, Rhs, Op, D, SQLType, Nullable> crate::row::IntoSelectTarget
    for ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
{
    type Marker = crate::row::SelectCols<(Self,)>;
}

/// Negation result for column arithmetic.
#[derive(Debug, Clone, Copy)]
pub struct ColumnNeg<T, D, SQLType, Nullable> {
    inner: T,
    _type: PhantomData<(D, SQLType, Nullable)>,
}

impl<T, D, SQLType, Nullable> ColumnNeg<T, D, SQLType, Nullable> {
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            _type: PhantomData,
        }
    }
}

impl<'a, V, T, D, SQLType, Nullable> ToSQL<'a, V> for ColumnNeg<T, D, SQLType, Nullable>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw("-").append(self.inner.to_sql())
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::raw("-").append(self.inner.into_sql())
    }
}

impl<'a, V, T, D, SQLType, Nullable> Expr<'a, V> for ColumnNeg<T, D, SQLType, Nullable>
where
    V: SQLParam<DialectMarker = D>,
    T: Expr<'a, V, Nullable = Nullable>,
    T::SQLType: Numeric + NegOutput<Output = SQLType>,
    SQLType: crate::types::DataType,
    Nullable: Nullability,
{
    type SQLType = SQLType;
    type Nullable = Nullable;
    type Aggregate = T::Aggregate;
}

impl<T: super::HasAggStatus, D, SQLType, Nullable> super::HasAggStatus
    for ColumnNeg<T, D, SQLType, Nullable>
{
    type Status = T::Status;
}

impl<T, D, SQLType, Nullable> crate::row::ExprValueType for ColumnNeg<T, D, SQLType, Nullable>
where
    SQLType: crate::types::DataType + crate::row::SQLTypeToRust<D>,
    Nullable:
        Nullability + crate::row::WrapNullable<<SQLType as crate::row::SQLTypeToRust<D>>::RustType>,
{
    type ValueType = <Nullable as crate::row::WrapNullable<
        <SQLType as crate::row::SQLTypeToRust<D>>::RustType,
    >>::Output;
}

impl<T, D, SQLType, Nullable> crate::row::IntoSelectTarget for ColumnNeg<T, D, SQLType, Nullable> {
    type Marker = crate::row::SelectCols<(Self,)>;
}

impl<T, D, SQLType, Nullable> ArithmeticRhs<D> for ColumnNeg<T, D, SQLType, Nullable>
where
    SQLType: Numeric,
    Nullable: Nullability,
{
    type SQLType = SQLType;
    type Nullable = Nullable;
}
