//------------------------------------------------------------------------------
// InsertValue Definition - SQL-based value for inserts
//------------------------------------------------------------------------------

use std::marker::PhantomData;

use drizzle_core::{Placeholder, SQL, SQLParam};

use crate::{OwnedSQLiteValue, SQLiteValue};

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
pub enum SQLiteInsertValue<'a, V: SQLParam, T> {
    /// Omit this column from the INSERT (use database default)
    #[default]
    Omit,
    /// Explicitly insert NULL
    Null,
    /// Insert a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, T> SQLiteInsertValue<'a, SQLiteValue<'a>, T> {
    /// Converts this InsertValue to an owned version with 'static lifetime
    pub fn into_owned(self) -> SQLiteInsertValue<'static, SQLiteValue<'static>, T> {
        match self {
            SQLiteInsertValue::Omit => SQLiteInsertValue::Omit,
            SQLiteInsertValue::Null => SQLiteInsertValue::Null,
            SQLiteInsertValue::Value(wrapper) => {
                // Extract the parameter value, convert to owned, then back to static SQLiteValue
                if let Some(drizzle_core::SQLChunk::Param(param)) = wrapper.value.chunks.first() {
                    if let Some(ref val) = param.value {
                        let owned_val = OwnedSQLiteValue::from(val.as_ref().clone());
                        let static_val: SQLiteValue<'static> = owned_val.into();
                        let static_sql = drizzle_core::SQL::param(static_val);
                        SQLiteInsertValue::Value(ValueWrapper::<SQLiteValue<'static>, T>::new(
                            static_sql,
                        ))
                    } else {
                        SQLiteInsertValue::Value(ValueWrapper::<SQLiteValue<'static>, T>::new(
                            drizzle_core::SQL::param(SQLiteValue::Null),
                        ))
                    }
                } else {
                    SQLiteInsertValue::Value(ValueWrapper::<SQLiteValue<'static>, T>::new(
                        drizzle_core::SQL::param(SQLiteValue::Null),
                    ))
                }
            }
        }
    }
}

impl<'a, T, U> From<T> for SQLiteInsertValue<'a, SQLiteValue<'a>, U>
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
        SQLiteInsertValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(sql))
    }
}

impl<'a, T> From<Placeholder> for SQLiteInsertValue<'a, SQLiteValue<'a>, T> {
    fn from(placeholder: Placeholder) -> Self {
        use drizzle_core::{Param, SQLChunk};
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        SQLiteInsertValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(
            std::iter::once(chunk).collect(),
        ))
    }
}

// Array conversion for Vec<u8> InsertValue - support flexible input types
impl<'a, const N: usize> From<[u8; N]> for SQLiteInsertValue<'a, SQLiteValue<'a>, Vec<u8>> {
    fn from(value: [u8; N]) -> Self {
        let sqlite_value = SQLiteValue::Blob(std::borrow::Cow::Owned(value.to_vec()));
        let sql = SQL::param(sqlite_value);
        SQLiteInsertValue::Value(ValueWrapper::<SQLiteValue<'a>, Vec<u8>>::new(sql))
    }
}
