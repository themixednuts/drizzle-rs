//! From and `TryFrom` implementations for `SQLiteValue`

use super::{OwnedSQLiteValue, SQLiteValue};
use crate::prelude::*;
use drizzle_core::{error::DrizzleError, sql::SQL, traits::ToSQL};

#[cfg(feature = "uuid")]
use uuid::Uuid;

//------------------------------------------------------------------------------
// ToSQL Implementation
//------------------------------------------------------------------------------

impl<'a> ToSQL<'a, Self> for SQLiteValue<'a> {
    fn to_sql(&self) -> SQL<'a, Self> {
        SQL::param(self.clone())
    }
}

//------------------------------------------------------------------------------
// From OwnedSQLiteValue
//------------------------------------------------------------------------------

impl From<OwnedSQLiteValue> for SQLiteValue<'_> {
    fn from(value: OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(f) => SQLiteValue::Integer(f),
            OwnedSQLiteValue::Real(r) => SQLiteValue::Real(r),
            OwnedSQLiteValue::Text(v) => SQLiteValue::Text(Cow::Owned(v)),
            OwnedSQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Owned(v.into())),
            OwnedSQLiteValue::Null => SQLiteValue::Null,
        }
    }
}

impl<'a> From<&'a OwnedSQLiteValue> for SQLiteValue<'a> {
    fn from(value: &'a OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(f) => SQLiteValue::Integer(*f),
            OwnedSQLiteValue::Real(r) => SQLiteValue::Real(*r),
            OwnedSQLiteValue::Text(v) => SQLiteValue::Text(Cow::Borrowed(v)),
            OwnedSQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Borrowed(v)),
            OwnedSQLiteValue::Null => SQLiteValue::Null,
        }
    }
}

impl<'a> From<&'a Self> for SQLiteValue<'a> {
    fn from(value: &'a Self) -> Self {
        match value {
            SQLiteValue::Integer(f) => SQLiteValue::Integer(*f),
            SQLiteValue::Real(r) => SQLiteValue::Real(*r),
            SQLiteValue::Text(v) => SQLiteValue::Text(Cow::Borrowed(v)),
            SQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Borrowed(v)),
            SQLiteValue::Null => SQLiteValue::Null,
        }
    }
}

impl<'a> From<Cow<'a, Self>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, Self>) -> Self {
        match value {
            Cow::Borrowed(r) => r.into(),
            Cow::Owned(o) => o,
        }
    }
}

impl<'a> From<&'a Cow<'a, Self>> for SQLiteValue<'a> {
    fn from(value: &'a Cow<'a, Self>) -> Self {
        match value {
            Cow::Borrowed(r) => (*r).into(),
            Cow::Owned(o) => o.into(),
        }
    }
}

//------------------------------------------------------------------------------
// From<T> implementations
// Macro-based to reduce boilerplate
//------------------------------------------------------------------------------

/// Integer widths where `i64::from(T)` is infallible.
macro_rules! impl_from_lossless_int_for_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> From<$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: $ty) -> Self {
                    SQLiteValue::Integer(i64::from(value))
                }
            }

            impl<'a> From<&$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: &$ty) -> Self {
                    SQLiteValue::Integer(i64::from(*value))
                }
            }
        )*
    };
}

// Types that widen to i64 without loss.
impl_from_lossless_int_for_sqlite_value!(i8, i16, i32, u8, u16, u32, bool);

// i64 identity — no conversion needed.
impl From<i64> for SQLiteValue<'_> {
    #[inline]
    fn from(value: i64) -> Self {
        SQLiteValue::Integer(value)
    }
}

impl From<&i64> for SQLiteValue<'_> {
    #[inline]
    fn from(value: &i64) -> Self {
        SQLiteValue::Integer(*value)
    }
}

// u64 → i64 by reinterpreting bits. SQLite stores INTEGER as signed 64-bit;
// this matches the round-trip convention used by the matching `i64 → u64`
// reinterpretation on read.
impl From<u64> for SQLiteValue<'_> {
    #[inline]
    fn from(value: u64) -> Self {
        SQLiteValue::Integer(value.cast_signed())
    }
}

impl From<&u64> for SQLiteValue<'_> {
    #[inline]
    fn from(value: &u64) -> Self {
        SQLiteValue::Integer(value.cast_signed())
    }
}

