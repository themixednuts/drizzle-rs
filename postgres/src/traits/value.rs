//! Value conversion traits for `PostgreSQL` types
//!
//! This module provides the `FromPostgresValue` trait for converting `PostgreSQL` values
//! to Rust types, and row capability traits for unified access across drivers.
//!
//! This pattern mirrors the `SQLite` implementation to provide driver-agnostic
//! row conversions for postgres, tokio-postgres, and potentially other drivers.

use crate::prelude::*;
use crate::values::{OwnedPostgresValue, PostgresValue};
use drizzle_core::error::DrizzleError;

/// Trait for types that can be converted from `PostgreSQL` values.
///
/// `PostgreSQL` has many types, but this trait focuses on the core conversions:
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
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError>;

    /// Convert from a 16-bit integer value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError>;

    /// Convert from a 32-bit integer value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError>;

    /// Convert from a 64-bit integer value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError>;

    /// Convert from a 32-bit float value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError>;

    /// Convert from a 64-bit float value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError>;

    /// Convert from a text/string value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError>;

    /// Convert from a binary/bytea value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the value cannot be represented as the target type.
    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError>;

    /// Convert from a NULL value (default returns error)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] unless the implementor treats NULL as a valid value.
    fn from_postgres_null() -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "unexpected NULL value".into(),
        ))
    }

    /// Convert from a UUID value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a UUID.
    #[cfg(feature = "uuid")]
    fn from_postgres_uuid(value: uuid::Uuid) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert UUID {value} to target type").into(),
        ))
    }

    /// Convert from a JSON value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a JSON value.
    #[cfg(feature = "serde")]
    fn from_postgres_json(value: serde_json::Value) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert JSON {value} to target type").into(),
        ))
    }

    /// Convert from a JSONB value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a JSONB value.
    #[cfg(feature = "serde")]
    fn from_postgres_jsonb(value: serde_json::Value) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert JSONB {value} to target type").into(),
        ))
    }

    /// Convert from an ARRAY value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent an ARRAY value.
    fn from_postgres_array(_value: Vec<PostgresValue<'_>>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert ARRAY to target type".into(),
        ))
    }

    /// Convert from a DATE value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a DATE.
    #[cfg(feature = "chrono")]
    fn from_postgres_date(value: chrono::NaiveDate) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert DATE {value} to target type").into(),
        ))
    }

    /// Convert from a TIME value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a TIME.
    #[cfg(feature = "chrono")]
    fn from_postgres_time(value: chrono::NaiveTime) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIME {value} to target type").into(),
        ))
    }

    /// Convert from a TIMESTAMP value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a TIMESTAMP.
    #[cfg(feature = "chrono")]
    fn from_postgres_timestamp(value: chrono::NaiveDateTime) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIMESTAMP {value} to target type").into(),
        ))
    }

    /// Convert from a TIMESTAMPTZ value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a TIMESTAMPTZ.
    #[cfg(feature = "chrono")]
    fn from_postgres_timestamptz(
        value: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIMESTAMPTZ {value} to target type").into(),
        ))
    }

    /// Convert from an INTERVAL value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent an INTERVAL.
    #[cfg(feature = "chrono")]
    fn from_postgres_interval(value: chrono::Duration) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert INTERVAL {value} to target type").into(),
        ))
    }

    /// Convert from a DATE value (time crate)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a DATE.
    #[cfg(feature = "time")]
    fn from_postgres_time_date(value: time::Date) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert DATE (time) {value:?} to target type").into(),
        ))
    }

    /// Convert from a TIME value (time crate)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a TIME.
    #[cfg(feature = "time")]
    fn from_postgres_time_time(value: time::Time) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIME (time) {value:?} to target type").into(),
        ))
    }

    /// Convert from a TIMESTAMP value (time crate)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a TIMESTAMP.
    #[cfg(feature = "time")]
    fn from_postgres_time_timestamp(value: time::PrimitiveDateTime) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIMESTAMP (time) {value:?} to target type").into(),
        ))
    }

    /// Convert from a TIMESTAMPTZ value (time crate)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a TIMESTAMPTZ.
    #[cfg(feature = "time")]
    fn from_postgres_time_timestamptz(value: time::OffsetDateTime) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert TIMESTAMPTZ (time) {value:?} to target type").into(),
        ))
    }

    /// Convert from an INTERVAL value (time crate)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent an INTERVAL.
    #[cfg(feature = "time")]
    fn from_postgres_time_interval(value: time::Duration) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert INTERVAL (time) {value:?} to target type").into(),
        ))
    }

    /// Convert from an INET value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent an INET address.
    #[cfg(feature = "cidr")]
    fn from_postgres_inet(value: cidr::IpInet) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert INET {value} to target type").into(),
        ))
    }

    /// Convert from a CIDR value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a CIDR network.
    #[cfg(feature = "cidr")]
    fn from_postgres_cidr(value: cidr::IpCidr) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert CIDR {value} to target type").into(),
        ))
    }

    /// Convert from a MACADDR value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a MAC address.
    #[cfg(feature = "cidr")]
    fn from_postgres_macaddr(value: [u8; 6]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert MACADDR {value:?} to target type").into(),
        ))
    }

    /// Convert from a MACADDR8 value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent an 8-byte MAC address.
    #[cfg(feature = "cidr")]
    fn from_postgres_macaddr8(value: [u8; 8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert MACADDR8 {value:?} to target type").into(),
        ))
    }

    /// Convert from a POINT value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a POINT.
    #[cfg(feature = "geo-types")]
    fn from_postgres_point(value: geo_types::Point<f64>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert POINT {value:?} to target type").into(),
        ))
    }

    /// Convert from a PATH value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a PATH.
    #[cfg(feature = "geo-types")]
    fn from_postgres_linestring(value: geo_types::LineString<f64>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert PATH {value:?} to target type").into(),
        ))
    }

    /// Convert from a BOX value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a BOX.
    #[cfg(feature = "geo-types")]
    fn from_postgres_rect(value: geo_types::Rect<f64>) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert BOX {value:?} to target type").into(),
        ))
    }

    /// Convert from a BIT/VARBIT value
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the target type cannot represent a bit vector.
    #[cfg(feature = "bit-vec")]
    fn from_postgres_bitvec(value: bit_vec::BitVec) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            format!("cannot convert BITVEC {value:?} to target type").into(),
        ))
    }
}

