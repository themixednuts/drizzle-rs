//! SQLite value conversion traits and types

use drizzle_core::SQL;

#[cfg(feature = "rusqlite")]
use rusqlite::types::FromSql;
#[cfg(feature = "turso")]
use turso::IntoValue;

use std::borrow::Cow;

//------------------------------------------------------------------------------
// InsertValue Definition - Three-state value for inserts
//------------------------------------------------------------------------------

/// Represents a value for INSERT operations that can be omitted, null, or a specific value
#[derive(Debug, Clone, PartialEq)]
pub enum InsertValue<T> {
    /// Omit this column from the INSERT (use database default)
    Omit,
    /// Explicitly insert NULL
    Null,
    /// Insert this specific value
    Value(T),
}

impl<T> Default for InsertValue<T> {
    fn default() -> Self {
        Self::Omit
    }
}

// Automatic conversion from T to InsertValue<T>
impl<T> From<T> for InsertValue<T> {
    fn from(value: T) -> Self {
        InsertValue::Value(value)
    }
}

// Conversion from Option<T>
impl<T> From<Option<T>> for InsertValue<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => InsertValue::Value(v),
            None => InsertValue::Omit, // Use database default when None
        }
    }
}

// String-specific InsertValue conversions
impl From<&str> for InsertValue<String> {
    fn from(value: &str) -> Self {
        InsertValue::Value(value.to_string())
    }
}

impl From<Option<&str>> for InsertValue<String> {
    fn from(value: Option<&str>) -> Self {
        match value {
            Some(s) => InsertValue::Value(s.to_string()),
            None => InsertValue::Omit, // Use database default when None
        }
    }
}

//------------------------------------------------------------------------------
// SQLiteValue Definition
//------------------------------------------------------------------------------

/// Represents a SQLite value
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum SQLiteValue<'a> {
    /// Integer value (i64)
    Integer(i64),
    /// Real value (f64)
    Real(f64),
    /// Text value (borrowed or owned string)
    Text(Cow<'a, str>),
    /// Blob value (borrowed or owned binary data)
    Blob(Cow<'a, [u8]>),
    /// NULL value
    #[default]
    Null,
}

impl<'a> std::fmt::Display for SQLiteValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SQLiteValue::Integer(i) => i.to_string(),
            SQLiteValue::Real(r) => r.to_string(),
            SQLiteValue::Text(cow) => cow.to_string(),
            SQLiteValue::Blob(cow) => String::from_utf8_lossy(cow).to_string(),
            SQLiteValue::Null => String::new(),
        };
        write!(f, "{value}")
    }
}

impl<'a> From<SQL<'a, SQLiteValue<'a>>> for SQLiteValue<'a> {
    fn from(value: SQL<'a, SQLiteValue<'a>>) -> Self {
        todo!()
    }
}

//------------------------------------------------------------------------------
// Database Driver Implementations
//------------------------------------------------------------------------------

// Implement rusqlite::ToSql for SQLiteValue when the rusqlite feature is enabled
#[cfg(feature = "rusqlite")]
impl<'a> rusqlite::ToSql for SQLiteValue<'a> {
    fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SQLiteValue::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Null,
            )),
            SQLiteValue::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(*i),
            )),
            SQLiteValue::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Real(*f),
            )),
            SQLiteValue::Text(s) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(s.as_bytes()),
            )),
            SQLiteValue::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Blob(b.as_ref()),
            )),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl<'a> FromSql for SQLiteValue<'a> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let result = match value {
            rusqlite::types::ValueRef::Null => SQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => SQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => SQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                SQLiteValue::Text(String::from_utf8_lossy(items).into_owned().into())
            }
            rusqlite::types::ValueRef::Blob(items) => SQLiteValue::Blob(items.to_vec().into()),
        };
        Ok(result)
    }
}

#[cfg(feature = "turso")]
impl<'a> IntoValue for SQLiteValue<'a> {
    fn into_value(self) -> turso::Result<turso::Value> {
        let result = match self {
            SQLiteValue::Integer(i) => turso::Value::Integer(i),
            SQLiteValue::Real(r) => turso::Value::Real(r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.into()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.into()),
            SQLiteValue::Null => turso::Value::Null,
        };
        Ok(result)
    }
}

#[cfg(feature = "turso")]
impl<'a> IntoValue for &SQLiteValue<'a> {
    fn into_value(self) -> turso::Result<turso::Value> {
        let result = match self {
            SQLiteValue::Integer(i) => turso::Value::Integer(*i),
            SQLiteValue::Real(r) => turso::Value::Real(*r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => turso::Value::Null,
        };
        Ok(result)
    }
}

#[cfg(feature = "turso")]
impl<'a> TryFrom<SQLiteValue<'a>> for turso::Value {
    type Error = turso::Error;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        let result = match value {
            SQLiteValue::Integer(i) => turso::Value::Integer(i),
            SQLiteValue::Real(r) => turso::Value::Real(r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => turso::Value::Null,
        };
        Ok(result)
    }
}

#[cfg(feature = "turso")]
impl<'a> TryFrom<&SQLiteValue<'a>> for turso::Value {
    type Error = turso::Error;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        let result = match value {
            SQLiteValue::Integer(i) => turso::Value::Integer(*i),
            SQLiteValue::Real(r) => turso::Value::Real(*r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => turso::Value::Null,
        };
        Ok(result)
    }
}

// Implement core traits required by Drizzle
impl<'a> drizzle_core::traits::SQLParam for SQLiteValue<'a> {}

impl<'a> From<SQLiteValue<'a>> for SQL<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        SQL::parameter(value)
    }
}

