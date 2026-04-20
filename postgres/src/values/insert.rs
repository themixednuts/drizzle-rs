//! Insert value types for `PostgreSQL`

use super::{OwnedPostgresValue, PostgresValue};
use crate::prelude::*;
use core::marker::PhantomData;
use drizzle_core::{
    ToSQL, TypedPlaceholder, param::Param, placeholder::Placeholder, sql::SQL, sql::SQLChunk,
    traits::SQLParam,
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

//------------------------------------------------------------------------------
// InsertValue Definition - SQL-based value for inserts
//------------------------------------------------------------------------------

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

/// Represents a value for INSERT operations that can be omitted, null, or a SQL expression
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum PostgresInsertValue<'a, V: SQLParam, T> {
    /// Omit this column from the INSERT (use database default)
    #[default]
    Omit,
    /// Explicitly insert NULL
    Null,
    /// Insert a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, T> PostgresInsertValue<'a, PostgresValue<'a>, T> {
    /// Converts this `InsertValue` to an owned version with 'static lifetime
    #[must_use]
    pub fn into_owned(self) -> PostgresInsertValue<'static, PostgresValue<'static>, T> {
        match self {
            PostgresInsertValue::Omit => PostgresInsertValue::Omit,
            PostgresInsertValue::Null => PostgresInsertValue::Null,
            PostgresInsertValue::Value(wrapper) => {
                // Convert PostgresValue parameters to owned values
                let static_sql = match wrapper.value.chunks.first() {
                    Some(SQLChunk::Param(param)) => param.value.as_ref().map_or_else(
                        || SQL::param(PostgresValue::Null),
                        |postgres_val| {
                            let owned_postgres_val =
                                OwnedPostgresValue::from(postgres_val.as_ref().clone());
                            let static_postgres_val = PostgresValue::from(owned_postgres_val);
                            SQL::param(static_postgres_val)
                        },
                    ),
                    // Non-parameter chunk, convert to NULL for simplicity
                    _ => SQL::param(PostgresValue::Null),
                };
                PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                    static_sql,
                ))
            }
        }
    }
}

// Conversion implementations for PostgresValue-based InsertValue

// Generic conversion from any type T to InsertValue (for same type T)
// This works for types that implement TryInto<PostgresValue>, like enums,
// ArrayString, ArrayVec, etc.
impl<'a, T> From<T> for PostgresInsertValue<'a, PostgresValue<'a>, T>
where
    T: TryInto<PostgresValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = value.try_into().map_or_else(
            |_| SQL::from(PostgresValue::Null),
            |v: PostgresValue<'a>| SQL::from(v),
        );
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
    }
}

// Specific conversion for &str to String InsertValue
impl<'a> From<&str> for PostgresInsertValue<'a, PostgresValue<'a>, String> {
    fn from(value: &str) -> Self {
        let postgres_value = SQL::param(Cow::Owned(PostgresValue::from(value.to_string())));
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(
            postgres_value,
        ))
    }
}

// Placeholder conversion
impl<'a, T> From<Placeholder> for PostgresInsertValue<'a, PostgresValue<'a>, T> {
    fn from(placeholder: Placeholder) -> Self {
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(
            core::iter::once(chunk).collect(),
        ))
    }
}

impl<'a, M: drizzle_core::types::DataType, N: drizzle_core::expr::Nullability, T>
    From<TypedPlaceholder<M, N>> for PostgresInsertValue<'a, PostgresValue<'a>, T>
{
    fn from(typed: TypedPlaceholder<M, N>) -> Self {
        Placeholder::from(typed).into()
    }
}

// Option conversion
impl<'a, T> From<Option<T>> for PostgresInsertValue<'a, PostgresValue<'a>, T>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(PostgresInsertValue::Omit, |v| {
            PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(v.to_sql()))
        })
    }
}

// UUID conversion for String InsertValue (for text columns)
#[cfg(feature = "uuid")]
impl<'a> From<Uuid> for PostgresInsertValue<'a, PostgresValue<'a>, String> {
    fn from(value: Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(value);
        let sql = SQL::param(postgres_value);
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for PostgresInsertValue<'a, PostgresValue<'a>, String> {
    fn from(value: &'a Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(*value);
        let sql = SQL::param(postgres_value);
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}
