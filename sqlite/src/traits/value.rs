//! Value conversion traits for SQLite types
//!
//! This module provides the `FromSQLiteValue` trait for converting SQLite values
//! to Rust types, and row capability traits for unified access across drivers.

use crate::prelude::*;
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

    /// Helper function to convert from rusqlite's ValueRef using FromSQLiteValue
    #[cfg(feature = "rusqlite")]
    fn from_value_ref(value: ::rusqlite::types::ValueRef<'_>) -> Result<Self, DrizzleError> {
        match value {
            ::rusqlite::types::ValueRef::Null => Self::from_sqlite_null(),
            ::rusqlite::types::ValueRef::Integer(i) => Self::from_sqlite_integer(i),
            ::rusqlite::types::ValueRef::Real(r) => Self::from_sqlite_real(r),
            ::rusqlite::types::ValueRef::Text(text) => {
                let s = core::str::from_utf8(text).map_err(|e| {
                    DrizzleError::ConversionError(format!("invalid UTF-8: {}", e).into())
                })?;
                Self::from_sqlite_text(s)
            }
            ::rusqlite::types::ValueRef::Blob(blob) => Self::from_sqlite_blob(blob),
        }
    }
}

/// Row capability for index-based extraction.
pub trait DrizzleRowByIndex {
    /// Get a column value by index
    fn get_column<T: FromSQLiteValue>(&self, idx: usize) -> Result<T, DrizzleError>;
}

/// Optional row capability for name-based extraction.
pub trait DrizzleRowByName: DrizzleRowByIndex {
    /// Get a column value by name.
    fn get_column_by_name<T: FromSQLiteValue>(&self, name: &str) -> Result<T, DrizzleError>;
}

fn checked_real_to_int<T>(value: f64, type_name: &str) -> Result<T, DrizzleError>
where
    T: TryFrom<i128>,
    <T as TryFrom<i128>>::Error: core::fmt::Display,
{
    if !value.is_finite() {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-finite REAL {} to {}", value, type_name).into(),
        ));
    }

    if value % 1.0 != 0.0 {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-integer REAL {} to {}", value, type_name).into(),
        ));
    }

    if value < i128::MIN as f64 || value > i128::MAX as f64 {
        return Err(DrizzleError::ConversionError(
            format!("REAL {} out of range for {}", value, type_name).into(),
        ));
    }

    let int_value = value as i128;
    int_value.try_into().map_err(|e| {
        DrizzleError::ConversionError(
            format!("REAL {} out of range for {}: {}", value, type_name, e).into(),
        )
    })
}

// =============================================================================
// Primitive implementations
// =============================================================================

/// Macro to implement FromSQLiteValue for integer types (handles narrowing conversion from i64)
macro_rules! impl_from_sqlite_value_int {
    // Special case for i64 - no conversion needed
    (i64) => {
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
                checked_real_to_int(value, "i64")
            }

            fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
                Err(DrizzleError::ConversionError("cannot convert BLOB to i64".into()))
            }
        }
    };
    // General case for other integer types - uses try_into for narrowing
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FromSQLiteValue for $ty {
                fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
                    value.try_into().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("i64 {} out of range for {}: {}", value, stringify!($ty), e).into(),
                        )
                    })
                }

                fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
                    value.parse().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("cannot parse '{}' as {}: {}", value, stringify!($ty), e).into()
                        )
                    })
                }

                fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
                    checked_real_to_int(value, stringify!($ty))
                }

                fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
                    Err(DrizzleError::ConversionError(
                        concat!("cannot convert BLOB to ", stringify!($ty)).into()
                    ))
                }
            }
        )+
    };
}

/// Macro to implement FromSQLiteValue for float types
macro_rules! impl_from_sqlite_value_float {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FromSQLiteValue for $ty {
                fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
                    value.parse().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("cannot parse '{}' as {}: {}", value, stringify!($ty), e).into()
                        )
                    })
                }

                fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_sqlite_blob(_value: &[u8]) -> Result<Self, DrizzleError> {
                    Err(DrizzleError::ConversionError(
                        concat!("cannot convert BLOB to ", stringify!($ty)).into()
                    ))
                }
            }
        )+
    };
}

