//! Owned SQLite value type and implementations

mod conversions;
mod drivers;

use super::SQLiteValue;
use crate::traits::FromSQLiteValue;
use drizzle_core::{error::DrizzleError, sql::SQL, traits::SQLParam};
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
// Core From<SQLiteValue> conversions
//------------------------------------------------------------------------------

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

//------------------------------------------------------------------------------
// Core traits required by Drizzle
//------------------------------------------------------------------------------

impl SQLParam for OwnedSQLiteValue {}

impl<'a> From<OwnedSQLiteValue> for SQL<'a, OwnedSQLiteValue> {
    fn from(value: OwnedSQLiteValue) -> Self {
        SQL::param(value)
    }
}

//------------------------------------------------------------------------------
// Cow integration for SQL struct
//------------------------------------------------------------------------------

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
