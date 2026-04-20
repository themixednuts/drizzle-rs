//! Owned `SQLite` value type and implementations

mod conversions;
mod drivers;

use super::SQLiteValue;
use crate::prelude::*;
use crate::traits::FromSQLiteValue;
use drizzle_core::{error::DrizzleError, sql::SQL, traits::SQLParam};

/// Represents a `SQLite` value (owned version)
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
    /// Returns true if this value is NULL.
    #[inline]
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the integer value if this is an INTEGER.
    #[inline]
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the real value if this is a REAL.
    #[inline]
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Real(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the text value if this is TEXT.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// Returns the blob value if this is BLOB.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Blob(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns a borrowed `SQLiteValue` view of this owned value.
    #[inline]
    #[must_use]
    pub fn as_value(&self) -> SQLiteValue<'_> {
        match self {
            Self::Integer(value) => SQLiteValue::Integer(*value),
            Self::Real(value) => SQLiteValue::Real(*value),
            Self::Text(value) => SQLiteValue::Text(Cow::Borrowed(value)),
            Self::Blob(value) => SQLiteValue::Blob(Cow::Borrowed(value)),
            Self::Null => SQLiteValue::Null,
        }
    }

    /// Convert this `SQLite` value to a Rust type using the `FromSQLiteValue` trait.
    ///
    /// This provides a unified conversion interface for all types that implement
    /// `FromSQLiteValue`, including primitives and enum types.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when the stored variant cannot
    /// be decoded into `T`.
    pub fn convert<T: FromSQLiteValue>(self) -> Result<T, DrizzleError> {
        match self {
            Self::Integer(i) => T::from_sqlite_integer(i),
            Self::Text(s) => T::from_sqlite_text(&s),
            Self::Real(r) => T::from_sqlite_real(r),
            Self::Blob(b) => T::from_sqlite_blob(&b),
            Self::Null => T::from_sqlite_null(),
        }
    }

    /// Convert a reference to this `SQLite` value to a Rust type.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when the stored variant cannot
    /// be decoded into `T`.
    pub fn convert_ref<T: FromSQLiteValue>(&self) -> Result<T, DrizzleError> {
        match self {
            Self::Integer(i) => T::from_sqlite_integer(*i),
            Self::Text(s) => T::from_sqlite_text(s),
            Self::Real(r) => T::from_sqlite_real(*r),
            Self::Blob(b) => T::from_sqlite_blob(b),
            Self::Null => T::from_sqlite_null(),
        }
    }
}

impl core::fmt::Display for OwnedSQLiteValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = match self {
            Self::Integer(i) => i.to_string(),
            Self::Real(r) => r.to_string(),
            Self::Text(s) => s.clone(),
            Self::Blob(b) => String::from_utf8_lossy(b).to_string(),
            Self::Null => String::new(),
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

impl SQLParam for OwnedSQLiteValue {
    const DIALECT: drizzle_core::Dialect = drizzle_core::Dialect::SQLite;
    type DialectMarker = drizzle_core::dialect::SQLiteDialect;
}

impl From<OwnedSQLiteValue> for SQL<'_, OwnedSQLiteValue> {
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
