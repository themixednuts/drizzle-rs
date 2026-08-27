//! Update value types for `PostgreSQL`.
//!
//! Each field in an UPDATE operation can be skipped (left unchanged),
//! set to NULL, or set to a value or expression.

use super::PostgresValue;
use super::insert::ValueWrapper;
use crate::prelude::*;
use crate::types::Any;
use drizzle_core::expr::{
    AcceptsNullability, ColumnBinOp, ColumnNeg, Excluded, Expr, Null, Nullability, SQLExpr, Scalar,
};
use drizzle_core::{
    PostgresDialect, SQLColumnInfo, ToSQL, TypedPlaceholder,
    param::Param,
    placeholder::Placeholder,
    sql::SQL,
    sql::SQLChunk,
    traits::SQLParam,
    types::{Assignable, DataType},
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Represents a value for UPDATE operations that can be skipped, null, or a SQL expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum PostgresUpdateValue<
    'a,
    V: SQLParam,
    T,
    Target: DataType = Any,
    TargetNull: Nullability = Null,
> {
    /// Don't include this column in the SET clause
    #[default]
    Skip,
    /// Explicitly set column = NULL
    Null,
    /// Set column to a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, (T, Target, TargetNull)>),
}

impl<V: SQLParam, T, Target: DataType, TargetNull: Nullability>
    PostgresUpdateValue<'_, V, T, Target, TargetNull>
{
    /// Returns true if this is `Skip`
    pub const fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

// Generic conversion from any type T to UpdateValue
impl<'a, T, Target, TargetNull> From<T>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    T: TryInto<PostgresValue<'a>>,
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: T) -> Self {
        let sql = value.try_into().map_or_else(
            |_| SQL::from(PostgresValue::Null),
            |v: PostgresValue<'a>| SQL::from(v),
        );
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
    }
}

// Specific conversion for &str to String UpdateValue
impl<'a, Target, TargetNull> From<&str>
    for PostgresUpdateValue<'a, PostgresValue<'a>, String, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: &str) -> Self {
        let postgres_value = SQL::param(Cow::Owned(PostgresValue::from(value.to_string())));
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(
            postgres_value,
        ))
    }
}

// Placeholder conversion
impl<'a, T, Target, TargetNull> From<Placeholder>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(placeholder: Placeholder) -> Self {
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(
            core::iter::once(chunk).collect(),
        ))
    }
}

impl<'a, M, N, T, Target, TargetNull> From<TypedPlaceholder<M, N>>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    M: DataType,
    N: Nullability,
    Target: DataType + Assignable<M>,
    TargetNull: Nullability + AcceptsNullability<N>,
{
    fn from(typed: TypedPlaceholder<M, N>) -> Self {
        Placeholder::from(typed).into()
    }
}

// Excluded column reference conversion (for ON CONFLICT DO UPDATE SET)
impl<'a, C, T, Target, TargetNull, Actual, ActualNull> From<Excluded<C>>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    C: SQLColumnInfo + Expr<'a, PostgresValue<'a>, SQLType = Actual, Nullable = ActualNull>,
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
{
    fn from(excluded: Excluded<C>) -> Self {
        use drizzle_core::ToSQL;
        let sql = excluded.to_sql();
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
    }
}

// UUID conversion for String UpdateValue (for text columns)
#[cfg(feature = "uuid")]
impl<'a, Target, TargetNull> From<Uuid>
    for PostgresUpdateValue<'a, PostgresValue<'a>, String, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(value);
        let sql = SQL::param(postgres_value);
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}

#[cfg(feature = "uuid")]
impl<'a, Target, TargetNull> From<&'a Uuid>
    for PostgresUpdateValue<'a, PostgresValue<'a>, String, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: &'a Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(*value);
        let sql = SQL::param(postgres_value);
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}

impl<'a, T, Target, TargetNull, Actual, ActualNull>
    From<SQLExpr<'a, PostgresValue<'a>, Actual, ActualNull, Scalar>>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
{
    fn from(value: SQLExpr<'a, PostgresValue<'a>, Actual, ActualNull, Scalar>) -> Self {
        Self::Value(ValueWrapper::<PostgresValue<'a>, T>::new(
            value.into_expr_sql(),
        ))
    }
}

impl<'a, T, Target, TargetNull, L, R, Op, Actual, ActualNull>
    From<ColumnBinOp<L, R, Op, PostgresDialect, Actual, ActualNull>>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
    ColumnBinOp<L, R, Op, PostgresDialect, Actual, ActualNull>: ToSQL<'a, PostgresValue<'a>>,
{
    fn from(value: ColumnBinOp<L, R, Op, PostgresDialect, Actual, ActualNull>) -> Self {
        Self::Value(ValueWrapper::<PostgresValue<'a>, T>::new(value.into_sql()))
    }
}

impl<'a, T, Target, TargetNull, E, Actual, ActualNull>
    From<ColumnNeg<E, PostgresDialect, Actual, ActualNull>>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
    ColumnNeg<E, PostgresDialect, Actual, ActualNull>: ToSQL<'a, PostgresValue<'a>>,
{
    fn from(value: ColumnNeg<E, PostgresDialect, Actual, ActualNull>) -> Self {
        Self::Value(ValueWrapper::<PostgresValue<'a>, T>::new(value.into_sql()))
    }
}