//------------------------------------------------------------------------------
// From<T> implementations
//------------------------------------------------------------------------------

// --- Integer Types ---

// i8
impl<'a> From<i8> for SQLiteValue<'a> {
    fn from(value: i8) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a i8> for SQLiteValue<'a> {
    fn from(value: &'a i8) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// i16
impl<'a> From<i16> for SQLiteValue<'a> {
    fn from(value: i16) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a i16> for SQLiteValue<'a> {
    fn from(value: &'a i16) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// i32
impl<'a> From<i32> for SQLiteValue<'a> {
    fn from(value: i32) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a i32> for SQLiteValue<'a> {
    fn from(value: &'a i32) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// i64
impl<'a> From<i64> for SQLiteValue<'a> {
    fn from(value: i64) -> Self {
        SQLiteValue::Integer(value)
    }
}

impl<'a> From<&'a i64> for SQLiteValue<'a> {
    fn from(value: &'a i64) -> Self {
        SQLiteValue::Integer(*value)
    }
}

// isize
impl<'a> From<isize> for SQLiteValue<'a> {
    fn from(value: isize) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a isize> for SQLiteValue<'a> {
    fn from(value: &'a isize) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// u8
impl<'a> From<u8> for SQLiteValue<'a> {
    fn from(value: u8) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a u8> for SQLiteValue<'a> {
    fn from(value: &'a u8) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// u16
impl<'a> From<u16> for SQLiteValue<'a> {
    fn from(value: u16) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a u16> for SQLiteValue<'a> {
    fn from(value: &'a u16) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// u32
impl<'a> From<u32> for SQLiteValue<'a> {
    fn from(value: u32) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a u32> for SQLiteValue<'a> {
    fn from(value: &'a u32) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// u64
impl<'a> From<u64> for SQLiteValue<'a> {
    fn from(value: u64) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a u64> for SQLiteValue<'a> {
    fn from(value: &'a u64) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// usize
impl<'a> From<usize> for SQLiteValue<'a> {
    fn from(value: usize) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a usize> for SQLiteValue<'a> {
    fn from(value: &'a usize) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// impl<'a, T> From<T> for SQLiteValue<'a>
// where
//     T: SQLEnum<Type = SQLiteEnumRepr>,
// {
//     fn from(value: T) -> Self {
//         todo!()
//     }
// }

// --- Floating Point Types ---

// f32
impl<'a> From<f32> for SQLiteValue<'a> {
    fn from(value: f32) -> Self {
        SQLiteValue::Real(value as f64)
    }
}

impl<'a> From<&'a f32> for SQLiteValue<'a> {
    fn from(value: &'a f32) -> Self {
        SQLiteValue::Real(*value as f64)
    }
}

// f64
impl<'a> From<f64> for SQLiteValue<'a> {
    fn from(value: f64) -> Self {
        SQLiteValue::Real(value)
    }
}

impl<'a> From<&'a f64> for SQLiteValue<'a> {
    fn from(value: &'a f64) -> Self {
        SQLiteValue::Real(*value)
    }
}

// --- Boolean ---

impl<'a> From<bool> for SQLiteValue<'a> {
    fn from(value: bool) -> Self {
        SQLiteValue::Integer(value as i64)
    }
}

impl<'a> From<&'a bool> for SQLiteValue<'a> {
    fn from(value: &'a bool) -> Self {
        SQLiteValue::Integer(*value as i64)
    }
}

// --- String Types ---

impl<'a> From<&'a str> for SQLiteValue<'a> {
    fn from(value: &'a str) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for SQLiteValue<'a> {
    fn from(value: String) -> Self {
        SQLiteValue::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a String> for SQLiteValue<'a> {
    fn from(value: &'a String) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value))
    }
}

// --- Binary Data ---

impl<'a> From<&'a [u8]> for SQLiteValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value))
    }
}

impl<'a> From<Vec<u8>> for SQLiteValue<'a> {
    fn from(value: Vec<u8>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value))
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> From<uuid::Uuid> for SQLiteValue<'a> {
    fn from(value: uuid::Uuid) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_bytes().to_vec()))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a uuid::Uuid> for SQLiteValue<'a> {
    fn from(value: &'a uuid::Uuid) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_bytes()))
    }
}

// --- Option Types ---
impl<'a, T> From<Option<T>> for SQLiteValue<'a>
where
    T: TryInto<SQLiteValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.try_into().unwrap_or(SQLiteValue::Null),
            None => SQLiteValue::Null,
        }
    }
}

// --- Cow integration for SQL struct ---
impl<'a> From<SQLiteValue<'a>> for Cow<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a SQLiteValue<'a>> for Cow<'a, SQLiteValue<'a>> {
    fn from(value: &'a SQLiteValue<'a>) -> Self {
        Cow::Borrowed(value)
    }
}
