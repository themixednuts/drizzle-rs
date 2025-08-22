//! SQLite value conversion traits and types

use drizzle_core::{Placeholder, SQL, SQLParam, ToSQL, error::DrizzleError};

mod owned;
pub use owned::OwnedSQLiteValue;

#[cfg(feature = "rusqlite")]
use rusqlite::types::FromSql;
#[cfg(feature = "turso")]
use turso::IntoValue;
#[cfg(feature = "uuid")]
use uuid::Uuid;

use std::borrow::Cow;
use std::marker::PhantomData;

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
pub enum InsertValue<'a, V: SQLParam, T> {
    /// Omit this column from the INSERT (use database default)
    #[default]
    Omit,
    /// Explicitly insert NULL
    Null,
    /// Insert a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, T> InsertValue<'a, SQLiteValue<'a>, T> {
    /// Converts this InsertValue to an owned version with 'static lifetime
    pub fn into_owned(self) -> InsertValue<'static, SQLiteValue<'static>, T> {
        match self {
            InsertValue::Omit => InsertValue::Omit,
            InsertValue::Null => InsertValue::Null,
            InsertValue::Value(wrapper) => {
                // Extract the parameter value, convert to owned, then back to static SQLiteValue
                if let Some(drizzle_core::SQLChunk::Param(param)) = wrapper.value.chunks.first() {
                    if let Some(ref val) = param.value {
                        let owned_val = OwnedSQLiteValue::from(val.as_ref().clone());
                        let static_val: SQLiteValue<'static> = owned_val.into();
                        let static_sql = drizzle_core::SQL::parameter(static_val);
                        InsertValue::Value(ValueWrapper::<SQLiteValue<'static>, T>::new(static_sql))
                    } else {
                        InsertValue::Value(ValueWrapper::<SQLiteValue<'static>, T>::new(
                            drizzle_core::SQL::parameter(SQLiteValue::Null),
                        ))
                    }
                } else {
                    InsertValue::Value(ValueWrapper::<SQLiteValue<'static>, T>::new(
                        drizzle_core::SQL::parameter(SQLiteValue::Null),
                    ))
                }
            }
        }
    }
}

// Conversion implementations for SQLiteValue-based InsertValue

// Generic conversion from any type T to InsertValue (for same type T)
impl<'a, T> From<T> for InsertValue<'a, SQLiteValue<'a>, T>
where
    T: TryInto<SQLiteValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = value
            .try_into()
            .map(|v: SQLiteValue<'a>| SQL::from(v))
            .unwrap_or_else(|_| SQL::from(SQLiteValue::Null));
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(sql))
    }
}

// Specific conversion for &str to String InsertValue
impl<'a> From<&str> for InsertValue<'a, SQLiteValue<'a>, String> {
    fn from(value: &str) -> Self {
        let sqlite_value = SQL::parameter(Cow::Owned(SQLiteValue::from(value.to_string())));
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, String>::new(sqlite_value))
    }
}

// Placeholder conversion for OwnedSQLiteValue
impl<'a, T> From<Placeholder> for InsertValue<'a, SQLiteValue<'a>, T> {
    fn from(placeholder: Placeholder) -> Self {
        // For now, placeholders become Null values in owned context
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(
            SQL::from_placeholder(placeholder),
        ))
    }
}

// Option conversion for OwnedSQLiteValue
impl<'a, T> From<Option<T>> for InsertValue<'a, SQLiteValue<'a>, T>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => {
                let sql = v.to_sql();
                InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(sql))
            }
            None => InsertValue::Omit,
        }
    }
}

impl<'a> FromIterator<OwnedSQLiteValue> for Vec<SQLiteValue<'a>> {
    fn from_iter<T: IntoIterator<Item = OwnedSQLiteValue>>(iter: T) -> Self {
        iter.into_iter().map(SQLiteValue::from).collect()
    }
}

impl<'a> FromIterator<&'a OwnedSQLiteValue> for Vec<SQLiteValue<'a>> {
    fn from_iter<T: IntoIterator<Item = &'a OwnedSQLiteValue>>(iter: T) -> Self {
        iter.into_iter().map(SQLiteValue::from).collect()
    }
}

// UUID conversion for String InsertValue (for text columns)
#[cfg(feature = "uuid")]
impl<'a> From<Uuid> for InsertValue<'a, SQLiteValue<'a>, String> {
    fn from(value: Uuid) -> Self {
        let sqlite_value = SQLiteValue::Text(std::borrow::Cow::Owned(value.to_string()));
        let sql = SQL::parameter(sqlite_value);
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, String>::new(sql))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for InsertValue<'a, SQLiteValue<'a>, String> {
    fn from(value: &'a Uuid) -> Self {
        let sqlite_value = SQLiteValue::Text(std::borrow::Cow::Owned(value.to_string()));
        let sql = SQL::parameter(sqlite_value);
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, String>::new(sql))
    }
}

