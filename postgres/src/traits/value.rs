//! Value conversion traits for PostgreSQL types
//!
//! This module provides the `FromPostgresValue` trait for converting PostgreSQL values
//! to Rust types, and row capability traits for unified access across drivers.
//!
//! This pattern mirrors the SQLite implementation to provide driver-agnostic
//! row conversions for postgres, tokio-postgres, and potentially other drivers.

use crate::values::{OwnedPostgresValue, PostgresValue};
use drizzle_core::error::DrizzleError;
use std::{rc::Rc, sync::Arc};

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

    /// Convert from a JSON value
    #[cfg(feature = "serde")]
    fn from_postgres_json(value: serde_json::Value) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert JSON {} to target type", value).into(),
        ))
    }

    /// Convert from a JSONB value
    #[cfg(feature = "serde")]
    fn from_postgres_jsonb(value: serde_json::Value) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert JSONB {} to target type", value).into(),
        ))
    }

    /// Convert from an ARRAY value
    fn from_postgres_array<'a>(_value: Vec<PostgresValue<'a>>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert ARRAY to target type".into(),
        ))
    }

    /// Convert from a DATE value
    #[cfg(feature = "chrono")]
    fn from_postgres_date(value: chrono::NaiveDate) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert DATE {} to target type", value).into(),
        ))
    }

    /// Convert from a TIME value
    #[cfg(feature = "chrono")]
    fn from_postgres_time(value: chrono::NaiveTime) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIME {} to target type", value).into(),
        ))
    }

    /// Convert from a TIMESTAMP value
    #[cfg(feature = "chrono")]
    fn from_postgres_timestamp(value: chrono::NaiveDateTime) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIMESTAMP {} to target type", value).into(),
        ))
    }

    /// Convert from a TIMESTAMPTZ value
    #[cfg(feature = "chrono")]
    fn from_postgres_timestamptz(
        value: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIMESTAMPTZ {} to target type", value).into(),
        ))
    }

    /// Convert from an INTERVAL value
    #[cfg(feature = "chrono")]
    fn from_postgres_interval(value: chrono::Duration) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert INTERVAL {} to target type", value).into(),
        ))
    }

    /// Convert from an INET value
    #[cfg(feature = "cidr")]
    fn from_postgres_inet(value: cidr::IpInet) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert INET {} to target type", value).into(),
        ))
    }

    /// Convert from a CIDR value
    #[cfg(feature = "cidr")]
    fn from_postgres_cidr(value: cidr::IpCidr) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert CIDR {} to target type", value).into(),
        ))
    }

    /// Convert from a MACADDR value
    #[cfg(feature = "cidr")]
    fn from_postgres_macaddr(value: [u8; 6]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert MACADDR {:?} to target type", value).into(),
        ))
    }

    /// Convert from a MACADDR8 value
    #[cfg(feature = "cidr")]
    fn from_postgres_macaddr8(value: [u8; 8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert MACADDR8 {:?} to target type", value).into(),
        ))
    }

    /// Convert from a POINT value
    #[cfg(feature = "geo-types")]
    fn from_postgres_point(value: geo_types::Point<f64>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert POINT {:?} to target type", value).into(),
        ))
    }

    /// Convert from a PATH value
    #[cfg(feature = "geo-types")]
    fn from_postgres_linestring(value: geo_types::LineString<f64>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert PATH {:?} to target type", value).into(),
        ))
    }

    /// Convert from a BOX value
    #[cfg(feature = "geo-types")]
    fn from_postgres_rect(value: geo_types::Rect<f64>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert BOX {:?} to target type", value).into(),
        ))
    }

    /// Convert from a BIT/VARBIT value
    #[cfg(feature = "bit-vec")]
    fn from_postgres_bitvec(value: bit_vec::BitVec) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert BITVEC {:?} to target type", value).into(),
        ))
    }
}

/// Row capability for index-based extraction.
pub trait DrizzleRowByIndex {
    /// Get a column value by index
    fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError>;
}

/// Row capability for name-based extraction.
pub trait DrizzleRowByName: DrizzleRowByIndex {
    /// Get a column value by name
    fn get_column_by_name<T: FromPostgresValue>(&self, name: &str) -> Result<T, DrizzleError>;
}

