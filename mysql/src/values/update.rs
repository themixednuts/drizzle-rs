//! Driver-neutral values used by generated MySQL update models.

use super::{MySQLValue, insert::ValueWrapper};
use crate::prelude::*;
use drizzle_core::{
    SQL, SQLChunk, SQLParam, TypedPlaceholder, param::Param, placeholder::Placeholder,
};

/// A generated update field: skipped, explicitly null, or a typed expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum MySQLUpdateValue<'a, V: SQLParam, T> {
    /// Leave the column unchanged.
    #[default]
    Skip,
    /// Assign SQL `NULL`.
    Null,
    /// Assign a value, placeholder, or typed SQL expression.
    Value(ValueWrapper<'a, V, T>),
}

impl<V: SQLParam, T> MySQLUpdateValue<'_, V, T> {
    /// Return whether this field is omitted from the `SET` clause.
    #[must_use]
    pub const fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

impl<'a, T> From<T> for MySQLUpdateValue<'a, MySQLValue<'a>, T>
where
    T: Into<MySQLValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = SQL::param(value.into());
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(sql))
    }
}

impl<'a> From<&str> for MySQLUpdateValue<'a, MySQLValue<'a>, String> {
    fn from(value: &str) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, String>::new(SQL::param(
            MySQLValue::from(String::from(value)),
        )))
    }
}

impl<'a, T> From<Placeholder> for MySQLUpdateValue<'a, MySQLValue<'a>, T> {
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

impl<'a, M, N, T> From<TypedPlaceholder<M, N>> for MySQLUpdateValue<'a, MySQLValue<'a>, T>
where
    M: drizzle_core::types::DataType,
    N: drizzle_core::expr::Nullability,
{
    fn from(value: TypedPlaceholder<M, N>) -> Self {
        Placeholder::from(value).into()
    }
}
