//! `SQLite` value types and conversions
//!
//! This module contains the core `SQLiteValue` type and all its conversions.

mod conversions;
mod drivers;
mod insert;
pub mod owned;
mod update;

pub use insert::*;
pub use owned::*;
pub use update::*;

use crate::prelude::*;
use crate::traits::FromSQLiteValue;
use drizzle_core::{dialect::Dialect, error::DrizzleError, sql::SQL, traits::SQLParam};

//------------------------------------------------------------------------------
// SQLiteValue Definition
//------------------------------------------------------------------------------

/// Represents a `SQLite` value
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

/// Borrowed view of a `SQLite` value.
///
/// This is the zero-copy read-side representation used by custom column
/// decoders. Text and blob payloads borrow directly from the driver row or
/// from an existing [`SQLiteValue`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub enum SQLiteValueRef<'a> {
    /// Integer value (i64)
    Integer(i64),
    /// Real value (f64)
    Real(f64),
    /// Text value
    Text(&'a str),
    /// Blob value
    Blob(&'a [u8]),
    /// NULL value
    #[default]
    Null,
}

impl<'a> SQLiteValueRef<'a> {
    /// Converts this borrowed value into a `SQLiteValue`.
    #[inline]
    #[must_use]
    pub const fn into_value(self) -> SQLiteValue<'a> {
        match self {
            Self::Integer(value) => SQLiteValue::Integer(value),
            Self::Real(value) => SQLiteValue::Real(value),
            Self::Text(value) => SQLiteValue::Text(Cow::Borrowed(value)),
            Self::Blob(value) => SQLiteValue::Blob(Cow::Borrowed(value)),
            Self::Null => SQLiteValue::Null,
        }
    }

    /// Converts a rusqlite borrowed value into a dialect-neutral borrowed
    /// value.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if a `TEXT` value is not valid
    /// UTF-8.
    #[cfg(feature = "rusqlite")]
    #[inline]
    pub fn try_from_rusqlite_value_ref(
        value: ::rusqlite::types::ValueRef<'a>,
    ) -> Result<Self, DrizzleError> {
        match value {
            ::rusqlite::types::ValueRef::Null => Ok(Self::Null),
            ::rusqlite::types::ValueRef::Integer(value) => Ok(Self::Integer(value)),
            ::rusqlite::types::ValueRef::Real(value) => Ok(Self::Real(value)),
            ::rusqlite::types::ValueRef::Text(value) => {
                let value = core::str::from_utf8(value).map_err(|e| {
                    DrizzleError::ConversionError(format!("invalid UTF-8: {e}").into())
                })?;
                Ok(Self::Text(value))
            }
            ::rusqlite::types::ValueRef::Blob(value) => Ok(Self::Blob(value)),
        }
    }
}

impl<'a> From<SQLiteValueRef<'a>> for SQLiteValue<'a> {
    #[inline]
    fn from(value: SQLiteValueRef<'a>) -> Self {
        value.into_value()
    }
}

impl<'a> From<&'a SQLiteValue<'_>> for SQLiteValueRef<'a> {
    #[inline]
    fn from(value: &'a SQLiteValue<'_>) -> Self {
        value.as_ref()
    }
}

impl SQLiteValue<'_> {
    /// Returns true if this value is NULL.
    #[inline]
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, SQLiteValue::Null)
    }

    /// Returns the integer value if this is an INTEGER.
    #[inline]
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            SQLiteValue::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the real value if this is a REAL.
    #[inline]
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            SQLiteValue::Real(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the text value if this is TEXT.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SQLiteValue::Text(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns the blob value if this is BLOB.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            SQLiteValue::Blob(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns a borrowed view of this value.
    #[inline]
    #[must_use]
    pub fn as_ref(&self) -> SQLiteValueRef<'_> {
        match self {
            SQLiteValue::Integer(value) => SQLiteValueRef::Integer(*value),
            SQLiteValue::Real(value) => SQLiteValueRef::Real(*value),
            SQLiteValue::Text(value) => SQLiteValueRef::Text(value.as_ref()),
            SQLiteValue::Blob(value) => SQLiteValueRef::Blob(value.as_ref()),
            SQLiteValue::Null => SQLiteValueRef::Null,
        }
    }

    /// Converts this value into an owned representation.
    #[inline]
    #[must_use]
    pub fn into_owned(self) -> OwnedSQLiteValue {
        self.into()
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
    ///
    /// # Example
    /// ```rust
    /// # let _ = r####"
    /// let value = SQLiteValue::Integer(42);
    /// let num: i64 = value.convert()?;
    /// # "####;
    /// ```
    pub fn convert<T: FromSQLiteValue>(self) -> Result<T, DrizzleError> {
        T::from_sqlite_ref(self.as_ref())
    }

    /// Convert a reference to this `SQLite` value to a Rust type.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when the stored variant cannot
    /// be decoded into `T`.
    pub fn convert_ref<T: FromSQLiteValue>(&self) -> Result<T, DrizzleError> {
        T::from_sqlite_ref(self.as_ref())
    }
}

impl core::fmt::Display for SQLiteValue<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

// Implement core traits required by Drizzle
impl SQLParam for SQLiteValue<'_> {
    const DIALECT: Dialect = Dialect::SQLite;
    type DialectMarker = drizzle_core::dialect::SQLiteDialect;
}

impl<'a> From<SQLiteValue<'a>> for SQL<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        SQL::param(value)
    }
}

impl FromIterator<OwnedSQLiteValue> for Vec<SQLiteValue<'_>> {
    fn from_iter<T: IntoIterator<Item = OwnedSQLiteValue>>(iter: T) -> Self {
        iter.into_iter().map(SQLiteValue::from).collect()
    }
}

impl<'a> FromIterator<&'a OwnedSQLiteValue> for Vec<SQLiteValue<'a>> {
    fn from_iter<T: IntoIterator<Item = &'a OwnedSQLiteValue>>(iter: T) -> Self {
        iter.into_iter().map(SQLiteValue::from).collect()
    }
}