// Array conversion for Vec<u8> InsertValue - support flexible input types
impl<'a, const N: usize> From<[u8; N]> for InsertValue<'a, SQLiteValue<'a>, Vec<u8>> {
    fn from(value: [u8; N]) -> Self {
        let sqlite_value = SQLiteValue::Blob(std::borrow::Cow::Owned(value.to_vec()));
        let sql = SQL::parameter(sqlite_value);
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, Vec<u8>>::new(sql))
    }
}

// Slice conversion for Vec<u8> InsertValue
impl<'a> From<&'a [u8]> for InsertValue<'a, SQLiteValue<'a>, Vec<u8>> {
    fn from(value: &'a [u8]) -> Self {
        let sqlite_value = SQLiteValue::Blob(std::borrow::Cow::Borrowed(value));
        let sql = SQL::parameter(sqlite_value);
        InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, Vec<u8>>::new(sql))
    }
}

// Vec<u8> conversion for Vec<u8> InsertValue
// impl<'a> From<Vec<u8>> for InsertValue<'a, SQLiteValue<'a>, Vec<u8>> {
//     fn from(value: Vec<u8>) -> Self {
//         let sqlite_value = SQLiteValue::Blob(std::borrow::Cow::Owned(value));
//         let sql = SQL::parameter(sqlite_value);
//         InsertValue::Value(ValueWrapper::<SQLiteValue<'a>, Vec<u8>>::new(sql))
//     }
// }

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
impl<'a> From<OwnedSQLiteValue> for SQLiteValue<'a> {
    fn from(value: OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(f) => SQLiteValue::Integer(f),
            OwnedSQLiteValue::Real(r) => SQLiteValue::Real(r),
            OwnedSQLiteValue::Text(v) => SQLiteValue::Text(Cow::Owned(v)),
            OwnedSQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Owned(v.into())),
            OwnedSQLiteValue::Null => SQLiteValue::Null,
        }
    }
}
impl<'a> From<&'a OwnedSQLiteValue> for SQLiteValue<'a> {
    fn from(value: &'a OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(f) => SQLiteValue::Integer(*f),
            OwnedSQLiteValue::Real(r) => SQLiteValue::Real(*r),
            OwnedSQLiteValue::Text(v) => SQLiteValue::Text(Cow::Borrowed(v)),
            OwnedSQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Borrowed(v)),
            OwnedSQLiteValue::Null => SQLiteValue::Null,
        }
    }
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
    fn from(_value: SQL<'a, SQLiteValue<'a>>) -> Self {
        unimplemented!()
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

#[cfg(feature = "rusqlite")]
impl<'a> From<rusqlite::types::Value> for SQLiteValue<'a> {
    fn from(value: rusqlite::types::Value) -> Self {
        match value {
            rusqlite::types::Value::Null => SQLiteValue::Null,
            rusqlite::types::Value::Integer(i) => SQLiteValue::Integer(i),
            rusqlite::types::Value::Real(r) => SQLiteValue::Real(r),
            rusqlite::types::Value::Text(s) => SQLiteValue::Text(s.into()),
            rusqlite::types::Value::Blob(b) => SQLiteValue::Blob(b.into()),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl<'a> From<rusqlite::types::ValueRef<'a>> for SQLiteValue<'a> {
    fn from(value: rusqlite::types::ValueRef<'a>) -> Self {
        match value {
            rusqlite::types::ValueRef::Null => SQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => SQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => SQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                SQLiteValue::Text(String::from_utf8_lossy(items).into_owned().into())
            }
            rusqlite::types::ValueRef::Blob(items) => SQLiteValue::Blob(items.to_vec().into()),
        }
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
impl<'a> From<SQLiteValue<'a>> for turso::Value {
    fn from(value: SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => turso::Value::Integer(i),
            SQLiteValue::Real(r) => turso::Value::Real(r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => turso::Value::Null,
        }
    }
}

#[cfg(feature = "turso")]
impl<'a> From<&SQLiteValue<'a>> for turso::Value {
    fn from(value: &SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => turso::Value::Integer(*i),
            SQLiteValue::Real(r) => turso::Value::Real(*r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => turso::Value::Null,
        }
    }
}

#[cfg(feature = "libsql")]
impl<'a> From<SQLiteValue<'a>> for libsql::Value {
    fn from(value: SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => libsql::Value::Integer(i),
            SQLiteValue::Real(r) => libsql::Value::Real(r),
            SQLiteValue::Text(cow) => libsql::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => libsql::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => libsql::Value::Null,
        }
    }
}

#[cfg(feature = "libsql")]
impl<'a> From<&SQLiteValue<'a>> for libsql::Value {
    fn from(value: &SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => libsql::Value::Integer(*i),
            SQLiteValue::Real(r) => libsql::Value::Real(*r),
            SQLiteValue::Text(cow) => libsql::Value::Text(cow.to_string()),
            SQLiteValue::Blob(cow) => libsql::Value::Blob(cow.to_vec()),
            SQLiteValue::Null => libsql::Value::Null,
        }
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

impl<'a> TryFrom<SQLiteValue<'a>> for i32 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i32",
                value
            ))),
        }
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
impl<'a> From<Uuid> for SQLiteValue<'a> {
    fn from(value: Uuid) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_bytes().to_vec()))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for SQLiteValue<'a> {
    fn from(value: &'a Uuid) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_bytes()))
    }
}

