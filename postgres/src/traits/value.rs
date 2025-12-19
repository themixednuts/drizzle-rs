//! Value conversion traits for PostgreSQL types
//!
//! This module provides the `FromPostgresValue` trait for converting PostgreSQL values
//! to Rust types, and the `DrizzleRow` trait for unified row access across drivers.
//!
//! This pattern mirrors the SQLite implementation to provide driver-agnostic
//! row conversions for postgres, tokio-postgres, and potentially other drivers.

use drizzle_core::error::DrizzleError;

/// Trait for types that can be converted from PostgreSQL values.
///
/// PostgreSQL has many types, but this trait focuses on the core conversions:
/// - Integers (i16, i32, i64)
/// - Floats (f32, f64)
/// - Text (String, &str)
/// - Binary (Vec<u8>, &[u8])
/// - Boolean
/// - NULL handling
///
/// # Implementation Notes
///
/// - Implement the methods that make sense for your type
/// - Return `Err` for unsupported conversions
/// - `PostgresEnum` derive automatically implements this trait
pub trait FromPostgresValue: Sized {
    /// Convert from a boolean value
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError>;

    /// Convert from a 16-bit integer value
    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError>;

    /// Convert from a 32-bit integer value
    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError>;

    /// Convert from a 64-bit integer value
    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError>;

    /// Convert from a 32-bit float value
    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError>;

    /// Convert from a 64-bit float value
    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError>;

    /// Convert from a text/string value
    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError>;

    /// Convert from a binary/bytea value
    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError>;

    /// Convert from a NULL value (default returns error)
    fn from_postgres_null() -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "unexpected NULL value".into(),
        ))
    }

    /// Convert from a UUID value
    #[cfg(feature = "uuid")]
    fn from_postgres_uuid(value: uuid::Uuid) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert UUID {} to target type", value).into(),
        ))
    }
}

/// Trait for database rows that can extract values using `FromPostgresValue`.
///
/// This provides a unified interface for extracting values from database rows
/// across different PostgreSQL drivers (postgres, tokio-postgres).
pub trait DrizzleRow {
    /// Get a column value by index
    fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError>;

    /// Get a column value by name
    fn get_column_by_name<T: FromPostgresValue>(&self, name: &str) -> Result<T, DrizzleError>;
}

// =============================================================================
// Primitive implementations
// =============================================================================

impl FromPostgresValue for bool {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value != 0)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(value != 0)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Ok(value != 0)
    }

    fn from_postgres_f32(_value: f32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f32 to bool".into(),
        ))
    }

    fn from_postgres_f64(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f64 to bool".into(),
        ))
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        match value.to_lowercase().as_str() {
            "true" | "t" | "1" | "yes" | "on" => Ok(true),
            "false" | "f" | "0" | "no" | "off" => Ok(false),
            _ => Err(DrizzleError::ConversionError(
                format!("cannot parse '{}' as bool", value).into(),
            )),
        }
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to bool".into(),
        ))
    }
}

/// Macro to implement FromPostgresValue for integer types
macro_rules! impl_from_postgres_value_int {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FromPostgresValue for $ty {
                fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
                    Ok(if value { 1 } else { 0 })
                }

                fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
                    value.try_into().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("i16 {} out of range for {}: {}", value, stringify!($ty), e).into(),
                        )
                    })
                }

                fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
                    value.try_into().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("i32 {} out of range for {}: {}", value, stringify!($ty), e).into(),
                        )
                    })
                }

                fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
                    value.try_into().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("i64 {} out of range for {}: {}", value, stringify!($ty), e).into(),
                        )
                    })
                }

                fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
                    value.parse().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("cannot parse '{}' as {}: {}", value, stringify!($ty), e).into()
                        )
                    })
                }

                fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
                    Err(DrizzleError::ConversionError(
                        concat!("cannot convert bytes to ", stringify!($ty)).into()
                    ))
                }
            }
        )+
    };
}