/// Pointer-sized integer widths (usize/isize). All Rust-supported targets have
/// pointers ≤ 64 bits, so `i64::try_from` succeeds; the saturating fallback is
/// defensive only.
macro_rules! impl_from_pointer_int_for_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> From<$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: $ty) -> Self {
                    SQLiteValue::Integer(i64::try_from(value).unwrap_or(i64::MAX))
                }
            }

            impl<'a> From<&$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: &$ty) -> Self {
                    SQLiteValue::Integer(i64::try_from(*value).unwrap_or(i64::MAX))
                }
            }
        )*
    };
}

impl_from_pointer_int_for_sqlite_value!(isize, usize);

// f32 widens exactly into f64.
impl From<f32> for SQLiteValue<'_> {
    #[inline]
    fn from(value: f32) -> Self {
        SQLiteValue::Real(f64::from(value))
    }
}

impl From<&f32> for SQLiteValue<'_> {
    #[inline]
    fn from(value: &f32) -> Self {
        SQLiteValue::Real(f64::from(*value))
    }
}

// f64 identity.
impl From<f64> for SQLiteValue<'_> {
    #[inline]
    fn from(value: f64) -> Self {
        SQLiteValue::Real(value)
    }
}

impl From<&f64> for SQLiteValue<'_> {
    #[inline]
    fn from(value: &f64) -> Self {
        SQLiteValue::Real(*value)
    }
}

// --- String Types ---

impl<'a> From<&'a str> for SQLiteValue<'a> {
    fn from(value: &'a str) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, str>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        SQLiteValue::Text(value)
    }
}

impl From<String> for SQLiteValue<'_> {
    fn from(value: String) -> Self {
        SQLiteValue::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a String> for SQLiteValue<'a> {
    fn from(value: &'a String) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value))
    }
}

