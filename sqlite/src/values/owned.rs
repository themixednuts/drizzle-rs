//! Owned SQLite value type and implementations

use crate::SQLiteValue;
use crate::traits::FromSQLiteValue;
use drizzle_core::{SQL, SQLParam, error::DrizzleError};

#[cfg(feature = "rusqlite")]
use rusqlite::types::FromSql;
#[cfg(feature = "turso")]
use turso::IntoValue;
#[cfg(feature = "uuid")]
use uuid::Uuid;

use std::borrow::Cow;

/// Represents a SQLite value (owned version)
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum OwnedSQLiteValue {
    /// Integer value (i64)
    Integer(i64),
    /// Real value (f64)
    Real(f64),
    /// Text value (owned string)
    Text(String),
    /// Blob value (owned binary data)
    Blob(Box<[u8]>),
    /// NULL value
    #[default]
    Null,
}

impl OwnedSQLiteValue {
    /// Convert this SQLite value to a Rust type using the `FromSQLiteValue` trait.
    ///
    /// This provides a unified conversion interface for all types that implement
    /// `FromSQLiteValue`, including primitives and enum types.
    pub fn convert<T: FromSQLiteValue>(self) -> Result<T, DrizzleError> {
        match self {
            OwnedSQLiteValue::Integer(i) => T::from_sqlite_integer(i),
            OwnedSQLiteValue::Text(s) => T::from_sqlite_text(&s),
            OwnedSQLiteValue::Real(r) => T::from_sqlite_real(r),
            OwnedSQLiteValue::Blob(b) => T::from_sqlite_blob(&b),
            OwnedSQLiteValue::Null => T::from_sqlite_null(),
        }
    }

    /// Convert a reference to this SQLite value to a Rust type.
    pub fn convert_ref<T: FromSQLiteValue>(&self) -> Result<T, DrizzleError> {
        match self {
            OwnedSQLiteValue::Integer(i) => T::from_sqlite_integer(*i),
            OwnedSQLiteValue::Text(s) => T::from_sqlite_text(s),
            OwnedSQLiteValue::Real(r) => T::from_sqlite_real(*r),
            OwnedSQLiteValue::Blob(b) => T::from_sqlite_blob(b),
            OwnedSQLiteValue::Null => T::from_sqlite_null(),
        }
    }
}

impl<'a> From<SQLiteValue<'a>> for OwnedSQLiteValue {
    fn from(value: SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => Self::Integer(i),
            SQLiteValue::Real(r) => Self::Real(r),
            SQLiteValue::Text(cow) => Self::Text(cow.into_owned()),
            SQLiteValue::Blob(cow) => Self::Blob(cow.into_owned().into_boxed_slice()),
            SQLiteValue::Null => Self::Null,
        }
    }
}
impl<'a> From<&SQLiteValue<'a>> for OwnedSQLiteValue {
    fn from(value: &SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => Self::Integer(*i),
            SQLiteValue::Real(r) => Self::Real(*r),
            SQLiteValue::Text(cow) => Self::Text(cow.clone().into_owned()),
            SQLiteValue::Blob(cow) => Self::Blob(cow.clone().into_owned().into_boxed_slice()),
            SQLiteValue::Null => Self::Null,
        }
    }
}

impl std::fmt::Display for OwnedSQLiteValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            OwnedSQLiteValue::Integer(i) => i.to_string(),
            OwnedSQLiteValue::Real(r) => r.to_string(),
            OwnedSQLiteValue::Text(s) => s.clone(),
            OwnedSQLiteValue::Blob(b) => String::from_utf8_lossy(b).to_string(),
            OwnedSQLiteValue::Null => String::new(),
        };
        write!(f, "{value}")
    }
}

//------------------------------------------------------------------------------
// Database Driver Implementations
//------------------------------------------------------------------------------

