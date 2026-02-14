//! Update value types for PostgreSQL
//!
//! Mirrors the `PostgresInsertValue` pattern but simplified for UPDATE operations.
//! All UPDATE fields are optional (Skip = don't include in SET clause).

use super::PostgresValue;
use super::insert::ValueWrapper;
use drizzle_core::expr::Excluded;
use drizzle_core::{
    SQLColumnInfo, param::Param, placeholder::Placeholder, sql::SQL, sql::SQLChunk,
    traits::SQLParam,
};
use std::borrow::Cow;

#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Represents a value for UPDATE operations that can be skipped, null, or a SQL expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum PostgresUpdateValue<'a, V: SQLParam, T> {
    /// Don't include this column in the SET clause
    #[default]
    Skip,
    /// Explicitly set column = NULL
    Null,
    /// Set column to a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, V: SQLParam, T> PostgresUpdateValue<'a, V, T> {
    /// Returns true if this is `Skip`
    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

// Generic conversion from any type T to UpdateValue
impl<'a, T> From<T> for PostgresUpdateValue<'a, PostgresValue<'a>, T>
where
    T: TryInto<PostgresValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = value
            .try_into()
            .map(|v: PostgresValue<'a>| SQL::from(v))
            .unwrap_or_else(|_| SQL::from(PostgresValue::Null));
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
    }
}

// Specific conversion for &str to String UpdateValue
impl<'a> From<&str> for PostgresUpdateValue<'a, PostgresValue<'a>, String> {
    fn from(value: &str) -> Self {
        let postgres_value = SQL::param(Cow::Owned(PostgresValue::from(value.to_string())));
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(
            postgres_value,
        ))
    }
}

// Placeholder conversion
impl<'a, T> From<Placeholder> for PostgresUpdateValue<'a, PostgresValue<'a>, T> {
    fn from(placeholder: Placeholder) -> Self {
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(
            std::iter::once(chunk).collect(),
        ))
    }
}

// Excluded column reference conversion (for ON CONFLICT DO UPDATE SET)
impl<'a, C, T> From<Excluded<C>> for PostgresUpdateValue<'a, PostgresValue<'a>, T>
where
    C: SQLColumnInfo,
{
    fn from(excluded: Excluded<C>) -> Self {
        use drizzle_core::ToSQL;
        let sql = excluded.to_sql();
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
    }
}

// UUID conversion for String UpdateValue (for text columns)
#[cfg(feature = "uuid")]
impl<'a> From<Uuid> for PostgresUpdateValue<'a, PostgresValue<'a>, String> {
    fn from(value: Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(value);
        let sql = SQL::param(postgres_value);
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for PostgresUpdateValue<'a, PostgresValue<'a>, String> {
    fn from(value: &'a Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(*value);
        let sql = SQL::param(postgres_value);
        PostgresUpdateValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}
