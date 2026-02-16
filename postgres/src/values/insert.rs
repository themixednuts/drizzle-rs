//! Insert value types for PostgreSQL

use super::{OwnedPostgresValue, PostgresValue};
use crate::prelude::*;
use core::marker::PhantomData;
use drizzle_core::{
    ToSQL, param::Param, placeholder::Placeholder, sql::SQL, sql::SQLChunk, traits::SQLParam,
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

//------------------------------------------------------------------------------
// InsertValue Definition - SQL-based value for inserts
//------------------------------------------------------------------------------

/// Wrapper for SQL with type information
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
    /// Converts this InsertValue to an owned version with 'static lifetime
    pub fn into_owned(self) -> PostgresInsertValue<'static, PostgresValue<'static>, T> {
        match self {
            PostgresInsertValue::Omit => PostgresInsertValue::Omit,
            PostgresInsertValue::Null => PostgresInsertValue::Null,
            PostgresInsertValue::Value(wrapper) => {
                // Convert PostgresValue parameters to owned values
                if let Some(SQLChunk::Param(param)) = wrapper.value.chunks.first() {
                    if let Some(ref postgres_val) = param.value {
                        let postgres_val = postgres_val.as_ref();
                        let owned_postgres_val = OwnedPostgresValue::from(postgres_val.clone());
                        let static_postgres_val = PostgresValue::from(owned_postgres_val);
                        let static_sql = SQL::param(static_postgres_val);
                        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                            static_sql,
                        ))
                    } else {
                        // NULL parameter
                        let static_sql = SQL::param(PostgresValue::Null);
                        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                            static_sql,
                        ))
                    }
                } else {
                    // Non-parameter chunk, convert to NULL for simplicity
                    let static_sql = SQL::param(PostgresValue::Null);
                    PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                        static_sql,
                    ))
                }
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
        let sql = value
            .try_into()
            .map(|v: PostgresValue<'a>| SQL::from(v))
            .unwrap_or_else(|_| SQL::from(PostgresValue::Null));
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

// Option conversion
impl<'a, T> From<Option<T>> for PostgresInsertValue<'a, PostgresValue<'a>, T>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => {
                let sql = v.to_sql();
                PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
            }
            None => PostgresInsertValue::Omit,
        }
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