// Implement rusqlite::ToSql for OwnedSQLiteValue when the rusqlite feature is enabled
#[cfg(feature = "rusqlite")]
impl rusqlite::ToSql for OwnedSQLiteValue {
    fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
        match self {
            OwnedSQLiteValue::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Null,
            )),
            OwnedSQLiteValue::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(*i),
            )),
            OwnedSQLiteValue::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Real(*f),
            )),
            OwnedSQLiteValue::Text(s) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(s.as_bytes()),
            )),
            OwnedSQLiteValue::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Blob(b.as_ref()),
            )),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl FromSql for OwnedSQLiteValue {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let result = match value {
            rusqlite::types::ValueRef::Null => OwnedSQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => OwnedSQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => OwnedSQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                OwnedSQLiteValue::Text(String::from_utf8_lossy(items).into_owned())
            }
            rusqlite::types::ValueRef::Blob(items) => {
                OwnedSQLiteValue::Blob(items.to_vec().into_boxed_slice())
            }
        };
        Ok(result)
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::types::Value> for OwnedSQLiteValue {
    fn from(value: rusqlite::types::Value) -> Self {
        match value {
            rusqlite::types::Value::Null => OwnedSQLiteValue::Null,
            rusqlite::types::Value::Integer(i) => OwnedSQLiteValue::Integer(i),
            rusqlite::types::Value::Real(r) => OwnedSQLiteValue::Real(r),
            rusqlite::types::Value::Text(s) => OwnedSQLiteValue::Text(s),
            rusqlite::types::Value::Blob(b) => OwnedSQLiteValue::Blob(b.into_boxed_slice()),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::types::ValueRef<'_>> for OwnedSQLiteValue {
    fn from(value: rusqlite::types::ValueRef<'_>) -> Self {
        match value {
            rusqlite::types::ValueRef::Null => OwnedSQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => OwnedSQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => OwnedSQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                OwnedSQLiteValue::Text(String::from_utf8_lossy(items).into_owned())
            }
            rusqlite::types::ValueRef::Blob(items) => {
                OwnedSQLiteValue::Blob(items.to_vec().into_boxed_slice())
            }
        }
    }
}

#[cfg(feature = "turso")]
impl IntoValue for OwnedSQLiteValue {
    fn into_value(self) -> turso::Result<turso::Value> {
        let result = match self {
            OwnedSQLiteValue::Integer(i) => turso::Value::Integer(i),
            OwnedSQLiteValue::Real(r) => turso::Value::Real(r),
            OwnedSQLiteValue::Text(s) => turso::Value::Text(s),
            OwnedSQLiteValue::Blob(b) => turso::Value::Blob(b.into_vec()),
            OwnedSQLiteValue::Null => turso::Value::Null,
        };
        Ok(result)
    }
}

#[cfg(feature = "turso")]
impl IntoValue for &OwnedSQLiteValue {
    fn into_value(self) -> turso::Result<turso::Value> {
        let result = match self {
            OwnedSQLiteValue::Integer(i) => turso::Value::Integer(*i),
            OwnedSQLiteValue::Real(r) => turso::Value::Real(*r),
            OwnedSQLiteValue::Text(s) => turso::Value::Text(s.clone()),
            OwnedSQLiteValue::Blob(b) => turso::Value::Blob(b.to_vec()),
            OwnedSQLiteValue::Null => turso::Value::Null,
        };
        Ok(result)
    }
}

#[cfg(feature = "turso")]
impl From<OwnedSQLiteValue> for turso::Value {
    fn from(value: OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(i) => turso::Value::Integer(i),
            OwnedSQLiteValue::Real(r) => turso::Value::Real(r),
            OwnedSQLiteValue::Text(s) => turso::Value::Text(s),
            OwnedSQLiteValue::Blob(b) => turso::Value::Blob(b.into_vec()),
            OwnedSQLiteValue::Null => turso::Value::Null,
        }
    }
}

#[cfg(feature = "turso")]
impl From<&OwnedSQLiteValue> for turso::Value {
    fn from(value: &OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(i) => turso::Value::Integer(*i),
            OwnedSQLiteValue::Real(r) => turso::Value::Real(*r),
            OwnedSQLiteValue::Text(s) => turso::Value::Text(s.clone()),
            OwnedSQLiteValue::Blob(b) => turso::Value::Blob(b.to_vec()),
            OwnedSQLiteValue::Null => turso::Value::Null,
        }
    }
}

#[cfg(feature = "libsql")]
impl From<OwnedSQLiteValue> for libsql::Value {
    fn from(value: OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(i) => libsql::Value::Integer(i),
            OwnedSQLiteValue::Real(r) => libsql::Value::Real(r),
            OwnedSQLiteValue::Text(s) => libsql::Value::Text(s),
            OwnedSQLiteValue::Blob(b) => libsql::Value::Blob(b.into_vec()),
            OwnedSQLiteValue::Null => libsql::Value::Null,
        }
    }
}

#[cfg(feature = "libsql")]
impl From<&OwnedSQLiteValue> for libsql::Value {
    fn from(value: &OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(i) => libsql::Value::Integer(*i),
            OwnedSQLiteValue::Real(r) => libsql::Value::Real(*r),
            OwnedSQLiteValue::Text(s) => libsql::Value::Text(s.clone()),
            OwnedSQLiteValue::Blob(b) => libsql::Value::Blob(b.to_vec()),
            OwnedSQLiteValue::Null => libsql::Value::Null,
        }
    }
}

