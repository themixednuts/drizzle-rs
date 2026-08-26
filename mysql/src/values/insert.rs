//! Driver-neutral values used by generated MySQL insert models.

use super::MySQLValue;
use crate::prelude::*;
use core::marker::PhantomData;
use drizzle_core::{
    SQL, SQLChunk, SQLParam, ToSQL, TypedPlaceholder, param::Param, placeholder::Placeholder,
};

/// A typed SQL fragment stored in an insert or update field.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ValueWrapper<'a, V: SQLParam, T> {
    pub value: SQL<'a, V>,
    pub _phantom: PhantomData<T>,
}

impl<'a, V: SQLParam, T> ValueWrapper<'a, V, T> {
    pub const fn new<U>(value: SQL<'a, V>) -> ValueWrapper<'a, V, U> {
        ValueWrapper {
            value,
            _phantom: PhantomData,
        }
    }
}

/// A generated insert field: omitted, explicitly null, or a typed expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum MySQLInsertValue<'a, V: SQLParam, T> {
    /// Omit the column and let MySQL apply its default.
    #[default]
    Omit,
    /// Insert SQL `NULL`.
    Null,
    /// Insert a value, placeholder, or typed SQL expression.
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, T> MySQLInsertValue<'a, MySQLValue<'a>, T> {
    /// Detach all borrowed SQL and parameter data.
    #[must_use]
    pub fn into_owned(self) -> MySQLInsertValue<'static, MySQLValue<'static>, T> {
        match self {
            Self::Omit => MySQLInsertValue::Omit,
            Self::Null => MySQLInsertValue::Null,
            Self::Value(wrapper) => {
                MySQLInsertValue::Value(ValueWrapper::<MySQLValue<'static>, T>::new(
                    wrapper
                        .value
                        .map_params_into_owned(|value| MySQLValue::from(value.into_owned())),
                ))
            }
        }
    }
}

impl<'a, T> From<T> for MySQLInsertValue<'a, MySQLValue<'a>, T>
where
    T: Into<MySQLValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = SQL::param(value.into());
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(sql))
    }
}

impl<'a> From<&str> for MySQLInsertValue<'a, MySQLValue<'a>, String> {
    fn from(value: &str) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, String>::new(SQL::param(
            MySQLValue::from(String::from(value)),
        )))
    }
}

impl<'a, T> From<Placeholder> for MySQLInsertValue<'a, MySQLValue<'a>, T> {
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

impl<'a, M, N, T> From<TypedPlaceholder<M, N>> for MySQLInsertValue<'a, MySQLValue<'a>, T>
where
    M: drizzle_core::types::DataType,
    N: drizzle_core::expr::Nullability,
{
    fn from(value: TypedPlaceholder<M, N>) -> Self {
        Placeholder::from(value).into()
    }
}

impl<'a, T> From<Option<T>> for MySQLInsertValue<'a, MySQLValue<'a>, T>
where
    T: ToSQL<'a, MySQLValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Omit, |value| {
            Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(value.to_sql()))
        })
    }
}