// Special case for i16 - no conversion needed
impl FromPostgresValue for i16 {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(if value { 1 } else { 0 })
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(
                format!("i32 {} out of range for i16: {}", value, e).into(),
            )
        })
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(
                format!("i64 {} out of range for i16: {}", value, e).into(),
            )
        })
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(value as i16)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i16)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i16: {}", value, e).into())
        })
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to i16".into(),
        ))
    }
}

// Special case for i32 - no conversion needed
impl FromPostgresValue for i32 {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(if value { 1 } else { 0 })
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value as i32)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(
                format!("i64 {} out of range for i32: {}", value, e).into(),
            )
        })
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(value as i32)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i32)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i32: {}", value, e).into())
        })
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to i32".into(),
        ))
    }
}

// Special case for i64 - no conversion needed
impl FromPostgresValue for i64 {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(if value { 1 } else { 0 })
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value as i64)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(value as i64)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(value as i64)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(value as i64)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{}' as i64: {}", value, e).into())
        })
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to i64".into(),
        ))
    }
}

// Other integer types that need conversion
impl_from_postgres_value_int!(i8, u8, u16, u32, u64, isize, usize);

/// Macro to implement FromPostgresValue for float types
macro_rules! impl_from_postgres_value_float {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FromPostgresValue for $ty {
                fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
                    Ok(if value { 1.0 } else { 0.0 })
                }

                fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
                    Ok(value as $ty)
                }

                fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
                    value.parse().map_err(|e| {
                        DrizzleError::ConversionError(
                            format!("cannot parse '{}' as {}: {}", value, stringify!($ty), e).into()
                        )
                    })
                }

                fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
                    Err(DrizzleError::ConversionError(
                        concat!("cannot convert bytes to ", stringify!($ty)).into()
                    ))
                }
            }
        )+
    };
}

impl_from_postgres_value_float!(f32, f64);

impl FromPostgresValue for String {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Ok(value.to_string())
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_utf8(value.to_vec()).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 in bytes: {}", e).into())
        })
    }
}

impl FromPostgresValue for Vec<u8> {
    fn from_postgres_bool(_value: bool) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bool to Vec<u8>".into(),
        ))
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(value.to_le_bytes().to_vec())
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Ok(value.as_bytes().to_vec())
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Ok(value.to_vec())
    }
}

// Option<T> implementation - handles NULL values
impl<T: FromPostgresValue> FromPostgresValue for Option<T> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        T::from_postgres_bool(value).map(Some)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        T::from_postgres_i16(value).map(Some)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        T::from_postgres_i32(value).map(Some)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        T::from_postgres_i64(value).map(Some)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        T::from_postgres_f32(value).map(Some)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        T::from_postgres_f64(value).map(Some)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        T::from_postgres_text(value).map(Some)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        T::from_postgres_bytes(value).map(Some)
    }

    fn from_postgres_null() -> Result<Self, DrizzleError> {
        Ok(None)
    }
}

// =============================================================================
// Driver-specific DrizzleRow implementations
// =============================================================================

// Note: postgres::Row is a re-export of tokio_postgres::Row, so we only need
// to implement for one type. We use tokio_postgres::Row as it's the underlying type.
// When only postgres-sync is enabled, postgres::Row will be available.

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
mod postgres_row_impl {
    use super::*;