// Implement core traits required by Drizzle
impl SQLParam for OwnedSQLiteValue {}

impl<'a> From<OwnedSQLiteValue> for SQL<'a, OwnedSQLiteValue> {
    fn from(value: OwnedSQLiteValue) -> Self {
        SQL::param(value)
    }
}

//------------------------------------------------------------------------------
// From<T> implementations
// Macro-based to reduce boilerplate
//------------------------------------------------------------------------------

/// Macro to implement From<integer> for OwnedSQLiteValue (converts to INTEGER)
macro_rules! impl_from_int_for_owned_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: $ty) -> Self {
                    OwnedSQLiteValue::Integer(value as i64)
                }
            }

            impl From<&$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: &$ty) -> Self {
                    OwnedSQLiteValue::Integer(*value as i64)
                }
            }
        )*
    };
}

/// Macro to implement From<float> for OwnedSQLiteValue (converts to REAL)
macro_rules! impl_from_float_for_owned_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: $ty) -> Self {
                    OwnedSQLiteValue::Real(value as f64)
                }
            }

            impl From<&$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: &$ty) -> Self {
                    OwnedSQLiteValue::Real(*value as f64)
                }
            }
        )*
    };
}

// Integer types -> OwnedSQLiteValue::Integer
impl_from_int_for_owned_sqlite_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

// Float types -> OwnedSQLiteValue::Real
impl_from_float_for_owned_sqlite_value!(f32, f64);

// --- String Types ---

impl From<&str> for OwnedSQLiteValue {
    fn from(value: &str) -> Self {
        OwnedSQLiteValue::Text(value.to_string())
    }
}

impl From<String> for OwnedSQLiteValue {
    fn from(value: String) -> Self {
        OwnedSQLiteValue::Text(value)
    }
}

impl From<&String> for OwnedSQLiteValue {
    fn from(value: &String) -> Self {
        OwnedSQLiteValue::Text(value.clone())
    }
}

// --- Binary Data ---

impl From<&[u8]> for OwnedSQLiteValue {
    fn from(value: &[u8]) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
    }
}

impl From<Vec<u8>> for OwnedSQLiteValue {
    fn from(value: Vec<u8>) -> Self {
        OwnedSQLiteValue::Blob(value.into_boxed_slice())
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl From<Uuid> for OwnedSQLiteValue {
    fn from(value: Uuid) -> Self {
        OwnedSQLiteValue::Blob(value.as_bytes().to_vec().into_boxed_slice())
    }
}

#[cfg(feature = "uuid")]
impl From<&Uuid> for OwnedSQLiteValue {
    fn from(value: &Uuid) -> Self {
        OwnedSQLiteValue::Blob(value.as_bytes().to_vec().into_boxed_slice())
    }
}

// --- Option Types ---
impl<T> From<Option<T>> for OwnedSQLiteValue
where
    T: TryInto<OwnedSQLiteValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.try_into().unwrap_or(OwnedSQLiteValue::Null),
            None => OwnedSQLiteValue::Null,
        }
    }
}

// --- Cow integration for SQL struct ---
impl From<OwnedSQLiteValue> for Cow<'_, OwnedSQLiteValue> {
    fn from(value: OwnedSQLiteValue) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a OwnedSQLiteValue> for Cow<'a, OwnedSQLiteValue> {
    fn from(value: &'a OwnedSQLiteValue) -> Self {
        Cow::Borrowed(value)
    }
}

//------------------------------------------------------------------------------
// TryFrom<OwnedSQLiteValue> implementations
// Uses the FromSQLiteValue trait via convert() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<OwnedSQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_owned_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<OwnedSQLiteValue> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
                    value.convert()
                }
            }
        )*
    };
}

impl_try_from_owned_sqlite_value!(
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
impl_try_from_owned_sqlite_value!(Uuid);

//------------------------------------------------------------------------------
// TryFrom<&OwnedSQLiteValue> implementations for borrowing without consuming
// Uses the FromSQLiteValue trait via convert_ref() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<&OwnedSQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_owned_sqlite_value_ref {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<&OwnedSQLiteValue> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
                    value.convert_ref()
                }
            }
        )*
    };
}

impl_try_from_owned_sqlite_value_ref!(
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
impl_try_from_owned_sqlite_value_ref!(Uuid);

// --- Borrowed reference types (cannot use FromSQLiteValue) ---

impl<'a> TryFrom<&'a OwnedSQLiteValue> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => Ok(s.as_str()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &str", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a OwnedSQLiteValue> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => Ok(b.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &[u8]", value).into(),
            )),
        }
    }
}