/// Row capability for index-based extraction.
pub trait DrizzleRowByIndex {
    /// Get a column value by index
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the column is out of bounds, the value is NULL for a non-nullable target,
    /// or the stored value cannot be converted into `T`.
    fn get_column<T: FromPostgresValue>(&self, idx: usize) -> Result<T, DrizzleError>;
}

/// Row capability for name-based extraction.
pub trait DrizzleRowByName: DrizzleRowByIndex {
    /// Get a column value by name
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the column name is unknown, the value is NULL for a non-nullable target,
    /// or the stored value cannot be converted into `T`.
    fn get_column_by_name<T: FromPostgresValue>(&self, name: &str) -> Result<T, DrizzleError>;
}

fn checked_float_to_int<T>(value: f64, type_name: &str) -> Result<T, DrizzleError>
where
    T: TryFrom<i128>,
    <T as TryFrom<i128>>::Error: core::fmt::Display,
{
    if !value.is_finite() {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-finite float {value} to {type_name}").into(),
        ));
    }

    if value % 1.0 != 0.0 {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-integer float {value} to {type_name}").into(),
        ));
    }

    // Convert via decimal string: `{value:.0}` forces fixed-point integer
    // formatting, and i128 parse rejects anything outside ±i128 range.
    // This avoids both the lossy `as i128` cast and the `i128::MAX as f64`
    // bounds-check cast (which clippy flags for precision loss).
    let int_value: i128 = format!("{value:.0}").parse().map_err(|e| {
        DrizzleError::ConversionError(
            format!("float {value} out of range for {type_name}: {e}").into(),
        )
    })?;
    int_value.try_into().map_err(|e| {
        DrizzleError::ConversionError(
            format!("float {value} out of range for {type_name}: {e}").into(),
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
                format!("cannot parse '{value}' as bool").into(),
            )),
        }
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to bool".into(),
        ))
    }
}

/// Macro to implement `FromPostgresValue` for integer types
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
                    checked_float_to_int(f64::from(value), stringify!($ty))
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
        Ok(Self::from(value))
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(format!("i32 {value} out of range for i16: {e}").into())
        })
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(format!("i64 {value} out of range for i16: {e}").into())
        })
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        checked_float_to_int(f64::from(value), "i16")
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        checked_float_to_int(value, "i16")
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{value}' as i16: {e}").into())
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
        Ok(Self::from(value))
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        value.try_into().map_err(|e| {
            DrizzleError::ConversionError(format!("i64 {value} out of range for i32: {e}").into())
        })
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        checked_float_to_int(f64::from(value), "i32")
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        checked_float_to_int(value, "i32")
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{value}' as i32: {e}").into())
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
        Ok(Self::from(value))
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        checked_float_to_int(f64::from(value), "i64")
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        checked_float_to_int(value, "i64")
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot parse '{value}' as i64: {e}").into())
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

/// Parses an integer via its decimal string form into a float.
///
/// This avoids the `cast_precision_loss` lint for `i32 as f32` / `i64 as f32` /
/// `i64 as f64` while preserving round-to-nearest semantics.
#[inline]
fn int_to_float<I: core::fmt::Display, F: core::str::FromStr>(value: I) -> Result<F, DrizzleError>
where
    <F as core::str::FromStr>::Err: core::fmt::Display,
{
    let s = format!("{value}");
    s.parse::<F>().map_err(|e| {
        DrizzleError::ConversionError(format!("cannot convert '{s}' to float: {e}").into())
    })
}

impl FromPostgresValue for f64 {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(if value { 1.0 } else { 0.0 })
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        // Decimal round-trip preserves round-to-nearest without an `as` cast.
        int_to_float::<_, Self>(value)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e: core::num::ParseFloatError| {
            DrizzleError::ConversionError(format!("cannot parse '{value}' as f64: {e}").into())
        })
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to f64".into(),
        ))
    }
}

