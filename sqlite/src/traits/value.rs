//! Value conversion traits for SQLite types
//!
//! This module provides the `FromSQLiteValue` trait for converting SQLite values
//! to Rust types, and the `DrizzleRow` trait for unified row access across drivers.

use drizzle_core::error::DrizzleError;

/// Trait for types that can be converted from SQLite values.
///
/// SQLite has 5 storage classes: NULL, INTEGER, REAL, TEXT, BLOB.
/// This trait provides conversion methods for each type.
///
/// # Implementation Notes
///
/// - Implement the methods that make sense for your type
/// - Return `Err` for unsupported conversions
/// - `SQLiteEnum` derive automatically implements this trait
pub trait FromSQLiteValue: Sized {
    /// Convert from a 64-bit integer value
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError>;

    /// Convert from a text/string value
    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError>;

    /// Convert from a real/float value
    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError>;

    /// Convert from a blob/binary value
    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError>;

    /// Convert from a NULL value (default returns error)
    fn from_sqlite_null() -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "unexpected NULL value".into(),
        ))
    }
}

/// Trait for database rows that can extract values using `FromSQLiteValue`.
///
/// This provides a unified interface for extracting values from database rows
/// across different SQLite drivers (libsql, turso).
pub trait DrizzleRow {
    /// Get a column value by index
    fn get_column<T: FromSQLiteValue>(&self, idx: usize) -> Result<T, DrizzleError>;

    /// Get a column value by name (optional, not all drivers support this efficiently)
    fn get_column_by_name<T: FromSQLiteValue>(&self, name: &str) -> Result<T, DrizzleError>;
}

// =============================================================================
// Primitive implementations
// =============================================================================

impl FromSQLiteValue for i64 {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i64: {}", value, e).into())
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i64)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to i64".into(),
        ))
    }
}

impl FromSQLiteValue for i32 {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(format!("i64 {} out of range for i32: {}", value, e).into())
        })
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i32: {}", value, e).into())
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i32)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to i32".into(),
        ))
    }
}

impl FromSQLiteValue for i16 {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(format!("i64 {} out of range for i16: {}", value, e).into())
        })
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i16: {}", value, e).into())
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i16)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to i16".into(),
        ))
    }
}

impl FromSQLiteValue for i8 {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(format!("i64 {} out of range for i8: {}", value, e).into())
        })
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i8: {}", value, e).into())
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i8)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to i8".into(),
        ))
    }
}

impl FromSQLiteValue for f64 {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(value as f64)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as f64: {}", value, e).into())
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to f64".into(),
        ))
    }
}

impl FromSQLiteValue for f32 {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(value as f32)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as f32: {}", value, e).into())
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as f32)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to f32".into(),
        ))
    }
}

impl FromSQLiteValue for bool {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(value != 0)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(DrizzleError::ConversionError(
                format!("cannot parse '{}' as bool", value).into(),
            )),
        }
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value != 0.0)
    }

    fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert BLOB to bool".into(),
        ))
    }
}

impl FromSQLiteValue for String {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_utf8(value.to_vec()).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 in BLOB: {}", e).into())
        })
    }
}

impl FromSQLiteValue for Vec<u8> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Ok(value.as_bytes().to_vec())
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Ok(value.to_vec())
    }
}

// Option<T> implementation - handles NULL values
impl<T: FromSQLiteValue> FromSQLiteValue for Option<T> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        T::from_sqlite_integer(value).map(Some)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        T::from_sqlite_text(value).map(Some)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        T::from_sqlite_real(value).map(Some)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        T::from_sqlite_blob(value).map(Some)
    }

    fn from_sqlite_null() -> Result<Self, DrizzleError> {
        Ok(None)
    }
}

// =============================================================================
// Driver-specific DrizzleRow implementations
// =============================================================================

#[cfg(feature = "libsql")]
impl DrizzleRow for libsql::Row {
    fn get_column<T: FromSQLiteValue>(&self, idx: usize) -> Result<T, DrizzleError> {
        let value = self.get_value(idx as i32)?;
        match value {
            libsql::Value::Integer(i) => T::from_sqlite_integer(i),
            libsql::Value::Text(ref s) => T::from_sqlite_text(s),
            libsql::Value::Real(r) => T::from_sqlite_real(r),
            libsql::Value::Blob(ref b) => T::from_sqlite_blob(b),
            libsql::Value::Null => T::from_sqlite_null(),
        }
    }

    fn get_column_by_name<T: FromSQLiteValue>(&self, _name: &str) -> Result<T, DrizzleError> {
        // libsql doesn't have efficient name-based access, would need to iterate columns
        Err(DrizzleError::ConversionError(
            "libsql does not support column access by name in FromRow".into(),
        ))
    }
}

#[cfg(feature = "turso")]
impl DrizzleRow for turso::Row {
    fn get_column<T: FromSQLiteValue>(&self, idx: usize) -> Result<T, DrizzleError> {
        let value = self.get_value(idx)?;
        if value.is_null() {
            T::from_sqlite_null()
        } else if let Some(&i) = value.as_integer() {
            T::from_sqlite_integer(i)
        } else if let Some(s) = value.as_text() {
            T::from_sqlite_text(s)
        } else if let Some(&r) = value.as_real() {
            T::from_sqlite_real(r)
        } else if let Some(b) = value.as_blob() {
            T::from_sqlite_blob(b)
        } else {
            Err(DrizzleError::ConversionError(
                "unknown SQLite value type".into(),
            ))
        }
    }

    fn get_column_by_name<T: FromSQLiteValue>(&self, _name: &str) -> Result<T, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "turso does not support column access by name in FromRow".into(),
        ))
    }
}

// =============================================================================
// UUID support (when feature enabled)
// =============================================================================

#[cfg(feature = "uuid")]
impl FromSQLiteValue for uuid::Uuid {
    fn from_sqlite_integer(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert INTEGER to UUID".into(),
        ))
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        uuid::Uuid::parse_str(value).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UUID string '{}': {}", value, e).into())
        })
    }

    fn from_sqlite_real(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert REAL to UUID".into(),
        ))
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        uuid::Uuid::from_slice(value).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UUID bytes: {}", e).into())
        })
    }
}

