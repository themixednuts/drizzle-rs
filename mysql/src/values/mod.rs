//! Client-neutral MySQL parameter values.

mod conversions;
mod insert;
#[cfg(feature = "serde")]
mod json;
mod owned;
mod update;

pub use insert::{MySQLInsertValue, ValueWrapper};
pub use owned::OwnedMySQLValue;
pub use update::MySQLUpdateValue;

use crate::prelude::*;
use drizzle_core::{Dialect, MySQLDialect, SQL, SQLParam, ToSQL};

/// A MySQL protocol value that may borrow byte data.
///
/// The variants match the value categories shared by the blocking and async
/// Rust clients. The dialect crate therefore does not depend on either wire
/// driver.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MySQLValue<'a> {
    /// SQL `NULL`.
    #[default]
    Null,
    /// Text, binary, decimal, JSON, enum, and set payloads.
    Bytes(Cow<'a, [u8]>),
    /// Signed integer payload.
    Int(i64),
    /// Unsigned integer payload.
    UInt(u64),
    /// Single-precision floating-point payload.
    Float(f32),
    /// Double-precision floating-point payload.
    Double(f64),
    /// Date, datetime, or timestamp payload.
    Date {
        /// Four-digit year.
        year: u16,
        /// Month in the range 1 through 12.
        month: u8,
        /// Day of the month.
        day: u8,
        /// Hour in the range 0 through 23.
        hour: u8,
        /// Minute in the range 0 through 59.
        minute: u8,
        /// Second in the range 0 through 59.
        second: u8,
        /// Fractional seconds in microseconds.
        microseconds: u32,
    },
    /// Time or duration payload.
    Time {
        /// Whether the duration is negative.
        negative: bool,
        /// Whole days in the duration.
        days: u32,
        /// Remaining hours after whole days.
        hours: u8,
        /// Remaining minutes after whole hours.
        minutes: u8,
        /// Remaining seconds after whole minutes.
        seconds: u8,
        /// Fractional seconds in microseconds.
        microseconds: u32,
    },
}

impl MySQLValue<'_> {
    /// Returns whether this value represents SQL `NULL`.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the byte payload used for textual and binary MySQL values.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns a borrowed view of this value.
    #[must_use]
    pub fn as_ref(&self) -> MySQLValue<'_> {
        match self {
            Self::Null => MySQLValue::Null,
            Self::Bytes(value) => MySQLValue::Bytes(Cow::Borrowed(value.as_ref())),
            Self::Int(value) => MySQLValue::Int(*value),
            Self::UInt(value) => MySQLValue::UInt(*value),
            Self::Float(value) => MySQLValue::Float(*value),
            Self::Double(value) => MySQLValue::Double(*value),
            Self::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => MySQLValue::Date {
                year: *year,
                month: *month,
                day: *day,
                hour: *hour,
                minute: *minute,
                second: *second,
                microseconds: *microseconds,
            },
            Self::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => MySQLValue::Time {
                negative: *negative,
                days: *days,
                hours: *hours,
                minutes: *minutes,
                seconds: *seconds,
                microseconds: *microseconds,
            },
        }
    }

    /// Converts this value into an owned representation.
    #[must_use]
    pub fn into_owned(self) -> OwnedMySQLValue {
        self.into()
    }
}

