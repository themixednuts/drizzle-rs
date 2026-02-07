//! Update value types for SQLite
//!
//! Mirrors the `SQLiteInsertValue` pattern but simplified for UPDATE operations.
//! All UPDATE fields are optional (Skip = don't include in SET clause).

use drizzle_core::{Placeholder, SQL, SQLParam};

use super::SQLiteValue;
use super::insert::ValueWrapper;

/// Represents a value for UPDATE operations that can be skipped, null, or a SQL expression.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum SQLiteUpdateValue<'a, V: SQLParam, T> {
    /// Don't include this column in the SET clause
    #[default]
    Skip,
    /// Explicitly set column = NULL
    Null,
    /// Set column to a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, V: SQLParam, T> SQLiteUpdateValue<'a, V, T> {
    /// Returns true if this is `Skip`
    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

// Generic conversion from any type T that can convert to SQLiteValue
impl<'a, T, U> From<T> for SQLiteUpdateValue<'a, SQLiteValue<'a>, U>
where
    T: TryInto<SQLiteValue<'a>>,
    T: TryInto<U>,
    U: TryInto<SQLiteValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = TryInto::<U>::try_into(value)
            .map(|v| v.try_into().unwrap_or_default())
            .map(|v: SQLiteValue<'a>| SQL::from(v))
            .unwrap_or_else(|_| SQL::from(SQLiteValue::Null));
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(sql))
    }
}

// Placeholder conversion
impl<'a, T> From<Placeholder> for SQLiteUpdateValue<'a, SQLiteValue<'a>, T> {
    fn from(placeholder: Placeholder) -> Self {
        use drizzle_core::{Param, SQLChunk};
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(
            std::iter::once(chunk).collect(),
        ))
    }
}

// Array conversion for Vec<u8> UpdateValue
impl<'a, const N: usize> From<[u8; N]> for SQLiteUpdateValue<'a, SQLiteValue<'a>, Vec<u8>> {
    fn from(value: [u8; N]) -> Self {
        let sqlite_value = SQLiteValue::Blob(std::borrow::Cow::Owned(value.to_vec()));
        let sql = SQL::param(sqlite_value);
        SQLiteUpdateValue::Value(ValueWrapper::<SQLiteValue<'a>, Vec<u8>>::new(sql))
    }
}
