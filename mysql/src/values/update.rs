//! Driver-neutral values used by generated MySQL update models.

use super::{MySQLValue, insert::ValueWrapper};
use crate::prelude::*;
use crate::types::Any;
use drizzle_core::{
    MySQLDialect, SQL, SQLChunk, SQLParam, ToSQL, TypedPlaceholder,
    expr::{AcceptsNullability, ColumnBinOp, ColumnNeg, Expr, Null, Nullability, SQLExpr, Scalar},
    param::Param,
    placeholder::Placeholder,
    types::{Assignable, DataType},
};

/// A generated update field: skipped, explicitly null, or a typed expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum MySQLUpdateValue<
    'a,
    V: SQLParam,
    T,
    Target: DataType = Any,
    TargetNull: Nullability = Null,
> {
    /// Leave the column unchanged.
    #[default]
    Skip,
    /// Assign SQL `NULL`.
    Null,
    /// Assign a value, placeholder, or typed SQL expression.
    Value(ValueWrapper<'a, V, (T, Target, TargetNull)>),
}

impl<V: SQLParam, T, Target: DataType, TargetNull: Nullability>
    MySQLUpdateValue<'_, V, T, Target, TargetNull>
{
    /// Return whether this field is omitted from the `SET` clause.
    #[must_use]
    pub const fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

impl<'a, T, Target, TargetNull> From<T>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    T: Into<MySQLValue<'a>>,
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: T) -> Self {
        let sql = SQL::param(value.into());
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(sql))
    }
}

impl<'a, Target, TargetNull> From<&str>
    for MySQLUpdateValue<'a, MySQLValue<'a>, String, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: &str) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, String>::new(SQL::param(
            MySQLValue::from(String::from(value)),
        )))
    }
}

impl<'a, T, Target, TargetNull> From<Placeholder>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(placeholder: Placeholder) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(
            core::iter::once(SQLChunk::Param(Param {
                placeholder,
                value: None,
            }))
            .collect(),
        ))
    }
}

impl<'a, M, N, T, Target, TargetNull> From<TypedPlaceholder<M, N>>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    M: DataType,
    N: Nullability,
    Target: DataType + Assignable<M>,
    TargetNull: Nullability + AcceptsNullability<N>,
{
    fn from(value: TypedPlaceholder<M, N>) -> Self {
        Placeholder::from(value).into()
    }
}

impl<'a, T, Target, TargetNull, Actual, ActualNull>
    From<SQLExpr<'a, MySQLValue<'a>, Actual, ActualNull, Scalar>>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
{
    fn from(value: SQLExpr<'a, MySQLValue<'a>, Actual, ActualNull, Scalar>) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(
            value.into_expr_sql(),
        ))
    }
}

impl<'a, T, Target, TargetNull, L, R, Op, Actual, ActualNull>
    From<ColumnBinOp<L, R, Op, MySQLDialect, Actual, ActualNull>>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
    ColumnBinOp<L, R, Op, MySQLDialect, Actual, ActualNull>: ToSQL<'a, MySQLValue<'a>>,
{
    fn from(value: ColumnBinOp<L, R, Op, MySQLDialect, Actual, ActualNull>) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(value.into_sql()))
    }
}

impl<'a, T, Target, TargetNull, E, Actual, ActualNull>
    From<ColumnNeg<E, MySQLDialect, Actual, ActualNull>>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
    ColumnNeg<E, MySQLDialect, Actual, ActualNull>: ToSQL<'a, MySQLValue<'a>>,
{
    fn from(value: ColumnNeg<E, MySQLDialect, Actual, ActualNull>) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(value.into_sql()))
    }
}