impl SQLParam for MySQLValue<'_> {
    const DIALECT: Dialect = Dialect::MySQL;
    type DialectMarker = MySQLDialect;

    fn pagination_param(value: usize) -> Option<Self> {
        u64::try_from(value).ok().map(Self::UInt)
    }

    fn write_literal(&self, buf: &mut String) -> bool {
        use core::fmt::Write;
        match self {
            Self::Null => buf.push_str("NULL"),
            Self::Int(value) => {
                let _ = write!(buf, "{value}");
            }
            Self::UInt(value) => {
                let _ = write!(buf, "{value}");
            }
            // MySQL has no NaN or infinity.
            Self::Float(value) if !value.is_finite() => return false,
            Self::Double(value) if !value.is_finite() => return false,
            Self::Float(value) => {
                let _ = write!(buf, "{value:?}");
            }
            Self::Double(value) => {
                let _ = write!(buf, "{value:?}");
            }
            Self::Bytes(bytes) => match core::str::from_utf8(bytes) {
                // Text is quoted with the same escaping the typed `query(..)`
                // view definitions use; binary data is written in hex.
                Ok(text) => {
                    buf.push('\'');
                    for ch in text.chars() {
                        match ch {
                            '\\' => buf.push_str("\\\\"),
                            '\'' => buf.push_str("''"),
                            '\0' => buf.push_str("\\0"),
                            _ => buf.push(ch),
                        }
                    }
                    buf.push('\'');
                }
                Err(_) => {
                    buf.push_str("X'");
                    for byte in bytes.iter() {
                        let _ = write!(buf, "{byte:02X}");
                    }
                    buf.push('\'');
                }
            },
            Self::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => {
                let _ = write!(
                    buf,
                    "'{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{microseconds:06}'"
                );
            }
            Self::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => {
                let sign = if *negative { "-" } else { "" };
                let hours = u64::from(*days) * 24 + u64::from(*hours);
                let _ = write!(
                    buf,
                    "'{sign}{hours:02}:{minutes:02}:{seconds:02}.{microseconds:06}'"
                );
            }
        }
        true
    }
}

impl<'a> ToSQL<'a, Self> for MySQLValue<'a> {
    fn to_sql(&self) -> SQL<'a, Self> {
        SQL::param(self.clone())
    }
}

impl<'a> From<MySQLValue<'a>> for SQL<'a, MySQLValue<'a>> {
    fn from(value: MySQLValue<'a>) -> Self {
        SQL::param(value)
    }
}

impl<'a, T> From<T> for MySQLValue<'a>
where
    T: crate::traits::DrizzleMySQLColumn,
{
    fn from(value: T) -> Self {
        value.encode_owned().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_bytes_become_owned_without_changing_content() {
        let value = MySQLValue::from("hello");
        assert_eq!(value.as_bytes(), Some(b"hello".as_slice()));
        assert_eq!(
            value.into_owned(),
            OwnedMySQLValue::Bytes(b"hello".to_vec())
        );
    }

    #[test]
    fn signed_and_unsigned_values_remain_distinct() {
        assert_eq!(MySQLValue::from(-1_i64), MySQLValue::Int(-1));
        assert_eq!(MySQLValue::from(u64::MAX), MySQLValue::UInt(u64::MAX));
    }

    #[test]
    fn optional_values_encode_none_as_sql_null() {
        assert_eq!(MySQLValue::from(Option::<u32>::None), MySQLValue::Null);
        assert_eq!(MySQLValue::from(Some(7_u32)), MySQLValue::UInt(7));
    }

    #[test]
    fn owned_values_accept_literals_and_optional_model_fields() {
        assert_eq!(OwnedMySQLValue::from(-7_i32), OwnedMySQLValue::Int(-7));
        assert_eq!(
            OwnedMySQLValue::from("hello"),
            OwnedMySQLValue::Bytes(b"hello".to_vec())
        );
        assert_eq!(
            OwnedMySQLValue::from(Some(u64::MAX)),
            OwnedMySQLValue::UInt(u64::MAX)
        );
        assert_eq!(
            OwnedMySQLValue::from(Option::<String>::None),
            OwnedMySQLValue::Null
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn timestamp_values_are_normalized_to_utc() {
        use chrono::{FixedOffset, TimeZone};

        let offset = FixedOffset::east_opt(2 * 60 * 60).unwrap();
        let local = offset.with_ymd_and_hms(2026, 8, 25, 15, 30, 45).unwrap();

        assert_eq!(
            MySQLValue::from(local),
            MySQLValue::Date {
                year: 2026,
                month: 8,
                day: 25,
                hour: 13,
                minute: 30,
                second: 45,
                microseconds: 0,
            }
        );
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn uuid_values_use_the_compact_sixteen_byte_form() {
        let uuid = uuid::Uuid::from_u128(0x12345678_90ab_cdef_1234_567890abcdef);
        assert_eq!(MySQLValue::from(uuid).as_bytes().unwrap().len(), 16);
    }
}
