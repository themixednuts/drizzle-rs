//! SQLite value types and conversions
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
    /// Returns true if this value is NULL.
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, SQLiteValue::Null)
    }

    /// Returns the integer value if this is an INTEGER.
    #[inline]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            SQLiteValue::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the real value if this is a REAL.
    #[inline]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            SQLiteValue::Real(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the text value if this is TEXT.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SQLiteValue::Text(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns the blob value if this is BLOB.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            SQLiteValue::Blob(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Converts this value into an owned representation.
    #[inline]
    pub fn into_owned(self) -> OwnedSQLiteValue {
        self.into()
    }

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

impl<'a> core::fmt::Display for SQLiteValue<'a> {
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
impl<'a> SQLParam for SQLiteValue<'a> {
    const DIALECT: Dialect = Dialect::SQLite;
    type DialectMarker = drizzle_core::dialect::SQLiteDialect;
}

impl<'a> From<SQLiteValue<'a>> for SQL<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        SQL::param(value)
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