// Integer types
impl_from_sqlite_value_int!(i64);
impl_from_sqlite_value_int!(i8, i16, i32, isize, u8, u16, u32, u64, usize);

// Float types
impl_from_sqlite_value_float!(f32, f64);

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

impl FromSQLiteValue for compact_str::CompactString {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Ok(compact_str::CompactString::new(value.to_string()))
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Ok(compact_str::CompactString::new(value))
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Ok(compact_str::CompactString::new(value.to_string()))
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        let s = String::from_utf8(value.to_vec()).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 in BLOB: {}", e).into())
        })?;
        Ok(compact_str::CompactString::new(s))
    }
}

impl FromSQLiteValue for Box<String> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        String::from_sqlite_integer(value).map(Box::new)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_sqlite_text(value).map(Box::new)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        String::from_sqlite_real(value).map(Box::new)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_sqlite_blob(value).map(Box::new)
    }
}

impl FromSQLiteValue for Rc<String> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        String::from_sqlite_integer(value).map(Rc::new)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_sqlite_text(value).map(Rc::new)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        String::from_sqlite_real(value).map(Rc::new)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_sqlite_blob(value).map(Rc::new)
    }
}

impl FromSQLiteValue for Arc<String> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        String::from_sqlite_integer(value).map(Arc::new)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_sqlite_text(value).map(Arc::new)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        String::from_sqlite_real(value).map(Arc::new)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_sqlite_blob(value).map(Arc::new)
    }
}

impl FromSQLiteValue for Box<str> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        String::from_sqlite_integer(value).map(String::into_boxed_str)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_sqlite_text(value).map(String::into_boxed_str)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        String::from_sqlite_real(value).map(String::into_boxed_str)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_sqlite_blob(value).map(String::into_boxed_str)
    }
}

impl FromSQLiteValue for Rc<str> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        String::from_sqlite_integer(value).map(Rc::from)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_sqlite_text(value).map(Rc::from)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        String::from_sqlite_real(value).map(Rc::from)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_sqlite_blob(value).map(Rc::from)
    }
}

impl FromSQLiteValue for Arc<str> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        String::from_sqlite_integer(value).map(Arc::from)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_sqlite_text(value).map(Arc::from)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        String::from_sqlite_real(value).map(Arc::from)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_sqlite_blob(value).map(Arc::from)
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

#[cfg(feature = "bytes")]
impl FromSQLiteValue for bytes::Bytes {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_integer(value).map(bytes::Bytes::from)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_text(value).map(bytes::Bytes::from)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_real(value).map(bytes::Bytes::from)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Ok(bytes::Bytes::copy_from_slice(value))
    }
}

#[cfg(feature = "bytes")]
impl FromSQLiteValue for bytes::BytesMut {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_integer(value).map(|v| bytes::BytesMut::from(v.as_slice()))
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_text(value).map(|v| bytes::BytesMut::from(v.as_slice()))
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_real(value).map(|v| bytes::BytesMut::from(v.as_slice()))
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Ok(bytes::BytesMut::from(value))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> FromSQLiteValue for smallvec::SmallVec<[u8; N]> {
    fn from_sqlite_integer(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert INTEGER to SmallVec<u8>, use BLOB".into(),
        ))
    }

    fn from_sqlite_text(_value: &str) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert TEXT to SmallVec<u8>, use BLOB".into(),
        ))
    }

    fn from_sqlite_real(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert REAL to SmallVec<u8>, use BLOB".into(),
        ))
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        let mut out = smallvec::SmallVec::<[u8; N]>::new();
        out.extend_from_slice(value);
        Ok(out)
    }
}

impl FromSQLiteValue for Box<Vec<u8>> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_integer(value).map(Box::new)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_text(value).map(Box::new)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_real(value).map(Box::new)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_blob(value).map(Box::new)
    }
}

impl FromSQLiteValue for Rc<Vec<u8>> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_integer(value).map(Rc::new)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_text(value).map(Rc::new)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_real(value).map(Rc::new)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_blob(value).map(Rc::new)
    }
}

impl FromSQLiteValue for Arc<Vec<u8>> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_integer(value).map(Arc::new)
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_text(value).map(Arc::new)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_real(value).map(Arc::new)
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_sqlite_blob(value).map(Arc::new)
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

