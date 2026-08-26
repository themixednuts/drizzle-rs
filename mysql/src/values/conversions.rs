//! Primitive conversions into MySQL parameter values.

use super::MySQLValue;
use crate::prelude::*;

macro_rules! signed_values {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for MySQLValue<'_> {
                fn from(value: $ty) -> Self {
                    Self::Int(value as i64)
                }
            }

            impl From<&$ty> for MySQLValue<'_> {
                fn from(value: &$ty) -> Self {
                    Self::from(*value)
                }
            }
        )+
    };
}

macro_rules! unsigned_values {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for MySQLValue<'_> {
                fn from(value: $ty) -> Self {
                    Self::UInt(value as u64)
                }
            }

            impl From<&$ty> for MySQLValue<'_> {
                fn from(value: &$ty) -> Self {
                    Self::from(*value)
                }
            }
        )+
    };
}

signed_values!(i8, i16, i32, i64, isize);
unsigned_values!(u8, u16, u32, u64, usize);

impl From<bool> for MySQLValue<'_> {
    fn from(value: bool) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<&bool> for MySQLValue<'_> {
    fn from(value: &bool) -> Self {
        Self::from(*value)
    }
}

impl From<char> for MySQLValue<'_> {
    fn from(value: char) -> Self {
        Self::from(String::from(value))
    }
}

impl From<f32> for MySQLValue<'_> {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<&f32> for MySQLValue<'_> {
    fn from(value: &f32) -> Self {
        Self::Float(*value)
    }
}

impl From<f64> for MySQLValue<'_> {
    fn from(value: f64) -> Self {
        Self::Double(value)
    }
}

impl From<&f64> for MySQLValue<'_> {
    fn from(value: &f64) -> Self {
        Self::Double(*value)
    }
}

impl<'a> From<&'a str> for MySQLValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::Bytes(Cow::Borrowed(value.as_bytes()))
    }
}

impl<'a> From<Cow<'a, str>> for MySQLValue<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        match value {
            Cow::Borrowed(value) => Self::Bytes(Cow::Borrowed(value.as_bytes())),
            Cow::Owned(value) => Self::Bytes(Cow::Owned(value.into_bytes())),
        }
    }
}

impl From<String> for MySQLValue<'_> {
    fn from(value: String) -> Self {
        Self::Bytes(Cow::Owned(value.into_bytes()))
    }
}

impl<'a> From<&'a String> for MySQLValue<'a> {
    fn from(value: &'a String) -> Self {
        Self::from(value.as_str())
    }
}

macro_rules! owned_string_wrapper {
    ($wrapper:ident) => {
        impl From<$wrapper<String>> for MySQLValue<'_> {
            fn from(value: $wrapper<String>) -> Self {
                Self::Bytes(Cow::Owned(value.as_bytes().to_vec()))
            }
        }

        impl<'a> From<&'a $wrapper<String>> for MySQLValue<'a> {
            fn from(value: &'a $wrapper<String>) -> Self {
                Self::Bytes(Cow::Borrowed(value.as_bytes()))
            }
        }

        impl From<$wrapper<str>> for MySQLValue<'_> {
            fn from(value: $wrapper<str>) -> Self {
                Self::Bytes(Cow::Owned(value.as_bytes().to_vec()))
            }
        }

        impl<'a> From<&'a $wrapper<str>> for MySQLValue<'a> {
            fn from(value: &'a $wrapper<str>) -> Self {
                Self::Bytes(Cow::Borrowed(value.as_bytes()))
            }
        }
    };
}

owned_string_wrapper!(Box);
owned_string_wrapper!(Rc);
owned_string_wrapper!(Arc);

#[cfg(feature = "compact-str")]
impl From<compact_str::CompactString> for MySQLValue<'_> {
    fn from(value: compact_str::CompactString) -> Self {
        Self::Bytes(Cow::Owned(value.as_bytes().to_vec()))
    }
}

#[cfg(feature = "compact-str")]
impl<'a> From<&'a compact_str::CompactString> for MySQLValue<'a> {
    fn from(value: &'a compact_str::CompactString) -> Self {
        Self::Bytes(Cow::Borrowed(value.as_bytes()))
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayString<N>> for MySQLValue<'_> {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        Self::Bytes(Cow::Owned(value.as_bytes().to_vec()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&'a arrayvec::ArrayString<N>> for MySQLValue<'a> {
    fn from(value: &'a arrayvec::ArrayString<N>) -> Self {
        Self::Bytes(Cow::Borrowed(value.as_bytes()))
    }
}

impl<'a> From<&'a [u8]> for MySQLValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, [u8]>> for MySQLValue<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        Self::Bytes(value)
    }
}

impl From<Vec<u8>> for MySQLValue<'_> {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(Cow::Owned(value))
    }
}

impl<const N: usize> From<[u8; N]> for MySQLValue<'_> {
    fn from(value: [u8; N]) -> Self {
        Self::Bytes(Cow::Owned(value.to_vec()))
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for MySQLValue<'a> {
    fn from(value: &'a [u8; N]) -> Self {
        Self::Bytes(Cow::Borrowed(value))
    }
}