fn checked_float_to_int<T>(value: f64, type_name: &str) -> Result<T, DrizzleError>
where
    T: TryFrom<i128>,
    <T as TryFrom<i128>>::Error: core::fmt::Display,
{
    if !value.is_finite() {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-finite float {} to {}", value, type_name).into(),
        ));
    }

    if value.fract() != 0.0 {
        return Err(DrizzleError::ConversionError(
            format!(
                "cannot convert non-integer float {} to {}",
                value, type_name
            )
            .into(),
        ));
    }

    if value < i128::MIN as f64 || value > i128::MAX as f64 {
        return Err(DrizzleError::ConversionError(
            format!("float {} out of range for {}", value, type_name).into(),
        ));
    }

    let int_value = value as i128;
    int_value.try_into().map_err(|e| {
        DrizzleError::ConversionError(
            format!("float {} out of range for {}: {}", value, type_name, e).into(),
        )
    })
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
                    checked_float_to_int(value as f64, stringify!($ty))
                }

                fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
                    checked_float_to_int(value, stringify!($ty))
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
        checked_float_to_int(value as f64, "i16")
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        checked_float_to_int(value, "i16")
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
        checked_float_to_int(value as f64, "i32")
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        checked_float_to_int(value, "i32")
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
        checked_float_to_int(value as f64, "i64")
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        checked_float_to_int(value, "i64")
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

impl FromPostgresValue for Box<String> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Box::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Box::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Box::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Box::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Box::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Box::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Box::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Box::new)
    }
}

impl FromPostgresValue for Rc<String> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Rc::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Rc::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Rc::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Rc::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Rc::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Rc::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Rc::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Rc::new)
    }
}

impl FromPostgresValue for Arc<String> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Arc::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Arc::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Arc::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Arc::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Arc::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Arc::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Arc::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Arc::new)
    }
}

impl FromPostgresValue for Box<str> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(String::into_boxed_str)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(String::into_boxed_str)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(String::into_boxed_str)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(String::into_boxed_str)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(String::into_boxed_str)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(String::into_boxed_str)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(String::into_boxed_str)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(String::into_boxed_str)
    }
}

impl FromPostgresValue for Rc<str> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Rc::from)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Rc::from)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Rc::from)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Rc::from)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Rc::from)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Rc::from)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Rc::from)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Rc::from)
    }
}

impl FromPostgresValue for Arc<str> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Arc::from)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Arc::from)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Arc::from)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Arc::from)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Arc::from)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Arc::from)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Arc::from)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Arc::from)
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

impl FromPostgresValue for Box<Vec<u8>> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(Box::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Box::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Box::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Box::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Box::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Box::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Box::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bytes(value).map(Box::new)
    }
}

impl FromPostgresValue for Rc<Vec<u8>> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(Rc::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Rc::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Rc::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Rc::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Rc::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Rc::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Rc::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bytes(value).map(Rc::new)
    }
}

