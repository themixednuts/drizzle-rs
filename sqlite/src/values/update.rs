//! Update value types for `SQLite`.
//!
//! Each field in an UPDATE operation can be skipped (left unchanged),
//! set to NULL, or set to a value or expression.

use crate::prelude::*;
use crate::types::Any;
use drizzle_core::expr::{
    AcceptsNullability, ColumnBinOp, ColumnNeg, Excluded, Expr, Null, Nullability, SQLExpr, Scalar,
};
use drizzle_core::types::{Assignable, DataType};
use drizzle_core::{
    Placeholder, SQL, SQLColumnInfo, SQLParam, SQLiteDialect, ToSQL, TypedPlaceholder,
};

use super::SQLiteValue;
use super::insert::ValueWrapper;

/// Represents a value for UPDATE operations that can be skipped, null, or a SQL expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum SQLiteUpdateValue<
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
    SQLiteUpdateValue<'_, V, T, Target, TargetNull>
{
    /// Returns true if this is `Skip`
    pub const fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

// Generic conversion from any type T that can convert to SQLiteValue
impl<'a, T, U, Target, TargetNull> From<T>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, U, Target, TargetNull>
where
    T: TryInto<SQLiteValue<'a>> + TryInto<U>,
    U: TryInto<SQLiteValue<'a>>,
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: T) -> Self {
        let sql = TryInto::<U>::try_into(value)
            .map(|v| v.try_into().unwrap_or_default())
            .map_or_else(
                |_| SQL::from(SQLiteValue::Null),
                |v: SQLiteValue<'a>| SQL::from(v),
            );
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(sql))
    }
}

// Placeholder conversion
impl<'a, T, Target, TargetNull> From<Placeholder>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(placeholder: Placeholder) -> Self {
        use drizzle_core::{Param, SQLChunk};
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(
            core::iter::once(chunk).collect(),
        ))
    }
}

impl<'a, M, N, T, Target, TargetNull> From<TypedPlaceholder<M, N>>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
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
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
where
    C: SQLColumnInfo + Expr<'a, SQLiteValue<'a>, SQLType = Actual, Nullable = ActualNull>,
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
{
    fn from(excluded: Excluded<C>) -> Self {
        use drizzle_core::ToSQL;
        let sql = excluded.to_sql();
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(sql))
    }
}

// Array conversion for Vec<u8> UpdateValue
impl<'a, const N: usize, Target, TargetNull> From<[u8; N]>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, Vec<u8>, Target, TargetNull>
where
    Target: DataType,
    TargetNull: Nullability,
{
    fn from(value: [u8; N]) -> Self {
        let sqlite_value = SQLiteValue::Blob(crate::prelude::Cow::Owned(value.to_vec()));
        let sql = SQL::param(sqlite_value);
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, Vec<u8>>::new(sql))
    }
}

impl<'a, T, Target, TargetNull, Actual, ActualNull>
    From<SQLExpr<'a, SQLiteValue<'a>, Actual, ActualNull, Scalar>>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
{
    fn from(value: SQLExpr<'a, SQLiteValue<'a>, Actual, ActualNull, Scalar>) -> Self {
        Self::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(
            value.into_expr_sql(),
        ))
    }
}

impl<'a, T, Target, TargetNull, L, R, Op, Actual, ActualNull>
    From<ColumnBinOp<L, R, Op, SQLiteDialect, Actual, ActualNull>>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
    ColumnBinOp<L, R, Op, SQLiteDialect, Actual, ActualNull>: ToSQL<'a, SQLiteValue<'a>>,
{
    fn from(value: ColumnBinOp<L, R, Op, SQLiteDialect, Actual, ActualNull>) -> Self {
        Self::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(value.into_sql()))
    }
}

impl<'a, T, Target, TargetNull, E, Actual, ActualNull>
    From<ColumnNeg<E, SQLiteDialect, Actual, ActualNull>>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
where
    Target: DataType + Assignable<Actual>,
    TargetNull: Nullability + AcceptsNullability<ActualNull>,
    Actual: DataType,
    ActualNull: Nullability,
    ColumnNeg<E, SQLiteDialect, Actual, ActualNull>: ToSQL<'a, SQLiteValue<'a>>,
{
    fn from(value: ColumnNeg<E, SQLiteDialect, Actual, ActualNull>) -> Self {
        Self::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(value.into_sql()))
    }
}