impl From<Box<String>> for SQLiteValue<'_> {
    fn from(value: Box<String>) -> Self {
        SQLiteValue::Text(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<String>> for SQLiteValue<'a> {
    fn from(value: &'a Box<String>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl From<Rc<String>> for SQLiteValue<'_> {
    fn from(value: Rc<String>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<String>> for SQLiteValue<'a> {
    fn from(value: &'a Rc<String>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl From<Arc<String>> for SQLiteValue<'_> {
    fn from(value: Arc<String>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<String>> for SQLiteValue<'a> {
    fn from(value: &'a Arc<String>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl From<Box<str>> for SQLiteValue<'_> {
    fn from(value: Box<str>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.into()))
    }
}

impl<'a> From<&'a Box<str>> for SQLiteValue<'a> {
    fn from(value: &'a Box<str>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl From<Rc<str>> for SQLiteValue<'_> {
    fn from(value: Rc<str>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Rc<str>> for SQLiteValue<'a> {
    fn from(value: &'a Rc<str>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl From<Arc<str>> for SQLiteValue<'_> {
    fn from(value: Arc<str>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Arc<str>> for SQLiteValue<'a> {
    fn from(value: &'a Arc<str>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

// --- ArrayString ---

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayString<N>> for SQLiteValue<'_> {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<&arrayvec::ArrayString<N>> for SQLiteValue<'_> {
    fn from(value: &arrayvec::ArrayString<N>) -> Self {
        SQLiteValue::Text(Cow::Owned(String::from(value.as_str())))
    }
}

impl From<compact_str::CompactString> for SQLiteValue<'_> {
    fn from(value: compact_str::CompactString) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

impl<'a> From<&'a compact_str::CompactString> for SQLiteValue<'a> {
    fn from(value: &'a compact_str::CompactString) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

// --- Binary Data ---

impl<'a> From<&'a [u8]> for SQLiteValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, [u8]>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        SQLiteValue::Blob(value)
    }
}

impl From<Vec<u8>> for SQLiteValue<'_> {
    fn from(value: Vec<u8>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value))
    }
}

impl From<Box<Vec<u8>>> for SQLiteValue<'_> {
    fn from(value: Box<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: &'a Box<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_slice()))
    }
}

impl From<Rc<Vec<u8>>> for SQLiteValue<'_> {
    fn from(value: Rc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: &'a Rc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_slice()))
    }
}

impl From<Arc<Vec<u8>>> for SQLiteValue<'_> {
    fn from(value: Arc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: &'a Arc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_slice()))
    }
}

// --- ArrayVec<u8, N> ---

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for SQLiteValue<'_> {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<&arrayvec::ArrayVec<u8, N>> for SQLiteValue<'_> {
    fn from(value: &arrayvec::ArrayVec<u8, N>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::Bytes> for SQLiteValue<'_> {
    fn from(value: bytes::Bytes) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl<'a> From<&'a bytes::Bytes> for SQLiteValue<'a> {
    fn from(value: &'a bytes::Bytes) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::BytesMut> for SQLiteValue<'_> {
    fn from(value: bytes::BytesMut) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl<'a> From<&'a bytes::BytesMut> for SQLiteValue<'a> {
    fn from(value: &'a bytes::BytesMut) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<smallvec::SmallVec<[u8; N]>> for SQLiteValue<'_> {
    fn from(value: smallvec::SmallVec<[u8; N]>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.into_vec()))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<&smallvec::SmallVec<[u8; N]>> for SQLiteValue<'_> {
    fn from(value: &smallvec::SmallVec<[u8; N]>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

// --- Chrono Date/Time Types (stored as ISO-8601 text) ---

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDate> for SQLiteValue<'_> {
    fn from(value: chrono::NaiveDate) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::NaiveDate> for SQLiteValue<'_> {
    fn from(value: &chrono::NaiveDate) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveTime> for SQLiteValue<'_> {
    fn from(value: chrono::NaiveTime) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::NaiveTime> for SQLiteValue<'_> {
    fn from(value: &chrono::NaiveTime) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDateTime> for SQLiteValue<'_> {
    fn from(value: chrono::NaiveDateTime) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::NaiveDateTime> for SQLiteValue<'_> {
    fn from(value: &chrono::NaiveDateTime) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::FixedOffset>> for SQLiteValue<'_> {
    fn from(value: chrono::DateTime<chrono::FixedOffset>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_rfc3339()))
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::DateTime<chrono::FixedOffset>> for SQLiteValue<'_> {
    fn from(value: &chrono::DateTime<chrono::FixedOffset>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_rfc3339()))
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::Utc>> for SQLiteValue<'_> {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_rfc3339()))
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::DateTime<chrono::Utc>> for SQLiteValue<'_> {
    fn from(value: &chrono::DateTime<chrono::Utc>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_rfc3339()))
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::Duration> for SQLiteValue<'_> {
    fn from(value: chrono::Duration) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "chrono")]
impl From<&chrono::Duration> for SQLiteValue<'_> {
    fn from(value: &chrono::Duration) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

// --- Time crate Date/Time Types (stored as ISO-8601 text) ---

#[cfg(feature = "time")]
impl From<time::Date> for SQLiteValue<'_> {
    fn from(value: time::Date) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Iso8601::DATE)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<&time::Date> for SQLiteValue<'_> {
    fn from(value: &time::Date) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Iso8601::DATE)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<time::Time> for SQLiteValue<'_> {
    fn from(value: time::Time) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Iso8601::TIME)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<&time::Time> for SQLiteValue<'_> {
    fn from(value: &time::Time) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Iso8601::TIME)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<time::PrimitiveDateTime> for SQLiteValue<'_> {
    fn from(value: time::PrimitiveDateTime) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Iso8601::DATE_TIME)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<&time::PrimitiveDateTime> for SQLiteValue<'_> {
    fn from(value: &time::PrimitiveDateTime) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Iso8601::DATE_TIME)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<time::OffsetDateTime> for SQLiteValue<'_> {
    fn from(value: time::OffsetDateTime) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<&time::OffsetDateTime> for SQLiteValue<'_> {
    fn from(value: &time::OffsetDateTime) -> Self {
        SQLiteValue::Text(Cow::Owned(
            value
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "time")]
impl From<time::Duration> for SQLiteValue<'_> {
    fn from(value: time::Duration) -> Self {
        SQLiteValue::Text(Cow::Owned(format!("{}s", value.whole_seconds())))
    }
}

#[cfg(feature = "time")]
impl From<&time::Duration> for SQLiteValue<'_> {
    fn from(value: &time::Duration) -> Self {
        SQLiteValue::Text(Cow::Owned(format!("{}s", value.whole_seconds())))
    }
}

// --- Decimal (stored as text for lossless round-trip) ---

#[cfg(feature = "rust-decimal")]
impl From<rust_decimal::Decimal> for SQLiteValue<'_> {
    fn from(value: rust_decimal::Decimal) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "rust-decimal")]
impl From<&rust_decimal::Decimal> for SQLiteValue<'_> {
    fn from(value: &rust_decimal::Decimal) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

// --- JSON ---

#[cfg(feature = "serde")]
impl From<serde_json::Value> for SQLiteValue<'_> {
    fn from(value: serde_json::Value) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "serde")]