impl<const N: usize> From<[char; N]> for MySQLValue<'_> {
    fn from(value: [char; N]) -> Self {
        Self::from(value.into_iter().collect::<String>())
    }
}

impl<const N: usize> From<&[char; N]> for MySQLValue<'_> {
    fn from(value: &[char; N]) -> Self {
        Self::from(value.iter().collect::<String>())
    }
}

macro_rules! owned_bytes_wrapper {
    ($wrapper:ident) => {
        impl From<$wrapper<Vec<u8>>> for MySQLValue<'_> {
            fn from(value: $wrapper<Vec<u8>>) -> Self {
                Self::Bytes(Cow::Owned(value.as_slice().to_vec()))
            }
        }

        impl<'a> From<&'a $wrapper<Vec<u8>>> for MySQLValue<'a> {
            fn from(value: &'a $wrapper<Vec<u8>>) -> Self {
                Self::Bytes(Cow::Borrowed(value.as_slice()))
            }
        }
    };
}

owned_bytes_wrapper!(Box);
owned_bytes_wrapper!(Rc);
owned_bytes_wrapper!(Arc);

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for MySQLValue<'_> {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        Self::Bytes(Cow::Owned(value.into_iter().collect()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&'a arrayvec::ArrayVec<u8, N>> for MySQLValue<'a> {
    fn from(value: &'a arrayvec::ArrayVec<u8, N>) -> Self {
        Self::Bytes(Cow::Borrowed(value.as_slice()))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<smallvec::SmallVec<[u8; N]>> for MySQLValue<'_> {
    fn from(value: smallvec::SmallVec<[u8; N]>) -> Self {
        Self::Bytes(Cow::Owned(value.into_vec()))
    }
}