impl FromPostgresValue for Arc<Vec<u8>> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(Arc::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Arc::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Arc::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Arc::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Arc::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Arc::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Arc::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bytes(value).map(Arc::new)
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

    #[cfg(feature = "uuid")]
    fn from_postgres_uuid(value: uuid::Uuid) -> Result<Self, DrizzleError> {
        T::from_postgres_uuid(value).map(Some)
    }

    #[cfg(feature = "serde")]
    fn from_postgres_json(value: serde_json::Value) -> Result<Self, DrizzleError> {
        T::from_postgres_json(value).map(Some)
    }

    #[cfg(feature = "serde")]
    fn from_postgres_jsonb(value: serde_json::Value) -> Result<Self, DrizzleError> {
        T::from_postgres_jsonb(value).map(Some)
    }

    #[cfg(feature = "chrono")]
    fn from_postgres_date(value: chrono::NaiveDate) -> Result<Self, DrizzleError> {
        T::from_postgres_date(value).map(Some)
    }

    #[cfg(feature = "chrono")]
    fn from_postgres_time(value: chrono::NaiveTime) -> Result<Self, DrizzleError> {
        T::from_postgres_time(value).map(Some)
    }

    #[cfg(feature = "chrono")]
    fn from_postgres_timestamp(value: chrono::NaiveDateTime) -> Result<Self, DrizzleError> {
        T::from_postgres_timestamp(value).map(Some)
    }

    #[cfg(feature = "chrono")]
    fn from_postgres_timestamptz(
        value: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<Self, DrizzleError> {
        T::from_postgres_timestamptz(value).map(Some)
    }

    #[cfg(feature = "chrono")]
    fn from_postgres_interval(value: chrono::Duration) -> Result<Self, DrizzleError> {
        T::from_postgres_interval(value).map(Some)
    }

    #[cfg(feature = "cidr")]
    fn from_postgres_inet(value: cidr::IpInet) -> Result<Self, DrizzleError> {
        T::from_postgres_inet(value).map(Some)
    }

    #[cfg(feature = "cidr")]
    fn from_postgres_cidr(value: cidr::IpCidr) -> Result<Self, DrizzleError> {
        T::from_postgres_cidr(value).map(Some)
    }

    #[cfg(feature = "cidr")]
    fn from_postgres_macaddr(value: [u8; 6]) -> Result<Self, DrizzleError> {
        T::from_postgres_macaddr(value).map(Some)
    }

    #[cfg(feature = "cidr")]
    fn from_postgres_macaddr8(value: [u8; 8]) -> Result<Self, DrizzleError> {
        T::from_postgres_macaddr8(value).map(Some)
    }

    #[cfg(feature = "geo-types")]
    fn from_postgres_point(value: geo_types::Point<f64>) -> Result<Self, DrizzleError> {
        T::from_postgres_point(value).map(Some)
    }

    #[cfg(feature = "geo-types")]
    fn from_postgres_linestring(value: geo_types::LineString<f64>) -> Result<Self, DrizzleError> {
        T::from_postgres_linestring(value).map(Some)
    }

    #[cfg(feature = "geo-types")]
    fn from_postgres_rect(value: geo_types::Rect<f64>) -> Result<Self, DrizzleError> {
        T::from_postgres_rect(value).map(Some)
    }

    #[cfg(feature = "bit-vec")]
    fn from_postgres_bitvec(value: bit_vec::BitVec) -> Result<Self, DrizzleError> {
        T::from_postgres_bitvec(value).map(Some)
    }

    fn from_postgres_array<'a>(value: Vec<PostgresValue<'a>>) -> Result<Self, DrizzleError> {
        T::from_postgres_array(value).map(Some)
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
        if let Some(oid) = row.type_oid(&column) {
            match oid {
                16 => {
                    if let Ok(Some(v)) = row.try_get_bool(&column) {
                        return T::from_postgres_bool(v);
                    }
                }
                20 => {
                    if let Ok(Some(v)) = row.try_get_i64(&column) {
                        return T::from_postgres_i64(v);
                    }
                }
                23 => {
                    if let Ok(Some(v)) = row.try_get_i32(&column) {
                        return T::from_postgres_i32(v);
                    }
                }
                21 => {
                    if let Ok(Some(v)) = row.try_get_i16(&column) {
                        return T::from_postgres_i16(v);
                    }
                }
                701 => {
                    if let Ok(Some(v)) = row.try_get_f64(&column) {
                        return T::from_postgres_f64(v);
                    }
                }
                700 => {
                    if let Ok(Some(v)) = row.try_get_f32(&column) {
                        return T::from_postgres_f32(v);
                    }
                }
                17 => {
                    if let Ok(Some(ref v)) = row.try_get_bytes(&column) {
                        return T::from_postgres_bytes(v);
                    }
                }
                25 | 1043 | 1042 => {
                    if let Ok(Some(ref v)) = row.try_get_string(&column) {
                        return T::from_postgres_text(v);
                    }
                }
                _ => {}
            }
        }

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

        // Try JSON/JSONB
        #[cfg(feature = "serde")]
        if let Ok(Some(v)) = row.try_get_json(&column) {
            return T::from_postgres_json(v);
        }

        // Try chrono types
        #[cfg(feature = "chrono")]
        if let Ok(Some(v)) = row.try_get_date(&column) {
            return T::from_postgres_date(v);
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(v)) = row.try_get_time(&column) {
            return T::from_postgres_time(v);
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(v)) = row.try_get_timestamp(&column) {
            return T::from_postgres_timestamp(v);
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(v)) = row.try_get_timestamptz(&column) {
            return T::from_postgres_timestamptz(v);
        }

        // Try network types
        #[cfg(feature = "cidr")]
        if let Ok(Some(v)) = row.try_get_inet(&column) {
            return T::from_postgres_inet(v);
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(v)) = row.try_get_cidr(&column) {
            return T::from_postgres_cidr(v);
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(v)) = row.try_get_macaddr(&column) {
            return T::from_postgres_macaddr(v);
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(v)) = row.try_get_macaddr8(&column) {
            return T::from_postgres_macaddr8(v);
        }

        // Try geometric types
        #[cfg(feature = "geo-types")]
        if let Ok(Some(v)) = row.try_get_point(&column) {
            return T::from_postgres_point(v);
        }

        #[cfg(feature = "geo-types")]
        if let Ok(Some(v)) = row.try_get_linestring(&column) {
            return T::from_postgres_linestring(v);
        }

        #[cfg(feature = "geo-types")]
        if let Ok(Some(v)) = row.try_get_rect(&column) {
            return T::from_postgres_rect(v);
        }

        // Try bit string types
        #[cfg(feature = "bit-vec")]
        if let Ok(Some(v)) = row.try_get_bitvec(&column) {
            return T::from_postgres_bitvec(v);
        }

        // Try arrays
        if let Ok(Some(values)) = row.try_get_array_bool(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_i16(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_i32(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_i64(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_f32(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_f64(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "uuid")]
        if let Ok(Some(values)) = row.try_get_array_uuid(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "serde")]
        if let Ok(Some(values)) = row.try_get_array_json(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(values)) = row.try_get_array_date(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(values)) = row.try_get_array_time(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(values)) = row.try_get_array_timestamp(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "chrono")]
        if let Ok(Some(values)) = row.try_get_array_timestamptz(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(values)) = row.try_get_array_inet(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(values)) = row.try_get_array_cidr(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(values)) = row.try_get_array_macaddr(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "cidr")]
        if let Ok(Some(values)) = row.try_get_array_macaddr8(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "geo-types")]
        if let Ok(Some(values)) = row.try_get_array_point(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "geo-types")]
        if let Ok(Some(values)) = row.try_get_array_linestring(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "geo-types")]
        if let Ok(Some(values)) = row.try_get_array_rect(&column) {
            return T::from_postgres_array(array_values(values));
        }

        #[cfg(feature = "bit-vec")]
        if let Ok(Some(values)) = row.try_get_array_bitvec(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_bytes(&column) {
            return T::from_postgres_array(array_values(values));
        }

        if let Ok(Some(values)) = row.try_get_array_text(&column) {
            return T::from_postgres_array(array_values(values));
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

    fn array_values<T>(values: Vec<Option<T>>) -> Vec<PostgresValue<'static>>
    where
        T: Into<PostgresValue<'static>>,
    {
        values
            .into_iter()
            .map(|value| value.map(Into::into).unwrap_or(PostgresValue::Null))
            .collect()
    }

    #[cfg(feature = "cidr")]
    fn parse_mac<const N: usize>(value: &str) -> Option<[u8; N]> {
        let mut bytes = [0u8; N];
        let mut parts = value.split(':');

        for slot in &mut bytes {
            let part = parts.next()?;
            if part.len() != 2 {
                return None;
            }
            *slot = u8::from_str_radix(part, 16).ok()?;
        }

        if parts.next().is_some() {
            return None;
        }

        Some(bytes)
    }

    #[cfg(feature = "cidr")]
    fn parse_mac_array<const N: usize>(
        values: Vec<Option<String>>,
    ) -> Result<Vec<Option<[u8; N]>>, ()> {
        let mut parsed = Vec::with_capacity(values.len());
        for value in values {
            match value {
                Some(value) => {
                    let parsed_value = parse_mac::<N>(&value).ok_or(())?;
                    parsed.push(Some(parsed_value));
                }
                None => parsed.push(None),
            }
        }
        Ok(parsed)
    }

    /// Internal trait to abstract over postgres/tokio-postgres Row types
    trait PostgresRowLike {
        fn type_oid(&self, column: &impl ColumnRef) -> Option<u32>;
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
        #[cfg(feature = "serde")]
        fn try_get_json(&self, column: &impl ColumnRef) -> Result<Option<serde_json::Value>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_date(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveDate>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_time(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveTime>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::NaiveDateTime>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_inet(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpInet>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_cidr(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpCidr>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_macaddr(&self, column: &impl ColumnRef) -> Result<Option<[u8; 6]>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_macaddr8(&self, column: &impl ColumnRef) -> Result<Option<[u8; 8]>, ()>;
        #[cfg(feature = "geo-types")]
        fn try_get_point(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Point<f64>>, ()>;
        #[cfg(feature = "geo-types")]
        fn try_get_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::LineString<f64>>, ()>;
        #[cfg(feature = "geo-types")]
        fn try_get_rect(&self, column: &impl ColumnRef)
        -> Result<Option<geo_types::Rect<f64>>, ()>;
        #[cfg(feature = "bit-vec")]
        fn try_get_bitvec(&self, column: &impl ColumnRef) -> Result<Option<bit_vec::BitVec>, ()>;

        fn try_get_array_bool(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bool>>>, ()>;
        fn try_get_array_i16(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i16>>>, ()>;
        fn try_get_array_i32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i32>>>, ()>;
        fn try_get_array_i64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i64>>>, ()>;
        fn try_get_array_f32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f32>>>, ()>;
        fn try_get_array_f64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f64>>>, ()>;
        fn try_get_array_text(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<String>>>, ()>;
        fn try_get_array_bytes(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<Vec<u8>>>>, ()>;
        #[cfg(feature = "uuid")]
        fn try_get_array_uuid(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<uuid::Uuid>>>, ()>;
        #[cfg(feature = "serde")]
        fn try_get_array_json(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<serde_json::Value>>>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_array_date(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDate>>>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_array_time(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveTime>>>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_array_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDateTime>>>, ()>;
        #[cfg(feature = "chrono")]
        fn try_get_array_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_array_inet(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpInet>>>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_array_cidr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpCidr>>>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_array_macaddr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<[u8; 6]>>>, ()>;
        #[cfg(feature = "cidr")]
        fn try_get_array_macaddr8(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<[u8; 8]>>>, ()>;
        #[cfg(feature = "geo-types")]
        fn try_get_array_point(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Point<f64>>>>, ()>;
        #[cfg(feature = "geo-types")]
        fn try_get_array_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::LineString<f64>>>>, ()>;
        #[cfg(feature = "geo-types")]
        fn try_get_array_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Rect<f64>>>>, ()>;
        #[cfg(feature = "bit-vec")]
        fn try_get_array_bitvec(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bit_vec::BitVec>>>, ()>;
    }

    // Use tokio_postgres when available, postgres when not
    #[cfg(feature = "tokio-postgres")]
    impl PostgresRowLike for tokio_postgres::Row {
        fn type_oid(&self, column: &impl ColumnRef) -> Option<u32> {
            let idx = if let Some(idx) = column.to_index() {
                idx
            } else if let Some(name) = column.to_name() {
                self.columns().iter().position(|c| c.name() == name)?
            } else {
                return None;
            };

            self.columns().get(idx).map(|c| c.type_().oid())
        }

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

        #[cfg(feature = "serde")]
        fn try_get_json(&self, column: &impl ColumnRef) -> Result<Option<serde_json::Value>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<serde_json::Value>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<serde_json::Value>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_date(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveDate>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::NaiveDate>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::NaiveDate>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_time(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveTime>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::NaiveTime>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::NaiveTime>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::NaiveDateTime>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::NaiveDateTime>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::NaiveDateTime>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::DateTime<chrono::FixedOffset>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::DateTime<chrono::FixedOffset>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_inet(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpInet>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<cidr::IpInet>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<cidr::IpInet>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_cidr(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpCidr>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<cidr::IpCidr>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<cidr::IpCidr>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_macaddr(&self, column: &impl ColumnRef) -> Result<Option<[u8; 6]>, ()> {
            match self.try_get_string(column) {
                Ok(Some(value)) => parse_mac::<6>(&value).ok_or(()).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_macaddr8(&self, column: &impl ColumnRef) -> Result<Option<[u8; 8]>, ()> {
            match self.try_get_string(column) {
                Ok(Some(value)) => parse_mac::<8>(&value).ok_or(()).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_point(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Point<f64>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<geo_types::Point<f64>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<geo_types::Point<f64>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::LineString<f64>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<geo_types::LineString<f64>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<geo_types::LineString<f64>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Rect<f64>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<geo_types::Rect<f64>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<geo_types::Rect<f64>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_bitvec(&self, column: &impl ColumnRef) -> Result<Option<bit_vec::BitVec>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<bit_vec::BitVec>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<bit_vec::BitVec>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_bool(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bool>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<bool>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<bool>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_i16(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i16>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<i16>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<i16>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_i32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i32>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<i32>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<i32>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_i64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i64>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<i64>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<i64>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_f32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f32>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<f32>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<f32>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_f64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f64>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<f64>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<f64>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_text(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<String>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<String>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<String>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_bytes(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<Vec<u8>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<Vec<u8>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<Vec<u8>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "uuid")]
        fn try_get_array_uuid(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<uuid::Uuid>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<uuid::Uuid>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<uuid::Uuid>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "serde")]
        fn try_get_array_json(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<serde_json::Value>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<serde_json::Value>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<serde_json::Value>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_date(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDate>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDate>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDate>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_time(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveTime>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveTime>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveTime>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDateTime>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDateTime>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDateTime>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_inet(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpInet>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<cidr::IpInet>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<cidr::IpInet>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_cidr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpCidr>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<cidr::IpCidr>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<cidr::IpCidr>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_macaddr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<[u8; 6]>>>, ()> {
            match self.try_get_array_text(column) {
                Ok(Some(values)) => parse_mac_array::<6>(values).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_macaddr8(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<[u8; 8]>>>, ()> {
            match self.try_get_array_text(column) {
                Ok(Some(values)) => parse_mac_array::<8>(values).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_point(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Point<f64>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<geo_types::Point<f64>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<geo_types::Point<f64>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::LineString<f64>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<geo_types::LineString<f64>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<geo_types::LineString<f64>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Rect<f64>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<geo_types::Rect<f64>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<geo_types::Rect<f64>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_array_bitvec(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bit_vec::BitVec>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<bit_vec::BitVec>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<bit_vec::BitVec>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }
    }

    #[cfg(feature = "tokio-postgres")]
    impl DrizzleRowByIndex for tokio_postgres::Row {
        fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError> {
            convert_column(self, idx)
        }
    }

    #[cfg(feature = "tokio-postgres")]
    impl DrizzleRowByName for tokio_postgres::Row {
        fn get_column_by_name<T: FromPostgresValue>(&self, name: &str) -> Result<T, DrizzleError> {
            convert_column(self, name)
        }
    }

    // postgres::Row is a re-export of tokio_postgres::Row, so when both features
    // are enabled, this implementation applies to both. When only postgres-sync
    // is enabled, we need a separate implementation.
    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    impl PostgresRowLike for postgres::Row {
        fn type_oid(&self, column: &impl ColumnRef) -> Option<u32> {
            let idx = if let Some(idx) = column.to_index() {
                idx
            } else if let Some(name) = column.to_name() {
                self.columns().iter().position(|c| c.name() == name)?
            } else {
                return None;
            };

            self.columns().get(idx).map(|c| c.type_().oid())
        }

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

        #[cfg(feature = "serde")]
        fn try_get_json(&self, column: &impl ColumnRef) -> Result<Option<serde_json::Value>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<serde_json::Value>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<serde_json::Value>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_date(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveDate>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::NaiveDate>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::NaiveDate>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_time(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveTime>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::NaiveTime>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::NaiveTime>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::NaiveDateTime>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::NaiveDateTime>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::NaiveDateTime>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<chrono::DateTime<chrono::FixedOffset>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<chrono::DateTime<chrono::FixedOffset>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_inet(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpInet>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<cidr::IpInet>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<cidr::IpInet>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_cidr(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpCidr>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<cidr::IpCidr>>(idx).map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<cidr::IpCidr>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_macaddr(&self, column: &impl ColumnRef) -> Result<Option<[u8; 6]>, ()> {
            match self.try_get_string(column) {
                Ok(Some(value)) => parse_mac::<6>(&value).ok_or(()).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_macaddr8(&self, column: &impl ColumnRef) -> Result<Option<[u8; 8]>, ()> {
            match self.try_get_string(column) {
                Ok(Some(value)) => parse_mac::<8>(&value).ok_or(()).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_point(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Point<f64>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<geo_types::Point<f64>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<geo_types::Point<f64>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::LineString<f64>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<geo_types::LineString<f64>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<geo_types::LineString<f64>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Rect<f64>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<geo_types::Rect<f64>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<geo_types::Rect<f64>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_bitvec(&self, column: &impl ColumnRef) -> Result<Option<bit_vec::BitVec>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<bit_vec::BitVec>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<bit_vec::BitVec>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_bool(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bool>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<bool>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<bool>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_i16(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i16>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<i16>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<i16>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_i32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i32>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<i32>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<i32>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_i64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i64>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<i64>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<i64>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_f32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f32>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<f32>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<f32>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_f64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f64>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<f64>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<f64>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_text(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<String>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<String>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<String>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        fn try_get_array_bytes(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<Vec<u8>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<Vec<u8>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<Vec<u8>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "uuid")]
        fn try_get_array_uuid(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<uuid::Uuid>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<uuid::Uuid>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<uuid::Uuid>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "serde")]
        fn try_get_array_json(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<serde_json::Value>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<serde_json::Value>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<serde_json::Value>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_date(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDate>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDate>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDate>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_time(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveTime>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveTime>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveTime>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDateTime>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDateTime>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::NaiveDateTime>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_inet(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpInet>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<cidr::IpInet>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<cidr::IpInet>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_cidr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpCidr>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<cidr::IpCidr>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<cidr::IpCidr>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_macaddr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<[u8; 6]>>>, ()> {
            match self.try_get_array_text(column) {
                Ok(Some(values)) => parse_mac_array::<6>(values).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_macaddr8(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<[u8; 8]>>>, ()> {
            match self.try_get_array_text(column) {
                Ok(Some(values)) => parse_mac_array::<8>(values).map(Some),
                Ok(None) => Ok(None),
                Err(()) => Err(()),
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_point(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Point<f64>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<geo_types::Point<f64>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<geo_types::Point<f64>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::LineString<f64>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<geo_types::LineString<f64>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<geo_types::LineString<f64>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Rect<f64>>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<geo_types::Rect<f64>>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<geo_types::Rect<f64>>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_array_bitvec(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bit_vec::BitVec>>>, ()> {
            if let Some(idx) = column.to_index() {
                self.try_get::<_, Option<Vec<Option<bit_vec::BitVec>>>>(idx)
                    .map_err(|_| ())
            } else if let Some(name) = column.to_name() {
                self.try_get::<_, Option<Vec<Option<bit_vec::BitVec>>>>(name)
                    .map_err(|_| ())
            } else {
                Err(())
            }
        }
    }

    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    impl DrizzleRowByIndex for postgres::Row {
        fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError> {
            convert_column(self, idx)
        }
    }

    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    impl DrizzleRowByName for postgres::Row {
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

macro_rules! impl_from_postgres_value_errors {
    ($target:expr) => {
        fn from_postgres_bool(_value: bool) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert bool to {}", $target).into(),
            ))
        }

        fn from_postgres_i16(_value: i16) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert i16 to {}", $target).into(),
            ))
        }

        fn from_postgres_i32(_value: i32) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert i32 to {}", $target).into(),
            ))
        }

        fn from_postgres_i64(_value: i64) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert i64 to {}", $target).into(),
            ))
        }

        fn from_postgres_f32(_value: f32) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert f32 to {}", $target).into(),
            ))
        }

        fn from_postgres_f64(_value: f64) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert f64 to {}", $target).into(),
            ))
        }

        fn from_postgres_text(_value: &str) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert text to {}", $target).into(),
            ))
        }

        fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
            Err(DrizzleError::ConversionError(
                format!("cannot convert bytes to {}", $target).into(),
            ))
        }
    };
}

// =============================================================================
// PostgresEnum support
// =============================================================================

impl<T> FromPostgresValue for T
where
    T: super::PostgresEnum,
{
    fn from_postgres_bool(_value: bool) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bool to PostgresEnum".into(),
        ))
    }

    fn from_postgres_i16(_value: i16) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i16 to PostgresEnum".into(),
        ))
    }

    fn from_postgres_i32(_value: i32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i32 to PostgresEnum".into(),
        ))
    }

    fn from_postgres_i64(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i64 to PostgresEnum".into(),
        ))
    }

    fn from_postgres_f32(_value: f32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f32 to PostgresEnum".into(),
        ))
    }

    fn from_postgres_f64(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f64 to PostgresEnum".into(),
        ))
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        T::try_from_str(value)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        let s = std::str::from_utf8(value).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 for enum: {}", e).into())
        })?;
        T::try_from_str(s)
    }
}

// =============================================================================
// ARRAY support
// =============================================================================

impl<'a> FromPostgresValue for Vec<PostgresValue<'a>> {
    impl_from_postgres_value_errors!("ARRAY");

    fn from_postgres_array<'b>(value: Vec<PostgresValue<'b>>) -> Result<Self, DrizzleError> {
        let values = value
            .into_iter()
            .map(OwnedPostgresValue::from)
            .map(PostgresValue::from)
            .collect();
        Ok(values)
    }
}

impl FromPostgresValue for Vec<OwnedPostgresValue> {
    impl_from_postgres_value_errors!("ARRAY");

    fn from_postgres_array<'a>(value: Vec<PostgresValue<'a>>) -> Result<Self, DrizzleError> {
        Ok(value.into_iter().map(OwnedPostgresValue::from).collect())
    }
}

// =============================================================================
// JSON support (when feature enabled)
// =============================================================================

#[cfg(feature = "serde")]
impl FromPostgresValue for serde_json::Value {
    impl_from_postgres_value_errors!("JSON");

    fn from_postgres_json(value: serde_json::Value) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_jsonb(value: serde_json::Value) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

// =============================================================================
// Chrono support (when feature enabled)
// =============================================================================

#[cfg(feature = "chrono")]
impl FromPostgresValue for chrono::NaiveDate {
    impl_from_postgres_value_errors!("NaiveDate");

    fn from_postgres_date(value: chrono::NaiveDate) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "chrono")]
impl FromPostgresValue for chrono::NaiveTime {
    impl_from_postgres_value_errors!("NaiveTime");

    fn from_postgres_time(value: chrono::NaiveTime) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "chrono")]
impl FromPostgresValue for chrono::NaiveDateTime {
    impl_from_postgres_value_errors!("NaiveDateTime");

    fn from_postgres_timestamp(value: chrono::NaiveDateTime) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "chrono")]
impl FromPostgresValue for chrono::DateTime<chrono::FixedOffset> {
    impl_from_postgres_value_errors!("DateTime<FixedOffset>");

    fn from_postgres_timestamptz(
        value: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "chrono")]
impl FromPostgresValue for chrono::DateTime<chrono::Utc> {
    impl_from_postgres_value_errors!("DateTime<Utc>");

    fn from_postgres_timestamptz(
        value: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<Self, DrizzleError> {
        Ok(value.with_timezone(&chrono::Utc))
    }
}

#[cfg(feature = "chrono")]
impl FromPostgresValue for chrono::Duration {
    impl_from_postgres_value_errors!("Duration");

    fn from_postgres_interval(value: chrono::Duration) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

// =============================================================================
// Network types (when feature enabled)
// =============================================================================

#[cfg(feature = "cidr")]
impl FromPostgresValue for cidr::IpInet {
    impl_from_postgres_value_errors!("IpInet");

    fn from_postgres_inet(value: cidr::IpInet) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "cidr")]
impl FromPostgresValue for cidr::IpCidr {
    impl_from_postgres_value_errors!("IpCidr");

    fn from_postgres_cidr(value: cidr::IpCidr) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "cidr")]
impl FromPostgresValue for [u8; 6] {
    impl_from_postgres_value_errors!("MACADDR");

    fn from_postgres_macaddr(value: [u8; 6]) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "cidr")]
impl FromPostgresValue for [u8; 8] {
    impl_from_postgres_value_errors!("MACADDR8");

    fn from_postgres_macaddr8(value: [u8; 8]) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

// =============================================================================
// Geometric types (when feature enabled)
// =============================================================================

#[cfg(feature = "geo-types")]
impl FromPostgresValue for geo_types::Point<f64> {
    impl_from_postgres_value_errors!("Point");

    fn from_postgres_point(value: geo_types::Point<f64>) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "geo-types")]
impl FromPostgresValue for geo_types::LineString<f64> {
    impl_from_postgres_value_errors!("LineString");

    fn from_postgres_linestring(value: geo_types::LineString<f64>) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "geo-types")]
impl FromPostgresValue for geo_types::Rect<f64> {
    impl_from_postgres_value_errors!("Rect");

    fn from_postgres_rect(value: geo_types::Rect<f64>) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

// =============================================================================
// Bit string types (when feature enabled)
// =============================================================================

#[cfg(feature = "bit-vec")]
impl FromPostgresValue for bit_vec::BitVec {
    impl_from_postgres_value_errors!("BitVec");

    fn from_postgres_bitvec(value: bit_vec::BitVec) -> Result<Self, DrizzleError> {
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
