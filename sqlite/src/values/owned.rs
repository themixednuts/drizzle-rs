//! Owned SQLite value type and implementations

use drizzle_core::{SQL, SQLParam, error::DrizzleError};

#[cfg(feature = "rusqlite")]
use rusqlite::types::FromSql;
#[cfg(feature = "turso")]
use turso::IntoValue;
#[cfg(feature = "uuid")]
use uuid::Uuid;

use std::borrow::Cow;

use crate::SQLiteValue;

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

impl From<OwnedSQLiteValue> for SQL<'_, OwnedSQLiteValue> {
    fn from(value: OwnedSQLiteValue) -> Self {
        SQL::parameter(value)
    }
}

//------------------------------------------------------------------------------
// From<T> implementations
//------------------------------------------------------------------------------

// --- Integer Types ---

// i8
impl From<i8> for OwnedSQLiteValue {
    fn from(value: i8) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&i8> for OwnedSQLiteValue {
    fn from(value: &i8) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// i16
impl From<i16> for OwnedSQLiteValue {
    fn from(value: i16) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&i16> for OwnedSQLiteValue {
    fn from(value: &i16) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// i32
impl From<i32> for OwnedSQLiteValue {
    fn from(value: i32) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&i32> for OwnedSQLiteValue {
    fn from(value: &i32) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

impl TryFrom<OwnedSQLiteValue> for i32 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i32",
                value
            ))),
        }
    }
}

// i64
impl From<i64> for OwnedSQLiteValue {
    fn from(value: i64) -> Self {
        OwnedSQLiteValue::Integer(value)
    }
}

impl From<&i64> for OwnedSQLiteValue {
    fn from(value: &i64) -> Self {
        OwnedSQLiteValue::Integer(*value)
    }
}

// isize
impl From<isize> for OwnedSQLiteValue {
    fn from(value: isize) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&isize> for OwnedSQLiteValue {
    fn from(value: &isize) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// u8
impl From<u8> for OwnedSQLiteValue {
    fn from(value: u8) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&u8> for OwnedSQLiteValue {
    fn from(value: &u8) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// u16
impl From<u16> for OwnedSQLiteValue {
    fn from(value: u16) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&u16> for OwnedSQLiteValue {
    fn from(value: &u16) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// u32
impl From<u32> for OwnedSQLiteValue {
    fn from(value: u32) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&u32> for OwnedSQLiteValue {
    fn from(value: &u32) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// u64
impl From<u64> for OwnedSQLiteValue {
    fn from(value: u64) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&u64> for OwnedSQLiteValue {
    fn from(value: &u64) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// usize
impl From<usize> for OwnedSQLiteValue {
    fn from(value: usize) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&usize> for OwnedSQLiteValue {
    fn from(value: &usize) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

// --- Floating Point Types ---

// f32
impl From<f32> for OwnedSQLiteValue {
    fn from(value: f32) -> Self {
        OwnedSQLiteValue::Real(value as f64)
    }
}

impl From<&f32> for OwnedSQLiteValue {
    fn from(value: &f32) -> Self {
        OwnedSQLiteValue::Real(*value as f64)
    }
}

// f64
impl From<f64> for OwnedSQLiteValue {
    fn from(value: f64) -> Self {
        OwnedSQLiteValue::Real(value)
    }
}

impl From<&f64> for OwnedSQLiteValue {
    fn from(value: &f64) -> Self {
        OwnedSQLiteValue::Real(*value)
    }
}

// --- Boolean ---

impl From<bool> for OwnedSQLiteValue {
    fn from(value: bool) -> Self {
        OwnedSQLiteValue::Integer(value as i64)
    }
}

impl From<&bool> for OwnedSQLiteValue {
    fn from(value: &bool) -> Self {
        OwnedSQLiteValue::Integer(*value as i64)
    }
}

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
//------------------------------------------------------------------------------

// --- Integer Types ---

impl TryFrom<OwnedSQLiteValue> for i8 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i8",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for i16 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i16",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for i64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i64",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for isize {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to isize",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for u8 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u8",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for u16 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u16",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for u32 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u32",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for u64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u64",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for usize {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to usize",
                value
            ))),
        }
    }
}

// --- Floating Point Types ---

impl TryFrom<OwnedSQLiteValue> for f32 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Real(f) => Ok(f as f32),
            OwnedSQLiteValue::Integer(i) => Ok(i as f32),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f32",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedSQLiteValue> for f64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Real(f) => Ok(f),
            OwnedSQLiteValue::Integer(i) => Ok(i as f64),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f64",
                value
            ))),
        }
    }
}

// --- Boolean ---

impl TryFrom<OwnedSQLiteValue> for bool {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(0) => Ok(false),
            OwnedSQLiteValue::Integer(_) => Ok(true),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}

// --- String Types ---

impl TryFrom<OwnedSQLiteValue> for String {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => Ok(s),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to String",
                value
            ))),
        }
    }
}

// --- Binary Data ---

impl TryFrom<OwnedSQLiteValue> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => Ok(b.into_vec()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to Vec<u8>",
                value
            ))),
        }
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl TryFrom<OwnedSQLiteValue> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => {
                let bytes: [u8; 16] = b.to_vec().try_into().map_err(|_| {
                    DrizzleError::ConversionError("UUID blob must be exactly 16 bytes".to_string())
                })?;
                Ok(Uuid::from_bytes(bytes))
            }
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to UUID",
                value
            ))),
        }
    }
}

//------------------------------------------------------------------------------
// TryFrom<&OwnedSQLiteValue> implementations for borrowing without consuming
//------------------------------------------------------------------------------

// --- Integer Types ---

impl TryFrom<&OwnedSQLiteValue> for i8 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i8",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for i16 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i16",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for i32 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i32",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for i64 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok(*i),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i64",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for isize {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to isize",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for u8 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u8",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for u16 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u16",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for u32 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u32",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for u64 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to u64",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for usize {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(i) => Ok((*i).try_into()?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to usize",
                value
            ))),
        }
    }
}

// --- Floating Point Types ---

impl TryFrom<&OwnedSQLiteValue> for f32 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Real(f) => Ok(*f as f32),
            OwnedSQLiteValue::Integer(i) => Ok(*i as f32),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f32",
                value
            ))),
        }
    }
}

impl TryFrom<&OwnedSQLiteValue> for f64 {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Real(f) => Ok(*f),
            OwnedSQLiteValue::Integer(i) => Ok(*i as f64),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f64",
                value
            ))),
        }
    }
}

// --- Boolean ---

impl TryFrom<&OwnedSQLiteValue> for bool {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Integer(0) => Ok(false),
            OwnedSQLiteValue::Integer(_) => Ok(true),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}

// --- String Types ---

impl TryFrom<&OwnedSQLiteValue> for String {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => Ok(s.clone()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to String",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&'a OwnedSQLiteValue> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => Ok(s.as_str()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to &str",
                value
            ))),
        }
    }
}

// --- Binary Data ---

impl TryFrom<&OwnedSQLiteValue> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => Ok(b.to_vec()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to Vec<u8>",
                value
            ))),
        }
    }
}

impl<'a> TryFrom<&'a OwnedSQLiteValue> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => Ok(b.as_ref()),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to &[u8]",
                value
            ))),
        }
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl TryFrom<&OwnedSQLiteValue> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => {
                let bytes: [u8; 16] = b.as_ref().try_into().map_err(|_| {
                    DrizzleError::ConversionError("UUID blob must be exactly 16 bytes".to_string())
                })?;
                Ok(Uuid::from_bytes(bytes))
            }
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to UUID",
                value
            ))),
        }
    }
}