#[cfg(feature = "smallvec")]
impl<'a, const N: usize> From<&'a smallvec::SmallVec<[u8; N]>> for MySQLValue<'a> {
    fn from(value: &'a smallvec::SmallVec<[u8; N]>) -> Self {
        Self::Bytes(Cow::Borrowed(value.as_slice()))
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::Bytes> for MySQLValue<'_> {
    fn from(value: bytes::Bytes) -> Self {
        Self::Bytes(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::BytesMut> for MySQLValue<'_> {
    fn from(value: bytes::BytesMut) -> Self {
        Self::Bytes(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<&bytes::Bytes> for MySQLValue<'_> {
    fn from(value: &bytes::Bytes) -> Self {
        Self::Bytes(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<&bytes::BytesMut> for MySQLValue<'_> {
    fn from(value: &bytes::BytesMut) -> Self {
        Self::Bytes(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for MySQLValue<'_> {
    fn from(value: uuid::Uuid) -> Self {
        Self::Bytes(Cow::Owned(value.into_bytes().to_vec()))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a uuid::Uuid> for MySQLValue<'a> {
    fn from(value: &'a uuid::Uuid) -> Self {
        Self::Bytes(Cow::Borrowed(value.as_bytes()))
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Value> for MySQLValue<'_> {
    fn from(value: serde_json::Value) -> Self {
        Self::Bytes(Cow::Owned(value.to_string().into_bytes()))
    }
}

#[cfg(feature = "serde")]
impl From<&serde_json::Value> for MySQLValue<'_> {
    fn from(value: &serde_json::Value) -> Self {
        Self::Bytes(Cow::Owned(value.to_string().into_bytes()))
    }
}

#[cfg(feature = "rust-decimal")]
impl From<rust_decimal::Decimal> for MySQLValue<'_> {
    fn from(value: rust_decimal::Decimal) -> Self {
        Self::Bytes(Cow::Owned(value.to_string().into_bytes()))
    }
}

#[cfg(feature = "rust-decimal")]
impl From<&rust_decimal::Decimal> for MySQLValue<'_> {
    fn from(value: &rust_decimal::Decimal) -> Self {
        Self::Bytes(Cow::Owned(value.to_string().into_bytes()))
    }
}

#[cfg(any(feature = "chrono", feature = "time"))]
struct DateParts {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    microseconds: u32,
}

#[cfg(any(feature = "chrono", feature = "time"))]
fn date_value(parts: DateParts, fallback: String) -> MySQLValue<'static> {
    match u16::try_from(parts.year) {
        Ok(year) if (1000..=9999).contains(&year) => MySQLValue::Date {
            year,
            month: parts.month,
            day: parts.day,
            hour: parts.hour,
            minute: parts.minute,
            second: parts.second,
            microseconds: parts.microseconds,
        },
        _ => MySQLValue::Bytes(Cow::Owned(fallback.into_bytes())),
    }
}

#[cfg(feature = "chrono")]
fn chrono_date(value: chrono::NaiveDate) -> MySQLValue<'static> {
    use chrono::Datelike;

    date_value(
        DateParts {
            year: value.year(),
            month: value.month() as u8,
            day: value.day() as u8,
            hour: 0,
            minute: 0,
            second: 0,
            microseconds: 0,
        },
        value.to_string(),
    )
}

#[cfg(feature = "chrono")]
fn chrono_datetime(value: chrono::NaiveDateTime) -> MySQLValue<'static> {
    use chrono::{Datelike, Timelike};

    date_value(
        DateParts {
            year: value.year(),
            month: value.month() as u8,
            day: value.day() as u8,
            hour: value.hour() as u8,
            minute: value.minute() as u8,
            second: value.second() as u8,
            microseconds: value.nanosecond() / 1_000,
        },
        value.to_string(),
    )
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDate> for MySQLValue<'_> {
    fn from(value: chrono::NaiveDate) -> Self {
        chrono_date(value)
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::NaiveDate> for MySQLValue<'_> {
    fn from(value: &chrono::NaiveDate) -> Self {
        chrono_date(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveTime> for MySQLValue<'_> {
    fn from(value: chrono::NaiveTime) -> Self {
        use chrono::Timelike;

        Self::Time {
            negative: false,
            days: 0,
            hours: value.hour() as u8,
            minutes: value.minute() as u8,
            seconds: value.second() as u8,
            microseconds: value.nanosecond() / 1_000,
        }
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::NaiveTime> for MySQLValue<'_> {
    fn from(value: &chrono::NaiveTime) -> Self {
        Self::from(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDateTime> for MySQLValue<'_> {
    fn from(value: chrono::NaiveDateTime) -> Self {
        chrono_datetime(value)
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::NaiveDateTime> for MySQLValue<'_> {
    fn from(value: &chrono::NaiveDateTime) -> Self {
        chrono_datetime(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::Utc>> for MySQLValue<'_> {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        chrono_datetime(value.naive_utc())
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::FixedOffset>> for MySQLValue<'_> {
    fn from(value: chrono::DateTime<chrono::FixedOffset>) -> Self {
        chrono_datetime(value.with_timezone(&chrono::Utc).naive_utc())
    }
}

#[cfg(feature = "time")]
fn time_date(value: time::Date) -> MySQLValue<'static> {
    date_value(
        DateParts {
            year: value.year(),
            month: u8::from(value.month()),
            day: value.day(),
            hour: 0,
            minute: 0,
            second: 0,
            microseconds: 0,
        },
        value.to_string(),
    )
}

#[cfg(feature = "time")]
fn time_datetime(value: time::PrimitiveDateTime) -> MySQLValue<'static> {
    date_value(
        DateParts {
            year: value.year(),
            month: u8::from(value.month()),
            day: value.day(),
            hour: value.hour(),
            minute: value.minute(),
            second: value.second(),
            microseconds: value.microsecond(),
        },
        value.to_string(),
    )
}

#[cfg(feature = "time")]
impl From<time::Date> for MySQLValue<'_> {
    fn from(value: time::Date) -> Self {
        time_date(value)
    }
}

#[cfg(feature = "time")]
impl From<&time::Date> for MySQLValue<'_> {
    fn from(value: &time::Date) -> Self {
        time_date(*value)
    }
}

#[cfg(feature = "time")]
impl From<time::Time> for MySQLValue<'_> {
    fn from(value: time::Time) -> Self {
        Self::Time {
            negative: false,
            days: 0,
            hours: value.hour(),
            minutes: value.minute(),
            seconds: value.second(),
            microseconds: value.microsecond(),
        }
    }
}

#[cfg(feature = "time")]
impl From<&time::Time> for MySQLValue<'_> {
    fn from(value: &time::Time) -> Self {
        Self::from(*value)
    }
}

#[cfg(feature = "time")]
impl From<time::PrimitiveDateTime> for MySQLValue<'_> {
    fn from(value: time::PrimitiveDateTime) -> Self {
        time_datetime(value)
    }
}

#[cfg(feature = "time")]
impl From<&time::PrimitiveDateTime> for MySQLValue<'_> {
    fn from(value: &time::PrimitiveDateTime) -> Self {
        time_datetime(*value)
    }
}

#[cfg(feature = "time")]
impl From<time::OffsetDateTime> for MySQLValue<'_> {
    fn from(value: time::OffsetDateTime) -> Self {
        let utc = value.to_offset(time::UtcOffset::UTC);
        time_datetime(time::PrimitiveDateTime::new(utc.date(), utc.time()))
    }
}

impl<'a, T> From<Option<T>> for MySQLValue<'a>
where
    MySQLValue<'a>: From<T>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, MySQLValue::from)
    }
}

impl<'a> From<MySQLValue<'a>> for Cow<'a, MySQLValue<'a>> {
    fn from(value: MySQLValue<'a>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a MySQLValue<'a>> for Cow<'a, MySQLValue<'a>> {
    fn from(value: &'a MySQLValue<'a>) -> Self {
        Cow::Borrowed(value)
    }
}
