//! Owned MySQL parameter values.

use super::MySQLValue;
use crate::prelude::*;
use drizzle_core::{Dialect, MySQLDialect, SQL, SQLParam};

/// An owned MySQL protocol value suitable for prepared-query storage.
///
/// Convert a [`MySQLValue`] explicitly with [`MySQLValue::into_owned`] when
/// borrowed bytes must be detached. The generic `SQL::into_owned()` operation
/// owns its chunk list but deliberately does not change one parameter value
/// type into another.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OwnedMySQLValue {
    #[default]
    Null,
    Bytes(Vec<u8>),
    Int(i64),
    UInt(u64),
    Float(f32),
    Double(f64),
    Date {
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        microseconds: u32,
    },
    Time {
        negative: bool,
        days: u32,
        hours: u8,
        minutes: u8,
        seconds: u8,
        microseconds: u32,
    },
}

impl SQLParam for OwnedMySQLValue {
    const DIALECT: Dialect = Dialect::MySQL;
    type DialectMarker = MySQLDialect;

    fn pagination_param(value: usize) -> Option<Self> {
        u64::try_from(value).ok().map(Self::UInt)
    }
}

impl From<OwnedMySQLValue> for SQL<'_, OwnedMySQLValue> {
    fn from(value: OwnedMySQLValue) -> Self {
        SQL::param(value)
    }
}

impl From<OwnedMySQLValue> for Cow<'_, OwnedMySQLValue> {
    fn from(value: OwnedMySQLValue) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a OwnedMySQLValue> for Cow<'a, OwnedMySQLValue> {
    fn from(value: &'a OwnedMySQLValue) -> Self {
        Cow::Borrowed(value)
    }
}

impl<'a> From<MySQLValue<'a>> for OwnedMySQLValue {
    fn from(value: MySQLValue<'a>) -> Self {
        match value {
            MySQLValue::Null => Self::Null,
            MySQLValue::Bytes(value) => Self::Bytes(value.into_owned()),
            MySQLValue::Int(value) => Self::Int(value),
            MySQLValue::UInt(value) => Self::UInt(value),
            MySQLValue::Float(value) => Self::Float(value),
            MySQLValue::Double(value) => Self::Double(value),
            MySQLValue::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => Self::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            },
            MySQLValue::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => Self::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            },
        }
    }
}

impl From<OwnedMySQLValue> for MySQLValue<'_> {
    fn from(value: OwnedMySQLValue) -> Self {
        match value {
            OwnedMySQLValue::Null => Self::Null,
            OwnedMySQLValue::Bytes(value) => Self::Bytes(Cow::Owned(value)),
            OwnedMySQLValue::Int(value) => Self::Int(value),
            OwnedMySQLValue::UInt(value) => Self::UInt(value),
            OwnedMySQLValue::Float(value) => Self::Float(value),
            OwnedMySQLValue::Double(value) => Self::Double(value),
            OwnedMySQLValue::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => Self::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            },
            OwnedMySQLValue::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => Self::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            },
        }
    }
}

impl<'a> From<&'a OwnedMySQLValue> for MySQLValue<'a> {
    fn from(value: &'a OwnedMySQLValue) -> Self {
        match value {
            OwnedMySQLValue::Null => Self::Null,
            OwnedMySQLValue::Bytes(value) => Self::Bytes(Cow::Borrowed(value)),
            OwnedMySQLValue::Int(value) => Self::Int(*value),
            OwnedMySQLValue::UInt(value) => Self::UInt(*value),
            OwnedMySQLValue::Float(value) => Self::Float(*value),
            OwnedMySQLValue::Double(value) => Self::Double(*value),
            OwnedMySQLValue::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => Self::Date {
                year: *year,
                month: *month,
                day: *day,
                hour: *hour,
                minute: *minute,
                second: *second,
                microseconds: *microseconds,
            },
            OwnedMySQLValue::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => Self::Time {
                negative: *negative,
                days: *days,
                hours: *hours,
                minutes: *minutes,
                seconds: *seconds,
                microseconds: *microseconds,
            },
        }
    }
}

macro_rules! via_mysql_value {
    (both: [$($both:ty),+ $(,)?], owned: [$($owned:ty),* $(,)?]) => {
        $(
            impl From<$both> for OwnedMySQLValue {
                fn from(value: $both) -> Self {
                    MySQLValue::from(value).into_owned()
                }
            }

            impl From<&$both> for OwnedMySQLValue {
                fn from(value: &$both) -> Self {
                    MySQLValue::from(value).into_owned()
                }
            }
        )+

        $(
            impl From<$owned> for OwnedMySQLValue {
                fn from(value: $owned) -> Self {
                    MySQLValue::from(value).into_owned()
                }
            }
        )*
    };
}

via_mysql_value!(
    both: [
        i8,
        i16,
        i32,
        i64,
        isize,
        u8,
        u16,
        u32,
        u64,
        usize,
        bool,
        f32,
        f64,
        String,
        Box<String>,
        Rc<String>,
        Arc<String>,
        Box<str>,
        Rc<str>,
        Arc<str>,
        Box<Vec<u8>>,
        Rc<Vec<u8>>,
        Arc<Vec<u8>>,
    ],
    owned: [Vec<u8>]
);

impl From<&str> for OwnedMySQLValue {
    fn from(value: &str) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl From<&[u8]> for OwnedMySQLValue {
    fn from(value: &[u8]) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl<const N: usize> From<[u8; N]> for OwnedMySQLValue {
    fn from(value: [u8; N]) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl<const N: usize> From<&[u8; N]> for OwnedMySQLValue {
    fn from(value: &[u8; N]) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl<const N: usize> From<[char; N]> for OwnedMySQLValue {
    fn from(value: [char; N]) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl<const N: usize> From<&[char; N]> for OwnedMySQLValue {
    fn from(value: &[char; N]) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl From<Cow<'_, str>> for OwnedMySQLValue {
    fn from(value: Cow<'_, str>) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl From<Cow<'_, [u8]>> for OwnedMySQLValue {
    fn from(value: Cow<'_, [u8]>) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

impl<T> From<Option<T>> for OwnedMySQLValue
where
    Self: From<T>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Self::from)
    }
}

#[cfg(feature = "compact-str")]
via_mysql_value!(both: [compact_str::CompactString], owned: []);

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayString<N>> for OwnedMySQLValue {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for OwnedMySQLValue {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<smallvec::SmallVec<[u8; N]>> for OwnedMySQLValue {
    fn from(value: smallvec::SmallVec<[u8; N]>) -> Self {
        MySQLValue::from(value).into_owned()
    }
}

#[cfg(feature = "bytes")]
via_mysql_value!(both: [bytes::Bytes, bytes::BytesMut], owned: []);

#[cfg(feature = "uuid")]
via_mysql_value!(both: [uuid::Uuid], owned: []);

#[cfg(feature = "serde")]
via_mysql_value!(both: [serde_json::Value], owned: []);

#[cfg(feature = "rust-decimal")]
via_mysql_value!(both: [rust_decimal::Decimal], owned: []);

#[cfg(feature = "chrono")]
via_mysql_value!(
    both: [chrono::NaiveDate, chrono::NaiveTime, chrono::NaiveDateTime],
    owned: [chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::FixedOffset>]
);

#[cfg(feature = "time")]
via_mysql_value!(
    both: [time::Date, time::Time, time::PrimitiveDateTime],
    owned: [time::OffsetDateTime]
);