impl FromPostgresValue for f32 {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(if value { 1.0 } else { 0.0 })
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        int_to_float::<_, Self>(value)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        int_to_float::<_, Self>(value)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(value)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        // Round via decimal round-trip instead of the lossy `as` cast.
        let s = format!("{value}");
        s.parse::<Self>().map_err(|e| {
            DrizzleError::ConversionError(format!("cannot convert '{s}' to f32: {e}").into())
        })
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        value.parse().map_err(|e: core::num::ParseFloatError| {
            DrizzleError::ConversionError(format!("cannot parse '{value}' as f32: {e}").into())
        })
    }

    fn from_postgres_bytes(_value: &[u8]) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bytes to f32".into(),
        ))
    }
}

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
        Self::from_utf8(value.to_vec()).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 in bytes: {e}").into())
        })
    }
}

impl FromPostgresValue for Box<String> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Self::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Self::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Self::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Self::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Self::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Self::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Self::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Self::new)
    }
}

impl FromPostgresValue for Rc<String> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Self::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Self::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Self::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Self::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Self::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Self::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Self::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Self::new)
    }
}

impl FromPostgresValue for Arc<String> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Self::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Self::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Self::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Self::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Self::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Self::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Self::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Self::new)
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
        String::from_postgres_bool(value).map(Self::from)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Self::from)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Self::from)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Self::from)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Self::from)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Self::from)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Self::from)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Self::from)
    }
}

impl FromPostgresValue for Arc<str> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        String::from_postgres_bool(value).map(Self::from)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        String::from_postgres_i16(value).map(Self::from)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        String::from_postgres_i32(value).map(Self::from)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        String::from_postgres_i64(value).map(Self::from)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        String::from_postgres_f32(value).map(Self::from)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        String::from_postgres_f64(value).map(Self::from)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        String::from_postgres_text(value).map(Self::from)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        String::from_postgres_bytes(value).map(Self::from)
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
        Vec::<u8>::from_postgres_bool(value).map(Self::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Self::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Self::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Self::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Self::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Self::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Self::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bytes(value).map(Self::new)
    }
}

impl FromPostgresValue for Rc<Vec<u8>> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(Self::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Self::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Self::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Self::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Self::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Self::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Self::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bytes(value).map(Self::new)
    }
}