#[cfg(feature = "rusqlite")]
impl DrizzleRowByIndex for rusqlite::Row<'_> {
    fn get_column<T: FromSQLiteValue>(&self, idx: usize) -> Result<T, DrizzleError> {
        let value_ref = self.get_ref(idx)?;
        match value_ref {
            rusqlite::types::ValueRef::Integer(i) => T::from_sqlite_integer(i),
            rusqlite::types::ValueRef::Text(s) => {
                let s = core::str::from_utf8(s).map_err(|e| {
                    DrizzleError::ConversionError(format!("invalid UTF-8: {}", e).into())
                })?;
                T::from_sqlite_text(s)
            }
            rusqlite::types::ValueRef::Real(r) => T::from_sqlite_real(r),
            rusqlite::types::ValueRef::Blob(b) => T::from_sqlite_blob(b),
            rusqlite::types::ValueRef::Null => T::from_sqlite_null(),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl DrizzleRowByName for rusqlite::Row<'_> {
    fn get_column_by_name<T: FromSQLiteValue>(&self, name: &str) -> Result<T, DrizzleError> {
        let idx = self.as_ref().column_index(name)?;
        DrizzleRowByIndex::get_column(self, idx)
    }
}

#[cfg(feature = "libsql")]
impl DrizzleRowByIndex for libsql::Row {
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
}

#[cfg(feature = "libsql")]
impl DrizzleRowByName for libsql::Row {
    fn get_column_by_name<T: FromSQLiteValue>(&self, name: &str) -> Result<T, DrizzleError> {
        let idx = (0..self.column_count())
            .find(|&i| self.column_name(i) == Some(name))
            .ok_or_else(|| {
                DrizzleError::ConversionError(format!("column '{}' not found", name).into())
            })?;

        DrizzleRowByIndex::get_column(self, idx as usize)
    }
}

#[cfg(feature = "turso")]
impl DrizzleRowByIndex for turso::Row {
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
        uuid::Uuid::from_slice(value)
            .map_err(|e| DrizzleError::ConversionError(format!("invalid UUID bytes: {}", e).into()))
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> FromSQLiteValue for arrayvec::ArrayString<N> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        let s = value.to_string();
        arrayvec::ArrayString::from(&s).map_err(|_| {
            DrizzleError::ConversionError(
                format!(
                    "String length {} exceeds ArrayString capacity {}",
                    s.len(),
                    N
                )
                .into(),
            )
        })
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        arrayvec::ArrayString::from(value).map_err(|_| {
            DrizzleError::ConversionError(
                format!(
                    "Text length {} exceeds ArrayString capacity {}",
                    value.len(),
                    N
                )
                .into(),
            )
        })
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        let s = value.to_string();
        arrayvec::ArrayString::from(&s).map_err(|_| {
            DrizzleError::ConversionError(
                format!(
                    "String length {} exceeds ArrayString capacity {}",
                    s.len(),
                    N
                )
                .into(),
            )
        })
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        let s = String::from_utf8(value.to_vec())
            .map_err(|e| DrizzleError::ConversionError(format!("invalid UTF-8: {}", e).into()))?;
        arrayvec::ArrayString::from(&s).map_err(|_| {
            DrizzleError::ConversionError(
                format!(
                    "String length {} exceeds ArrayString capacity {}",
                    s.len(),
                    N
                )
                .into(),
            )
        })
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> FromSQLiteValue for arrayvec::ArrayVec<u8, N> {
    fn from_sqlite_integer(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert INTEGER to ArrayVec<u8>, use BLOB".into(),
        ))
    }

    fn from_sqlite_text(_value: &str) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert TEXT to ArrayVec<u8>, use BLOB".into(),
        ))
    }

    fn from_sqlite_real(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert REAL to ArrayVec<u8>, use BLOB".into(),
        ))
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        arrayvec::ArrayVec::try_from(value).map_err(|_| {
            DrizzleError::ConversionError(
                format!(
                    "Blob length {} exceeds ArrayVec capacity {}",
                    value.len(),
                    N
                )
                .into(),
            )
        })
    }
}