// --- JSON ---
// Note: JSON types should be handled through serialization in the schema macros
// These implementations provide fallback support but JSON fields should primarily
// be serialized to TEXT or BLOB through the field generation logic

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

//------------------------------------------------------------------------------
// TryFrom<SQLiteValue> implementations
//------------------------------------------------------------------------------

// --- Integer Types ---

impl<'a> TryFrom<SQLiteValue<'a>> for i8 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i8",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for i16 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i16",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for i64 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i64",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for isize {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to isize",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for u8 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u8",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for u16 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u16",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for u32 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u32",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for u64 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u64",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for usize {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to usize",
                value
            ))),
        }
    }
}

// --- Floating Point Types ---

impl<'a> TryFrom<SQLiteValue<'a>> for f32 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Real(f) => Ok(f as f32),
            SQLiteValue::Integer(i) => Ok(i as f32),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f32",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<SQLiteValue<'a>> for f64 {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Real(f) => Ok(f),
            SQLiteValue::Integer(i) => Ok(i as f64),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f64",
                value
            ))),
        }
    }
}

// --- Boolean ---

impl<'a> TryFrom<SQLiteValue<'a>> for bool {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(0) => Ok(false),
            SQLiteValue::Integer(_) => Ok(true),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}

// --- String Types ---

impl<'a> TryFrom<SQLiteValue<'a>> for String {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Text(cow) => Ok(cow.into_owned()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to String",
                value
            ))),
        }
    }
}

// --- Binary Data ---

impl<'a> TryFrom<SQLiteValue<'a>> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => Ok(cow.into_owned()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to Vec<u8>",
                value
            ))),
        }
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> TryFrom<SQLiteValue<'a>> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => {
                let bytes: [u8; 16] = cow.as_ref().try_into().map_err(|_| {
                    DrizzleError::ConversionError("UUID blob must be exactly 16 bytes".to_string())
                })?;
                Ok(Uuid::from_bytes(bytes))
            }
            SQLiteValue::Text(cow) => Ok(Uuid::parse_str(cow.as_ref())?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to UUID",
                value
            ))),
        }
    }
}

//------------------------------------------------------------------------------
// TryFrom<&SQLiteValue> implementations for borrowing without consuming
//------------------------------------------------------------------------------

// --- Integer Types ---

impl<'a> TryFrom<&SQLiteValue<'a>> for i8 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i8",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for i16 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i16",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for i32 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i32",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for i64 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok(*i),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i64",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for isize {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to isize",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for u8 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u8",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for u16 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u16",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for u32 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u32",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for u64 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u64",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for usize {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to usize",
                value
            ))),
        }
    }
}

// --- Floating Point Types ---

impl<'a> TryFrom<&SQLiteValue<'a>> for f32 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Real(f) => Ok(*f as f32),
            SQLiteValue::Integer(i) => Ok(*i as f32),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f32",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&SQLiteValue<'a>> for f64 {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Real(f) => Ok(*f),
            SQLiteValue::Integer(i) => Ok(*i as f64),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f64",
                value
            ))),
        }
    }
}

// --- Boolean ---

impl<'a> TryFrom<&SQLiteValue<'a>> for bool {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Integer(0) => Ok(false),
            SQLiteValue::Integer(_) => Ok(true),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}

// --- String Types ---

impl<'a> TryFrom<&SQLiteValue<'a>> for String {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Text(cow) => Ok(cow.to_string()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to String",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Text(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to &str",
                value
            ))),
        }
    }
}

// --- Binary Data ---

impl<'a> TryFrom<&SQLiteValue<'a>> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => Ok(cow.to_vec()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to Vec<u8>",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to &[u8]",
                value
            ))),
        }
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> TryFrom<&SQLiteValue<'a>> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => {
                let bytes: [u8; 16] = cow.as_ref().try_into().map_err(|_| {
                    DrizzleError::ConversionError("UUID blob must be exactly 16 bytes".to_string())
                })?;
                Ok(Uuid::from_bytes(bytes))
            }
            SQLiteValue::Text(cow) => Ok(Uuid::parse_str(cow.as_ref())?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to UUID",
                value
            ))),
        }
    }
}