    // Helper function to convert a row value to our type
    // This uses the native driver's try_get functionality
    fn convert_column<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: impl ColumnRef,
    ) -> Result<T, DrizzleError> {
        // Try bool first
        if let Ok(Some(v)) = row.try_get_bool(&column) {
            return T::from_postgres_bool(v);
        }

        // Try i64 (covers BIGINT)
        if let Ok(Some(v)) = row.try_get_i64(&column) {
            return T::from_postgres_i64(v);
        }

        // Try i32 (covers INTEGER)
        if let Ok(Some(v)) = row.try_get_i32(&column) {
            return T::from_postgres_i32(v);
        }

        // Try i16 (covers SMALLINT)
        if let Ok(Some(v)) = row.try_get_i16(&column) {
            return T::from_postgres_i16(v);
        }

        // Try f64 (covers DOUBLE PRECISION)
        if let Ok(Some(v)) = row.try_get_f64(&column) {
            return T::from_postgres_f64(v);
        }

        // Try f32 (covers REAL)
        if let Ok(Some(v)) = row.try_get_f32(&column) {
            return T::from_postgres_f32(v);
        }

        // Try String (covers TEXT, VARCHAR, etc.)
        if let Ok(Some(ref v)) = row.try_get_string(&column) {
            return T::from_postgres_text(v);
        }

        // Try bytes (covers BYTEA)
        if let Ok(Some(ref v)) = row.try_get_bytes(&column) {
            return T::from_postgres_bytes(v);
        }

        // Try UUID
        #[cfg(feature = "uuid")]
        if let Ok(Some(v)) = row.try_get_uuid(&column) {
            return T::from_postgres_uuid(v);
        }

        // Check for NULL - if all type probes returned None/error, assume NULL
        T::from_postgres_null()
    }

    /// Trait for column reference types (index or name)
    trait ColumnRef: Copy {
        fn to_index(&self) -> Option<usize>;
        fn to_name(&self) -> Option<&str>;
    }

    impl ColumnRef for usize {
        fn to_index(&self) -> Option<usize> {
            Some(*self)
        }
        fn to_name(&self) -> Option<&str> {
            None
        }
    }

    impl ColumnRef for &str {
        fn to_index(&self) -> Option<usize> {
            None
        }
        fn to_name(&self) -> Option<&str> {
            Some(*self)
        }
    }

    /// Internal trait to abstract over postgres/tokio-postgres Row types
    trait PostgresRowLike {
        fn try_get_bool(&self, column: &impl ColumnRef) -> Result<Option<bool>, ()>;
        fn try_get_i16(&self, column: &impl ColumnRef) -> Result<Option<i16>, ()>;
        fn try_get_i32(&self, column: &impl ColumnRef) -> Result<Option<i32>, ()>;
        fn try_get_i64(&self, column: &impl ColumnRef) -> Result<Option<i64>, ()>;
        fn try_get_f32(&self, column: &impl ColumnRef) -> Result<Option<f32>, ()>;
        fn try_get_f64(&self, column: &impl ColumnRef) -> Result<Option<f64>, ()>;
        fn try_get_string(&self, column: &impl ColumnRef) -> Result<Option<String>, ()>;
        fn try_get_bytes(&self, column: &impl ColumnRef) -> Result<Option<Vec<u8>>, ()>;
        #[cfg(feature = "uuid")]
        fn try_get_uuid(&self, column: &impl ColumnRef) -> Result<Option<uuid::Uuid>, ()>;
    }

    // Use tokio_postgres when available, postgres when not
    #[cfg(feature = "tokio-postgres")]
    impl PostgresRowLike for tokio_postgres::Row {
        fn try_get_bool(&self, column: &impl ColumnRef) -> Result<Option<bool>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<bool>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<bool>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_i16(&self, column: &impl ColumnRef) -> Result<Option<i16>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<i16>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<i16>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_i32(&self, column: &impl ColumnRef) -> Result<Option<i32>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<i32>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<i32>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_i64(&self, column: &impl ColumnRef) -> Result<Option<i64>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<i64>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<i64>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_f32(&self, column: &impl ColumnRef) -> Result<Option<f32>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<f32>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<f32>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_f64(&self, column: &impl ColumnRef) -> Result<Option<f64>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<f64>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<f64>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_string(&self, column: &impl ColumnRef) -> Result<Option<String>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<String>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<String>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_bytes(&self, column: &impl ColumnRef) -> Result<Option<Vec<u8>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<u8>>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<u8>>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "uuid")]
        fn try_get_uuid(&self, column: &impl ColumnRef) -> Result<Option<uuid::Uuid>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<uuid::Uuid>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<uuid::Uuid>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }
    }

    #[cfg(feature = "tokio-postgres")]
    impl DrizzleRow for tokio_postgres::Row {
        fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError> {
            convert_column(self, idx)
        }

        fn get_column_by_name<T: FromPostgresValue>(&self, name: &str) -> Result<T, DrizzleError> {
            convert_column(self, name)
        }
    }

    // postgres::Row is a re-export of tokio_postgres::Row, so when both features
    // are enabled, this implementation applies to both. When only postgres-sync
    // is enabled, we need a separate implementation.
    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    impl PostgresRowLike for postgres::Row {
        fn try_get_bool(&self, column: &impl ColumnRef) -> Result<Option<bool>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<bool>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<bool>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_i16(&self, column: &impl ColumnRef) -> Result<Option<i16>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<i16>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<i16>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_i32(&self, column: &impl ColumnRef) -> Result<Option<i32>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<i32>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<i32>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_i64(&self, column: &impl ColumnRef) -> Result<Option<i64>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<i64>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<i64>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_f32(&self, column: &impl ColumnRef) -> Result<Option<f32>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<f32>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<f32>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_f64(&self, column: &impl ColumnRef) -> Result<Option<f64>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<f64>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<f64>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_string(&self, column: &impl ColumnRef) -> Result<Option<String>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<String>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<String>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_bytes(&self, column: &impl ColumnRef) -> Result<Option<Vec<u8>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<u8>>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<u8>>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "uuid")]
        fn try_get_uuid(&self, column: &impl ColumnRef) -> Result<Option<uuid::Uuid>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<uuid::Uuid>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<uuid::Uuid>>(name).map_err(|_| ())
            } else {
                Err(())
            }
        }
    }

    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    impl DrizzleRow for postgres::Row {
        fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError> {
            convert_column(self, idx)
        }

        fn get_column_by_name<T: FromPostgresValue>(&self, name: &str) -> Result<T, DrizzleError> {
            convert_column(self, name)
        }
    }
}

