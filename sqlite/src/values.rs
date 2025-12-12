//! SQLite value conversion traits and types

use crate::traits::FromSQLiteValue;
use drizzle_core::{Dialect, SQL, SQLParam, ToSQL, error::DrizzleError};

mod insert;
mod owned;
pub use insert::*;
pub use owned::OwnedSQLiteValue;

#[cfg(feature = "rusqlite")]
use rusqlite::types::FromSql;
#[cfg(feature = "turso")]
use turso::IntoValue;
#[cfg(feature = "uuid")]
use uuid::Uuid;

use std::borrow::Cow;

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

impl<'a> SQLiteValue<'a> {
    /// Convert this SQLite value to a Rust type using the `FromSQLiteValue` trait.
    ///
    /// This provides a unified conversion interface for all types that implement
    /// `FromSQLiteValue`, including primitives and enum types.
    ///
    /// # Example
    /// ```ignore
    /// let value = SQLiteValue::Integer(42);
    /// let num: i64 = value.convert()?;
    /// ```
    pub fn convert<T: FromSQLiteValue>(self) -> Result<T, DrizzleError> {
        match self {
            SQLiteValue::Integer(i) => T::from_sqlite_integer(i),
            SQLiteValue::Text(s) => T::from_sqlite_text(&s),
            SQLiteValue::Real(r) => T::from_sqlite_real(r),
            SQLiteValue::Blob(b) => T::from_sqlite_blob(&b),
            SQLiteValue::Null => T::from_sqlite_null(),
        }
    }

    /// Convert a reference to this SQLite value to a Rust type.
    pub fn convert_ref<T: FromSQLiteValue>(&self) -> Result<T, DrizzleError> {
        match self {
            SQLiteValue::Integer(i) => T::from_sqlite_integer(*i),
            SQLiteValue::Text(s) => T::from_sqlite_text(s),
            SQLiteValue::Real(r) => T::from_sqlite_real(*r),
            SQLiteValue::Blob(b) => T::from_sqlite_blob(b),
            SQLiteValue::Null => T::from_sqlite_null(),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for SQLiteValue<'a> {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        SQL::param(self.clone())
    }
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
impl<'a> From<&'a SQLiteValue<'a>> for SQLiteValue<'a> {
    fn from(value: &'a SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(f) => SQLiteValue::Integer(*f),
            SQLiteValue::Real(r) => SQLiteValue::Real(*r),
            SQLiteValue::Text(v) => SQLiteValue::Text(Cow::Borrowed(v)),
            SQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Borrowed(v)),
            SQLiteValue::Null => SQLiteValue::Null,
        }
    }
}
impl<'a> From<Cow<'a, SQLiteValue<'a>>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, SQLiteValue<'a>>) -> Self {
        match value {
            Cow::Borrowed(r) => r.into(),
            Cow::Owned(o) => o,
        }
    }
}
impl<'a> From<&'a Cow<'a, SQLiteValue<'a>>> for SQLiteValue<'a> {
    fn from(value: &'a Cow<'a, SQLiteValue<'a>>) -> Self {
        match value {
            Cow::Borrowed(r) => (*r).into(),
            Cow::Owned(o) => o.into(),
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
            SQLiteValue::Text(cow) => turso::Value::Text(cow.into_owned()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.into_owned()),
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
            SQLiteValue::Text(cow) => libsql::Value::Text(cow.into_owned()),
            SQLiteValue::Blob(cow) => libsql::Value::Blob(cow.into_owned()),
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
impl<'a> SQLParam for SQLiteValue<'a> {
    const DIALECT: Dialect = Dialect::SQLite;
}

impl<'a> From<SQLiteValue<'a>> for SQL<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        SQL::param(value)
    }
}

//------------------------------------------------------------------------------
// From<T> implementations
// Macro-based to reduce boilerplate
//------------------------------------------------------------------------------

/// Macro to implement From<integer> for SQLiteValue (converts to INTEGER)
macro_rules! impl_from_int_for_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> From<$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: $ty) -> Self {
                    SQLiteValue::Integer(value as i64)
                }
            }

            impl<'a> From<&$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: &$ty) -> Self {
                    SQLiteValue::Integer(*value as i64)
                }
            }
        )*
    };
}

/// Macro to implement From<float> for SQLiteValue (converts to REAL)
macro_rules! impl_from_float_for_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> From<$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: $ty) -> Self {
                    SQLiteValue::Real(value as f64)
                }
            }

            impl<'a> From<&$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: &$ty) -> Self {
                    SQLiteValue::Real(*value as f64)
                }
            }
        )*
    };
}

// Integer types -> SQLiteValue::Integer
impl_from_int_for_sqlite_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

// Float types -> SQLiteValue::Real
impl_from_float_for_sqlite_value!(f32, f64);

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

// --- ArrayString ---

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<arrayvec::ArrayString<N>> for SQLiteValue<'a> {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&arrayvec::ArrayString<N>> for SQLiteValue<'a> {
    fn from(value: &arrayvec::ArrayString<N>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_str().to_owned()))
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

// --- ArrayVec<u8, N> ---

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<arrayvec::ArrayVec<u8, N>> for SQLiteValue<'a> {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&arrayvec::ArrayVec<u8, N>> for SQLiteValue<'a> {
    fn from(value: &arrayvec::ArrayVec<u8, N>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
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
// Uses the FromSQLiteValue trait via convert() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<SQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> TryFrom<SQLiteValue<'a>> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
                    value.convert()
                }
            }
        )*
    };
}

impl_try_from_sqlite_value!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    f32,
    f64,
    bool,
    String,
    Vec<u8>,
);

#[cfg(feature = "uuid")]
impl_try_from_sqlite_value!(Uuid);

//------------------------------------------------------------------------------
// TryFrom<&SQLiteValue> implementations for borrowing without consuming
// Uses the FromSQLiteValue trait via convert_ref() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<&SQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_sqlite_value_ref {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> TryFrom<&SQLiteValue<'a>> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
                    value.convert_ref()
                }
            }
        )*
    };
}

impl_try_from_sqlite_value_ref!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    f32,
    f64,
    bool,
    String,
    Vec<u8>,
);

#[cfg(feature = "uuid")]
impl_try_from_sqlite_value_ref!(Uuid);

// --- Borrowed reference types (cannot use FromSQLiteValue) ---

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Text(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &str", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &[u8]", value).into(),
            )),
        }
    }
}