impl From<&serde_json::Value> for SQLiteValue<'_> {
    fn from(value: &serde_json::Value) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl From<Uuid> for SQLiteValue<'_> {
    fn from(value: Uuid) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_bytes().to_vec()))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for SQLiteValue<'a> {
    fn from(value: &'a Uuid) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_bytes()))
    }
}

// --- Option Types ---
impl<T> From<Option<T>> for SQLiteValue<'_>
where
    T: TryInto<Self>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(SQLiteValue::Null, |v| {
            v.try_into().unwrap_or(SQLiteValue::Null)
        })
    }
}

// --- Cow integration for SQL struct ---
impl<'a> From<SQLiteValue<'a>> for Cow<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a SQLiteValue<'a>> for Cow<'a, SQLiteValue<'a>> {
    fn from(value: &'a SQLiteValue<'a>) -> Self {
        Cow::Borrowed(value)
    }
}

//------------------------------------------------------------------------------
// TryFrom<SQLiteValue> implementations
// Uses the FromSQLiteValue trait via convert() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement `TryFrom`<SQLiteValue> for types implementing `FromSQLiteValue`
macro_rules! impl_try_from_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> TryFrom<SQLiteValue<'a>> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
                    value.convert()
                }
            }
        )*
    };
}

impl_try_from_sqlite_value!(
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
    f32,
    f64,
    bool,
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
    Vec<u8>,
    compact_str::CompactString,
);

#[cfg(feature = "uuid")]
impl_try_from_sqlite_value!(Uuid);

#[cfg(feature = "chrono")]
impl_try_from_sqlite_value!(
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::FixedOffset>,
    chrono::DateTime<chrono::Utc>,
    chrono::Duration,
);

#[cfg(feature = "time")]
impl_try_from_sqlite_value!(
    time::Date,
    time::Time,
    time::PrimitiveDateTime,
    time::OffsetDateTime,
    time::Duration,
);

#[cfg(feature = "rust-decimal")]
impl_try_from_sqlite_value!(rust_decimal::Decimal);

#[cfg(feature = "serde")]
impl_try_from_sqlite_value!(serde_json::Value);

#[cfg(feature = "bytes")]
impl_try_from_sqlite_value!(bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "smallvec")]
impl<const N: usize> TryFrom<SQLiteValue<'_>> for smallvec::SmallVec<[u8; N]> {
    type Error = DrizzleError;

    fn try_from(value: SQLiteValue<'_>) -> Result<Self, Self::Error> {
        value.convert()
    }
}

//------------------------------------------------------------------------------
// TryFrom<&SQLiteValue> implementations for borrowing without consuming
// Uses the FromSQLiteValue trait via convert_ref() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement `TryFrom`<&`SQLiteValue`> for types implementing `FromSQLiteValue`
macro_rules! impl_try_from_sqlite_value_ref {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> TryFrom<&SQLiteValue<'a>> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
                    value.convert_ref()
                }
            }
        )*
    };
}

impl_try_from_sqlite_value_ref!(
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
    f32,
    f64,
    bool,
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
    Vec<u8>,
    compact_str::CompactString,
);

#[cfg(feature = "uuid")]
impl_try_from_sqlite_value_ref!(Uuid);

#[cfg(feature = "chrono")]
impl_try_from_sqlite_value_ref!(
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::FixedOffset>,
    chrono::DateTime<chrono::Utc>,
    chrono::Duration,
);

#[cfg(feature = "time")]
impl_try_from_sqlite_value_ref!(
    time::Date,
    time::Time,
    time::PrimitiveDateTime,
    time::OffsetDateTime,
    time::Duration,
);

#[cfg(feature = "rust-decimal")]
impl_try_from_sqlite_value_ref!(rust_decimal::Decimal);

#[cfg(feature = "serde")]
impl_try_from_sqlite_value_ref!(serde_json::Value);

#[cfg(feature = "bytes")]
impl_try_from_sqlite_value_ref!(bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "smallvec")]
impl<const N: usize> TryFrom<&SQLiteValue<'_>> for smallvec::SmallVec<[u8; N]> {
    type Error = DrizzleError;

    fn try_from(value: &SQLiteValue<'_>) -> Result<Self, Self::Error> {
        value.convert_ref()
    }
}

// --- Borrowed reference types (cannot use FromSQLiteValue) ---

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Text(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to &str").into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to &[u8]").into(),
            )),
        }
    }
}