// =============================================================================
// UUID support (when feature enabled)
// =============================================================================

#[cfg(feature = "uuid")]
impl FromPostgresValue for uuid::Uuid {
    fn from_postgres_bool(_value: bool) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bool to UUID".into(),
        ))
    }

    fn from_postgres_i16(_value: i16) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i16 to UUID".into(),
        ))
    }

    fn from_postgres_i32(_value: i32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i32 to UUID".into(),
        ))
    }

    fn from_postgres_i64(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i64 to UUID".into(),
        ))
    }

    fn from_postgres_f32(_value: f32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f32 to UUID".into(),
        ))
    }

    fn from_postgres_f64(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f64 to UUID".into(),
        ))
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        uuid::Uuid::parse_str(value).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UUID string '{}': {}", value, e).into())
        })
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        uuid::Uuid::from_slice(value)
            .map_err(|e| DrizzleError::ConversionError(format!("invalid UUID bytes: {}", e).into()))
    }

    fn from_postgres_uuid(value: uuid::Uuid) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

// =============================================================================
// ArrayVec/ArrayString support (when feature enabled)
// =============================================================================

#[cfg(feature = "arrayvec")]
impl<const N: usize> FromPostgresValue for arrayvec::ArrayString<N> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
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

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
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

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
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

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
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

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
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

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
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

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
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

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
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
impl<const N: usize> FromPostgresValue for arrayvec::ArrayVec<u8, N> {
    fn from_postgres_bool(_value: bool) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bool to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_i16(_value: i16) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i16 to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_i32(_value: i32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i32 to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_i64(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i64 to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_f32(_value: f32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f32 to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_f64(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f64 to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_text(_value: &str) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert TEXT to ArrayVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        arrayvec::ArrayVec::try_from(value).map_err(|_| {
            DrizzleError::ConversionError(
                format!(
                    "Bytes length {} exceeds ArrayVec capacity {}",
                    value.len(),
                    N
                )
                .into(),
            )
        })
    }
}