impl FromPostgresValue for Arc<Vec<u8>> {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(Self::new)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Self::new)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Self::new)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Self::new)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Self::new)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Self::new)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Self::new)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bytes(value).map(Self::new)
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

    #[cfg(feature = "time")]
    fn from_postgres_time_date(value: time::Date) -> Result<Self, DrizzleError> {
        T::from_postgres_time_date(value).map(Some)
    }

    #[cfg(feature = "time")]
    fn from_postgres_time_time(value: time::Time) -> Result<Self, DrizzleError> {
        T::from_postgres_time_time(value).map(Some)
    }

    #[cfg(feature = "time")]
    fn from_postgres_time_timestamp(value: time::PrimitiveDateTime) -> Result<Self, DrizzleError> {
        T::from_postgres_time_timestamp(value).map(Some)
    }

    #[cfg(feature = "time")]
    fn from_postgres_time_timestamptz(value: time::OffsetDateTime) -> Result<Self, DrizzleError> {
        T::from_postgres_time_timestamptz(value).map(Some)
    }

    #[cfg(feature = "time")]
    fn from_postgres_time_interval(value: time::Duration) -> Result<Self, DrizzleError> {
        T::from_postgres_time_interval(value).map(Some)
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

    fn from_postgres_array(value: Vec<PostgresValue<'_>>) -> Result<Self, DrizzleError> {
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
    use super::{
        DrizzleError, DrizzleRowByIndex, DrizzleRowByName, FromPostgresValue, PostgresValue,
        String, Vec,
    };

    // Helper function to convert a row value to our type
    // This uses the native driver's try_get functionality
    fn convert_column<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: impl ColumnRef,
    ) -> Result<T, DrizzleError> {
        if let Some(result) = try_oid_dispatch::<T, R>(row, &column) {
            return result;
        }
        if let Some(result) = try_scalar_fallbacks::<T, R>(row, &column) {
            return result;
        }
        if let Some(result) = try_array_fallbacks::<T, R>(row, &column) {
            return result;
        }

        // If all type probes returned None/error, assume NULL.
        T::from_postgres_null()
    }

    /// `PostgreSQL` OID fast-path: when the column's declared type matches a
    /// known primitive OID, decode directly without running the full fallback
    /// chain.
    fn try_oid_dispatch<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        let oid = row.type_oid(column)?;
        match oid {
            16 => row
                .try_get_bool(column)
                .ok()
                .flatten()
                .map(T::from_postgres_bool),
            20 => row
                .try_get_i64(column)
                .ok()
                .flatten()
                .map(T::from_postgres_i64),
            23 => row
                .try_get_i32(column)
                .ok()
                .flatten()
                .map(T::from_postgres_i32),
            21 => row
                .try_get_i16(column)
                .ok()
                .flatten()
                .map(T::from_postgres_i16),
            701 => row
                .try_get_f64(column)
                .ok()
                .flatten()
                .map(T::from_postgres_f64),
            700 => row
                .try_get_f32(column)
                .ok()
                .flatten()
                .map(T::from_postgres_f32),
            17 => row
                .try_get_bytes(column)
                .ok()
                .flatten()
                .map(|v| T::from_postgres_bytes(&v)),
            25 | 1043 | 1042 => row
                .try_get_string(column)
                .ok()
                .flatten()
                .map(|v| T::from_postgres_text(&v)),
            _ => None,
        }
    }

    /// Scalar fallback chain: try each supported primitive / library type in
    /// priority order, returning the first that decodes successfully.
    fn try_scalar_fallbacks<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        try_scalar_primitives::<T, R>(row, column)
            .or_else(|| try_scalar_uuid::<T, R>(row, column))
            .or_else(|| try_scalar_json::<T, R>(row, column))
            .or_else(|| try_scalar_chrono::<T, R>(row, column))
            .or_else(|| try_scalar_cidr::<T, R>(row, column))
            .or_else(|| try_scalar_geo::<T, R>(row, column))
            .or_else(|| try_scalar_bitvec::<T, R>(row, column))
    }

    /// Primitive scalar fallbacks: bool / integers / floats / text / bytes.
    fn try_scalar_primitives<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(v)) = row.try_get_bool(column) {
            return Some(T::from_postgres_bool(v));
        }
        if let Ok(Some(v)) = row.try_get_i64(column) {
            return Some(T::from_postgres_i64(v));
        }
        if let Ok(Some(v)) = row.try_get_i32(column) {
            return Some(T::from_postgres_i32(v));
        }
        if let Ok(Some(v)) = row.try_get_i16(column) {
            return Some(T::from_postgres_i16(v));
        }
        if let Ok(Some(v)) = row.try_get_f64(column) {
            return Some(T::from_postgres_f64(v));
        }
        if let Ok(Some(v)) = row.try_get_f32(column) {
            return Some(T::from_postgres_f32(v));
        }
        if let Ok(Some(ref v)) = row.try_get_string(column) {
            return Some(T::from_postgres_text(v));
        }
        if let Ok(Some(ref v)) = row.try_get_bytes(column) {
            return Some(T::from_postgres_bytes(v));
        }
        None
    }

    #[cfg(feature = "uuid")]
    fn try_scalar_uuid<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        row.try_get_uuid(column)
            .ok()
            .flatten()
            .map(T::from_postgres_uuid)
    }

    #[cfg(not(feature = "uuid"))]
    const fn try_scalar_uuid<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "serde")]
    fn try_scalar_json<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        row.try_get_json(column)
            .ok()
            .flatten()
            .map(T::from_postgres_json)
    }

    #[cfg(not(feature = "serde"))]
    const fn try_scalar_json<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "chrono")]
    fn try_scalar_chrono<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(v)) = row.try_get_date(column) {
            return Some(T::from_postgres_date(v));
        }
        if let Ok(Some(v)) = row.try_get_time(column) {
            return Some(T::from_postgres_time(v));
        }
        if let Ok(Some(v)) = row.try_get_timestamp(column) {
            return Some(T::from_postgres_timestamp(v));
        }
        if let Ok(Some(v)) = row.try_get_timestamptz(column) {
            return Some(T::from_postgres_timestamptz(v));
        }
        None
    }

    #[cfg(not(feature = "chrono"))]
    const fn try_scalar_chrono<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "cidr")]
    fn try_scalar_cidr<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(v)) = row.try_get_inet(column) {
            return Some(T::from_postgres_inet(v));
        }
        if let Ok(Some(v)) = row.try_get_cidr(column) {
            return Some(T::from_postgres_cidr(v));
        }
        if let Ok(Some(v)) = row.try_get_macaddr(column) {
            return Some(T::from_postgres_macaddr(v));
        }
        if let Ok(Some(v)) = row.try_get_macaddr8(column) {
            return Some(T::from_postgres_macaddr8(v));
        }
        None
    }

    #[cfg(not(feature = "cidr"))]
    const fn try_scalar_cidr<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "geo-types")]
    fn try_scalar_geo<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(v)) = row.try_get_point(column) {
            return Some(T::from_postgres_point(v));
        }
        if let Ok(Some(v)) = row.try_get_linestring(column) {
            return Some(T::from_postgres_linestring(v));
        }
        if let Ok(Some(v)) = row.try_get_rect(column) {
            return Some(T::from_postgres_rect(v));
        }
        None
    }

    #[cfg(not(feature = "geo-types"))]
    const fn try_scalar_geo<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "bit-vec")]
    fn try_scalar_bitvec<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        row.try_get_bitvec(column)
            .ok()
            .flatten()
            .map(T::from_postgres_bitvec)
    }

    #[cfg(not(feature = "bit-vec"))]
    const fn try_scalar_bitvec<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    /// Array fallback chain: try each supported array element type in priority
    /// order.
    fn try_array_fallbacks<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        try_array_primitives::<T, R>(row, column)
            .or_else(|| try_array_uuid::<T, R>(row, column))
            .or_else(|| try_array_json::<T, R>(row, column))
            .or_else(|| try_array_chrono::<T, R>(row, column))
            .or_else(|| try_array_cidr::<T, R>(row, column))
            .or_else(|| try_array_geo::<T, R>(row, column))
            .or_else(|| try_array_bitvec::<T, R>(row, column))
            .or_else(|| try_array_text_bytes::<T, R>(row, column))
    }

    /// Primitive array fallbacks: bool / integers / floats (priority order).
    fn try_array_primitives<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(values)) = row.try_get_array_bool(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_i16(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_i32(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_i64(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_f32(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_f64(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        None
    }

    #[cfg(feature = "uuid")]
    fn try_array_uuid<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        row.try_get_array_uuid(column)
            .ok()
            .flatten()
            .map(|values| T::from_postgres_array(array_values(values)))
    }

    #[cfg(not(feature = "uuid"))]
    const fn try_array_uuid<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "serde")]
    fn try_array_json<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        row.try_get_array_json(column)
            .ok()
            .flatten()
            .map(|values| T::from_postgres_array(array_values(values)))
    }

    #[cfg(not(feature = "serde"))]
    const fn try_array_json<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "chrono")]
    fn try_array_chrono<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(values)) = row.try_get_array_date(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_time(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_timestamp(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_timestamptz(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        None
    }

    #[cfg(not(feature = "chrono"))]
    const fn try_array_chrono<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "cidr")]
    fn try_array_cidr<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(values)) = row.try_get_array_inet(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_cidr(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_macaddr(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_macaddr8(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        None
    }

    #[cfg(not(feature = "cidr"))]
    const fn try_array_cidr<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "geo-types")]
    fn try_array_geo<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(values)) = row.try_get_array_point(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_linestring(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_rect(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        None
    }

    #[cfg(not(feature = "geo-types"))]
    const fn try_array_geo<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    #[cfg(feature = "bit-vec")]
    fn try_array_bitvec<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        row.try_get_array_bitvec(column)
            .ok()
            .flatten()
            .map(|values| T::from_postgres_array(array_values(values)))
    }

    #[cfg(not(feature = "bit-vec"))]
    const fn try_array_bitvec<T: FromPostgresValue, R: PostgresRowLike>(
        _row: &R,
        _column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        None
    }

    /// Final bytes/text array fallback (lowest priority).
    fn try_array_text_bytes<T: FromPostgresValue, R: PostgresRowLike>(
        row: &R,
        column: &impl ColumnRef,
    ) -> Option<Result<T, DrizzleError>> {
        if let Ok(Some(values)) = row.try_get_array_bytes(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        if let Ok(Some(values)) = row.try_get_array_text(column) {
            return Some(T::from_postgres_array(array_values(values)));
        }
        None
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
            .map(|value| value.map_or(PostgresValue::Null, Into::into))
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

    /// Resolve a `ColumnRef` to either an index-based or name-based `try_get` call
    /// on the underlying driver `Row`, returning `Err(())` when neither key resolves.
    macro_rules! try_get_typed {
        ($self:ident, $column:ident, $ty:ty) => {
            match ($column.to_index(), $column.to_name()) {
                (Some(idx), _) => $self.try_get::<_, Option<$ty>>(idx).map_err(|_| ()),
                (None, Some(name)) => $self.try_get::<_, Option<$ty>>(name).map_err(|_| ()),
                (None, None) => Err(()),
            }
        };
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
            let idx = match (column.to_index(), column.to_name()) {
                (Some(idx), _) => idx,
                (None, Some(name)) => self.columns().iter().position(|c| c.name() == name)?,
                (None, None) => return None,
            };

            self.columns().get(idx).map(|c| c.type_().oid())
        }

        fn try_get_bool(&self, column: &impl ColumnRef) -> Result<Option<bool>, ()> {
            try_get_typed!(self, column, bool)
        }

        fn try_get_i16(&self, column: &impl ColumnRef) -> Result<Option<i16>, ()> {
            try_get_typed!(self, column, i16)
        }

        fn try_get_i32(&self, column: &impl ColumnRef) -> Result<Option<i32>, ()> {
            try_get_typed!(self, column, i32)
        }

        fn try_get_i64(&self, column: &impl ColumnRef) -> Result<Option<i64>, ()> {
            try_get_typed!(self, column, i64)
        }

        fn try_get_f32(&self, column: &impl ColumnRef) -> Result<Option<f32>, ()> {
            try_get_typed!(self, column, f32)
        }

        fn try_get_f64(&self, column: &impl ColumnRef) -> Result<Option<f64>, ()> {
            try_get_typed!(self, column, f64)
        }

        fn try_get_string(&self, column: &impl ColumnRef) -> Result<Option<String>, ()> {
            try_get_typed!(self, column, String)
        }

        fn try_get_bytes(&self, column: &impl ColumnRef) -> Result<Option<Vec<u8>>, ()> {
            try_get_typed!(self, column, Vec<u8>)
        }

        #[cfg(feature = "uuid")]
        fn try_get_uuid(&self, column: &impl ColumnRef) -> Result<Option<uuid::Uuid>, ()> {
            try_get_typed!(self, column, uuid::Uuid)
        }

        #[cfg(feature = "serde")]
        fn try_get_json(&self, column: &impl ColumnRef) -> Result<Option<serde_json::Value>, ()> {
            try_get_typed!(self, column, serde_json::Value)
        }

        #[cfg(feature = "chrono")]
        fn try_get_date(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveDate>, ()> {
            try_get_typed!(self, column, chrono::NaiveDate)
        }

        #[cfg(feature = "chrono")]
        fn try_get_time(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveTime>, ()> {
            try_get_typed!(self, column, chrono::NaiveTime)
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::NaiveDateTime>, ()> {
            try_get_typed!(self, column, chrono::NaiveDateTime)
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ()> {
            try_get_typed!(self, column, chrono::DateTime<chrono::FixedOffset>)
        }

        #[cfg(feature = "cidr")]
        fn try_get_inet(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpInet>, ()> {
            try_get_typed!(self, column, cidr::IpInet)
        }

        #[cfg(feature = "cidr")]
        fn try_get_cidr(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpCidr>, ()> {
            try_get_typed!(self, column, cidr::IpCidr)
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
            try_get_typed!(self, column, geo_types::Point<f64>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::LineString<f64>>, ()> {
            try_get_typed!(self, column, geo_types::LineString<f64>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Rect<f64>>, ()> {
            try_get_typed!(self, column, geo_types::Rect<f64>)
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_bitvec(&self, column: &impl ColumnRef) -> Result<Option<bit_vec::BitVec>, ()> {
            try_get_typed!(self, column, bit_vec::BitVec)
        }

        fn try_get_array_bool(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bool>>>, ()> {
            try_get_typed!(self, column, Vec<Option<bool>>)
        }

        fn try_get_array_i16(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i16>>>, ()> {
            try_get_typed!(self, column, Vec<Option<i16>>)
        }

        fn try_get_array_i32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i32>>>, ()> {
            try_get_typed!(self, column, Vec<Option<i32>>)
        }

        fn try_get_array_i64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i64>>>, ()> {
            try_get_typed!(self, column, Vec<Option<i64>>)
        }

        fn try_get_array_f32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f32>>>, ()> {
            try_get_typed!(self, column, Vec<Option<f32>>)
        }

        fn try_get_array_f64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f64>>>, ()> {
            try_get_typed!(self, column, Vec<Option<f64>>)
        }

        fn try_get_array_text(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<String>>>, ()> {
            try_get_typed!(self, column, Vec<Option<String>>)
        }

        fn try_get_array_bytes(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<Vec<u8>>>>, ()> {
            try_get_typed!(self, column, Vec<Option<Vec<u8>>>)
        }

        #[cfg(feature = "uuid")]
        fn try_get_array_uuid(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<uuid::Uuid>>>, ()> {
            try_get_typed!(self, column, Vec<Option<uuid::Uuid>>)
        }

        #[cfg(feature = "serde")]
        fn try_get_array_json(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<serde_json::Value>>>, ()> {
            try_get_typed!(self, column, Vec<Option<serde_json::Value>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_date(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDate>>>, ()> {
            try_get_typed!(self, column, Vec<Option<chrono::NaiveDate>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_time(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveTime>>>, ()> {
            try_get_typed!(self, column, Vec<Option<chrono::NaiveTime>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDateTime>>>, ()> {
            try_get_typed!(self, column, Vec<Option<chrono::NaiveDateTime>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>, ()> {
            try_get_typed!(
                self,
                column,
                Vec<Option<chrono::DateTime<chrono::FixedOffset>>>
            )
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_inet(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpInet>>>, ()> {
            try_get_typed!(self, column, Vec<Option<cidr::IpInet>>)
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_cidr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpCidr>>>, ()> {
            try_get_typed!(self, column, Vec<Option<cidr::IpCidr>>)
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
            try_get_typed!(self, column, Vec<Option<geo_types::Point<f64>>>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::LineString<f64>>>>, ()> {
            try_get_typed!(self, column, Vec<Option<geo_types::LineString<f64>>>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Rect<f64>>>>, ()> {
            try_get_typed!(self, column, Vec<Option<geo_types::Rect<f64>>>)
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_array_bitvec(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bit_vec::BitVec>>>, ()> {
            try_get_typed!(self, column, Vec<Option<bit_vec::BitVec>>)
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
            let idx = match (column.to_index(), column.to_name()) {
                (Some(idx), _) => idx,
                (None, Some(name)) => self.columns().iter().position(|c| c.name() == name)?,
                (None, None) => return None,
            };

            self.columns().get(idx).map(|c| c.type_().oid())
        }

        fn try_get_bool(&self, column: &impl ColumnRef) -> Result<Option<bool>, ()> {
            try_get_typed!(self, column, bool)
        }

        fn try_get_i16(&self, column: &impl ColumnRef) -> Result<Option<i16>, ()> {
            try_get_typed!(self, column, i16)
        }

        fn try_get_i32(&self, column: &impl ColumnRef) -> Result<Option<i32>, ()> {
            try_get_typed!(self, column, i32)
        }

        fn try_get_i64(&self, column: &impl ColumnRef) -> Result<Option<i64>, ()> {
            try_get_typed!(self, column, i64)
        }

        fn try_get_f32(&self, column: &impl ColumnRef) -> Result<Option<f32>, ()> {
            try_get_typed!(self, column, f32)
        }

        fn try_get_f64(&self, column: &impl ColumnRef) -> Result<Option<f64>, ()> {
            try_get_typed!(self, column, f64)
        }

        fn try_get_string(&self, column: &impl ColumnRef) -> Result<Option<String>, ()> {
            try_get_typed!(self, column, String)
        }

        fn try_get_bytes(&self, column: &impl ColumnRef) -> Result<Option<Vec<u8>>, ()> {
            try_get_typed!(self, column, Vec<u8>)
        }

        #[cfg(feature = "uuid")]
        fn try_get_uuid(&self, column: &impl ColumnRef) -> Result<Option<uuid::Uuid>, ()> {
            try_get_typed!(self, column, uuid::Uuid)
        }

        #[cfg(feature = "serde")]
        fn try_get_json(&self, column: &impl ColumnRef) -> Result<Option<serde_json::Value>, ()> {
            try_get_typed!(self, column, serde_json::Value)
        }

        #[cfg(feature = "chrono")]
        fn try_get_date(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveDate>, ()> {
            try_get_typed!(self, column, chrono::NaiveDate)
        }

        #[cfg(feature = "chrono")]
        fn try_get_time(&self, column: &impl ColumnRef) -> Result<Option<chrono::NaiveTime>, ()> {
            try_get_typed!(self, column, chrono::NaiveTime)
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::NaiveDateTime>, ()> {
            try_get_typed!(self, column, chrono::NaiveDateTime)
        }

        #[cfg(feature = "chrono")]
        fn try_get_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ()> {
            try_get_typed!(self, column, chrono::DateTime<chrono::FixedOffset>)
        }

        #[cfg(feature = "cidr")]
        fn try_get_inet(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpInet>, ()> {
            try_get_typed!(self, column, cidr::IpInet)
        }

        #[cfg(feature = "cidr")]
        fn try_get_cidr(&self, column: &impl ColumnRef) -> Result<Option<cidr::IpCidr>, ()> {
            try_get_typed!(self, column, cidr::IpCidr)
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
            try_get_typed!(self, column, geo_types::Point<f64>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::LineString<f64>>, ()> {
            try_get_typed!(self, column, geo_types::LineString<f64>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<geo_types::Rect<f64>>, ()> {
            try_get_typed!(self, column, geo_types::Rect<f64>)
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_bitvec(&self, column: &impl ColumnRef) -> Result<Option<bit_vec::BitVec>, ()> {
            try_get_typed!(self, column, bit_vec::BitVec)
        }

        fn try_get_array_bool(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bool>>>, ()> {
            try_get_typed!(self, column, Vec<Option<bool>>)
        }

        fn try_get_array_i16(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i16>>>, ()> {
            try_get_typed!(self, column, Vec<Option<i16>>)
        }

        fn try_get_array_i32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i32>>>, ()> {
            try_get_typed!(self, column, Vec<Option<i32>>)
        }

        fn try_get_array_i64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<i64>>>, ()> {
            try_get_typed!(self, column, Vec<Option<i64>>)
        }

        fn try_get_array_f32(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f32>>>, ()> {
            try_get_typed!(self, column, Vec<Option<f32>>)
        }

        fn try_get_array_f64(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<f64>>>, ()> {
            try_get_typed!(self, column, Vec<Option<f64>>)
        }

        fn try_get_array_text(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<String>>>, ()> {
            try_get_typed!(self, column, Vec<Option<String>>)
        }

        fn try_get_array_bytes(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<Vec<u8>>>>, ()> {
            try_get_typed!(self, column, Vec<Option<Vec<u8>>>)
        }

        #[cfg(feature = "uuid")]
        fn try_get_array_uuid(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<uuid::Uuid>>>, ()> {
            try_get_typed!(self, column, Vec<Option<uuid::Uuid>>)
        }

        #[cfg(feature = "serde")]
        fn try_get_array_json(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<serde_json::Value>>>, ()> {
            try_get_typed!(self, column, Vec<Option<serde_json::Value>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_date(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDate>>>, ()> {
            try_get_typed!(self, column, Vec<Option<chrono::NaiveDate>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_time(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveTime>>>, ()> {
            try_get_typed!(self, column, Vec<Option<chrono::NaiveTime>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamp(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::NaiveDateTime>>>, ()> {
            try_get_typed!(self, column, Vec<Option<chrono::NaiveDateTime>>)
        }

        #[cfg(feature = "chrono")]
        fn try_get_array_timestamptz(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<chrono::DateTime<chrono::FixedOffset>>>>, ()> {
            try_get_typed!(
                self,
                column,
                Vec<Option<chrono::DateTime<chrono::FixedOffset>>>
            )
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_inet(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpInet>>>, ()> {
            try_get_typed!(self, column, Vec<Option<cidr::IpInet>>)
        }

        #[cfg(feature = "cidr")]
        fn try_get_array_cidr(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<cidr::IpCidr>>>, ()> {
            try_get_typed!(self, column, Vec<Option<cidr::IpCidr>>)
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
            try_get_typed!(self, column, Vec<Option<geo_types::Point<f64>>>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_linestring(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::LineString<f64>>>>, ()> {
            try_get_typed!(self, column, Vec<Option<geo_types::LineString<f64>>>)
        }

        #[cfg(feature = "geo-types")]
        fn try_get_array_rect(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<geo_types::Rect<f64>>>>, ()> {
            try_get_typed!(self, column, Vec<Option<geo_types::Rect<f64>>>)
        }

        #[cfg(feature = "bit-vec")]
        fn try_get_array_bitvec(
            &self,
            column: &impl ColumnRef,
        ) -> Result<Option<Vec<Option<bit_vec::BitVec>>>, ()> {
            try_get_typed!(self, column, Vec<Option<bit_vec::BitVec>>)
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
        Self::parse_str(value).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UUID string '{value}': {e}").into())
        })
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Self::from_slice(value)
            .map_err(|e| DrizzleError::ConversionError(format!("invalid UUID bytes: {e}").into()))
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
        let s = core::str::from_utf8(value).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 for enum: {e}").into())
        })?;
        T::try_from_str(s)
    }
}

// =============================================================================
// ARRAY support
// =============================================================================

impl FromPostgresValue for Vec<PostgresValue<'_>> {
    impl_from_postgres_value_errors!("ARRAY");

    fn from_postgres_array(value: Vec<PostgresValue<'_>>) -> Result<Self, DrizzleError> {
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

    fn from_postgres_array(value: Vec<PostgresValue<'_>>) -> Result<Self, DrizzleError> {
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
// Time crate support (when feature enabled)
// =============================================================================

#[cfg(feature = "time")]
impl FromPostgresValue for time::Date {
    impl_from_postgres_value_errors!("time::Date");

    fn from_postgres_time_date(value: time::Date) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "time")]
impl FromPostgresValue for time::Time {
    impl_from_postgres_value_errors!("time::Time");

    fn from_postgres_time_time(value: time::Time) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "time")]
impl FromPostgresValue for time::PrimitiveDateTime {
    impl_from_postgres_value_errors!("time::PrimitiveDateTime");

    fn from_postgres_time_timestamp(value: time::PrimitiveDateTime) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "time")]
impl FromPostgresValue for time::OffsetDateTime {
    impl_from_postgres_value_errors!("time::OffsetDateTime");

    fn from_postgres_time_timestamptz(value: time::OffsetDateTime) -> Result<Self, DrizzleError> {
        Ok(value)
    }
}

#[cfg(feature = "time")]
impl FromPostgresValue for time::Duration {
    impl_from_postgres_value_errors!("time::Duration");

    fn from_postgres_time_interval(value: time::Duration) -> Result<Self, DrizzleError> {
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
        Self::from(&s).map_err(|_| {
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
        Self::from(&s).map_err(|_| {
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
        Self::from(&s).map_err(|_| {
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
        Self::from(&s).map_err(|_| {
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
        Self::from(&s).map_err(|_| {
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
        Self::from(&s).map_err(|_| {
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
        Self::from(value).map_err(|_| {
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
            .map_err(|e| DrizzleError::ConversionError(format!("invalid UTF-8: {e}").into()))?;
        Self::from(&s).map_err(|_| {
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
        Self::try_from(value).map_err(|_| {
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

#[cfg(feature = "compact-str")]
impl FromPostgresValue for compact_str::CompactString {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Ok(Self::new(value.to_string()))
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Ok(Self::new(value.to_string()))
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Ok(Self::new(value.to_string()))
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Ok(Self::new(value.to_string()))
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Ok(Self::new(value.to_string()))
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Ok(Self::new(value.to_string()))
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Ok(Self::new(value))
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        let s = String::from_utf8(value.to_vec()).map_err(|e| {
            DrizzleError::ConversionError(format!("invalid UTF-8 in BYTEA: {e}").into())
        })?;
        Ok(Self::new(s))
    }
}

#[cfg(feature = "bytes")]
impl FromPostgresValue for bytes::Bytes {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(Self::from)
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(Self::from)
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(Self::from)
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(Self::from)
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(Self::from)
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(Self::from)
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(Self::from)
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Ok(Self::copy_from_slice(value))
    }
}

#[cfg(feature = "bytes")]
impl FromPostgresValue for bytes::BytesMut {
    fn from_postgres_bool(value: bool) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_bool(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_i16(value: i16) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i16(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_i32(value: i32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i32(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_i64(value: i64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_i64(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_f32(value: f32) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f32(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_f64(value: f64) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_f64(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_text(value: &str) -> Result<Self, DrizzleError> {
        Vec::<u8>::from_postgres_text(value).map(|v| Self::from(v.as_slice()))
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        Ok(Self::from(value))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> FromPostgresValue for smallvec::SmallVec<[u8; N]> {
    fn from_postgres_bool(_value: bool) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert bool to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_i16(_value: i16) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i16 to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_i32(_value: i32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i32 to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_i64(_value: i64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert i64 to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_f32(_value: f32) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f32 to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_f64(_value: f64) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert f64 to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_text(_value: &str) -> Result<Self, DrizzleError> {
        Err(DrizzleError::ConversionError(
            "cannot convert TEXT to SmallVec<u8>, use BYTEA".into(),
        ))
    }

    fn from_postgres_bytes(value: &[u8]) -> Result<Self, DrizzleError> {
        let mut out = Self::new();
        out.extend_from_slice(value);
        Ok(out)
    }
}
