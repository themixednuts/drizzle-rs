//! From<T> and `TryFrom`<PostgresValue> implementations

use super::PostgresValue;
use crate::prelude::*;
use drizzle_core::error::DrizzleError;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};

#[cfg(feature = "time")]
use time::{
    Date as TimeDate, Duration as TimeDuration, OffsetDateTime, PrimitiveDateTime, Time as TimeTime,
};

#[cfg(feature = "cidr")]
use cidr::{IpCidr, IpInet};

#[cfg(feature = "geo-types")]
use geo_types::{LineString, Point, Rect};

#[cfg(feature = "bit-vec")]
use bit_vec::BitVec;

#[cfg(feature = "rust-decimal")]
use rust_decimal::Decimal;

//------------------------------------------------------------------------------
// From<T> implementations
//------------------------------------------------------------------------------

// --- Integer Types ---

// i8 → SMALLINT (PostgreSQL doesn't have a native i8 type)
impl From<i8> for PostgresValue<'_> {
    fn from(value: i8) -> Self {
        PostgresValue::Smallint(i16::from(value))
    }
}

impl<'a> From<&'a i8> for PostgresValue<'a> {
    fn from(value: &'a i8) -> Self {
        PostgresValue::Smallint(i16::from(*value))
    }
}

// i16 (SMALLINT)
impl From<i16> for PostgresValue<'_> {
    fn from(value: i16) -> Self {
        PostgresValue::Smallint(value)
    }
}

impl<'a> From<&'a i16> for PostgresValue<'a> {
    fn from(value: &'a i16) -> Self {
        PostgresValue::Smallint(*value)
    }
}

// i32 (INTEGER)
impl From<i32> for PostgresValue<'_> {
    fn from(value: i32) -> Self {
        PostgresValue::Integer(value)
    }
}

impl<'a> From<&'a i32> for PostgresValue<'a> {
    fn from(value: &'a i32) -> Self {
        PostgresValue::Integer(*value)
    }
}

// i64 (BIGINT)
impl From<i64> for PostgresValue<'_> {
    fn from(value: i64) -> Self {
        PostgresValue::Bigint(value)
    }
}

impl<'a> From<&'a i64> for PostgresValue<'a> {
    fn from(value: &'a i64) -> Self {
        PostgresValue::Bigint(*value)
    }
}

// u8 → SMALLINT (PostgreSQL doesn't have unsigned types)
impl From<u8> for PostgresValue<'_> {
    fn from(value: u8) -> Self {
        PostgresValue::Smallint(i16::from(value))
    }
}

impl<'a> From<&'a u8> for PostgresValue<'a> {
    fn from(value: &'a u8) -> Self {
        PostgresValue::Smallint(i16::from(*value))
    }
}

// u16 → INTEGER (cast to larger signed type)
impl From<u16> for PostgresValue<'_> {
    fn from(value: u16) -> Self {
        PostgresValue::Integer(i32::from(value))
    }
}

impl<'a> From<&'a u16> for PostgresValue<'a> {
    fn from(value: &'a u16) -> Self {
        PostgresValue::Integer(i32::from(*value))
    }
}

// u32 → BIGINT (cast to larger signed type since u32 max > i32 max)
impl From<u32> for PostgresValue<'_> {
    fn from(value: u32) -> Self {
        PostgresValue::Bigint(i64::from(value))
    }
}

impl<'a> From<&'a u32> for PostgresValue<'a> {
    fn from(value: &'a u32) -> Self {
        PostgresValue::Bigint(i64::from(*value))
    }
}

// u64 → BIGINT (saturating to i64::MAX since u64 max > i64 max)
impl From<u64> for PostgresValue<'_> {
    fn from(value: u64) -> Self {
        PostgresValue::Bigint(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

impl<'a> From<&'a u64> for PostgresValue<'a> {
    fn from(value: &'a u64) -> Self {
        PostgresValue::Bigint(i64::try_from(*value).unwrap_or(i64::MAX))
    }
}

// isize → BIGINT (platform-dependent size)
impl From<isize> for PostgresValue<'_> {
    fn from(value: isize) -> Self {
        PostgresValue::Bigint(value as i64)
    }
}

impl<'a> From<&'a isize> for PostgresValue<'a> {
    fn from(value: &'a isize) -> Self {
        PostgresValue::Bigint(*value as i64)
    }
}

// usize → BIGINT (platform-dependent size; saturates to i64::MAX on 64-bit targets)
impl From<usize> for PostgresValue<'_> {
    fn from(value: usize) -> Self {
        PostgresValue::Bigint(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

impl<'a> From<&'a usize> for PostgresValue<'a> {
    fn from(value: &'a usize) -> Self {
        PostgresValue::Bigint(i64::try_from(*value).unwrap_or(i64::MAX))
    }
}

// --- Floating Point Types ---

// f32 (REAL)
impl From<f32> for PostgresValue<'_> {
    fn from(value: f32) -> Self {
        PostgresValue::Real(value)
    }
}

impl<'a> From<&'a f32> for PostgresValue<'a> {
    fn from(value: &'a f32) -> Self {
        PostgresValue::Real(*value)
    }
}

// f64 (DOUBLE PRECISION)
impl From<f64> for PostgresValue<'_> {
    fn from(value: f64) -> Self {
        PostgresValue::DoublePrecision(value)
    }
}

impl<'a> From<&'a f64> for PostgresValue<'a> {
    fn from(value: &'a f64) -> Self {
        PostgresValue::DoublePrecision(*value)
    }
}

#[cfg(feature = "rust-decimal")]
impl From<Decimal> for PostgresValue<'_> {
    fn from(value: Decimal) -> Self {
        PostgresValue::Numeric(value)
    }
}

#[cfg(feature = "rust-decimal")]
impl<'a> From<&'a Decimal> for PostgresValue<'a> {
    fn from(value: &'a Decimal) -> Self {
        PostgresValue::Numeric(*value)
    }
}

// --- Boolean ---

impl From<bool> for PostgresValue<'_> {
    fn from(value: bool) -> Self {
        PostgresValue::Boolean(value)
    }
}

impl<'a> From<&'a bool> for PostgresValue<'a> {
    fn from(value: &'a bool) -> Self {
        PostgresValue::Boolean(*value)
    }
}

// --- String Types ---

impl<'a> From<&'a str> for PostgresValue<'a> {
    fn from(value: &'a str) -> Self {
        PostgresValue::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, str>> for PostgresValue<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        PostgresValue::Text(value)
    }
}

impl From<String> for PostgresValue<'_> {
    fn from(value: String) -> Self {
        PostgresValue::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a String> for PostgresValue<'a> {
    fn from(value: &'a String) -> Self {
        PostgresValue::Text(Cow::Borrowed(value))
    }
}

impl From<Box<String>> for PostgresValue<'_> {
    fn from(value: Box<String>) -> Self {
        PostgresValue::Text(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<String>> for PostgresValue<'a> {
    fn from(value: &'a Box<String>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl From<Rc<String>> for PostgresValue<'_> {
    fn from(value: Rc<String>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<String>> for PostgresValue<'a> {
    fn from(value: &'a Rc<String>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl From<Arc<String>> for PostgresValue<'_> {
    fn from(value: Arc<String>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<String>> for PostgresValue<'a> {
    fn from(value: &'a Arc<String>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl From<Box<str>> for PostgresValue<'_> {
    fn from(value: Box<str>) -> Self {
        PostgresValue::Text(Cow::Owned(value.into()))
    }
}

impl<'a> From<&'a Box<str>> for PostgresValue<'a> {
    fn from(value: &'a Box<str>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl From<Rc<str>> for PostgresValue<'_> {
    fn from(value: Rc<str>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Rc<str>> for PostgresValue<'a> {
    fn from(value: &'a Rc<str>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl From<Arc<str>> for PostgresValue<'_> {
    fn from(value: Arc<str>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Arc<str>> for PostgresValue<'a> {
    fn from(value: &'a Arc<str>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

// --- ArrayString ---

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayString<N>> for PostgresValue<'_> {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        PostgresValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<&arrayvec::ArrayString<N>> for PostgresValue<'_> {
    fn from(value: &arrayvec::ArrayString<N>) -> Self {
        PostgresValue::Text(Cow::Owned(String::from(value.as_str())))
    }
}

#[cfg(feature = "compact-str")]
impl From<compact_str::CompactString> for PostgresValue<'_> {
    fn from(value: compact_str::CompactString) -> Self {
        PostgresValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "compact-str")]
impl<'a> From<&'a compact_str::CompactString> for PostgresValue<'a> {
    fn from(value: &'a compact_str::CompactString) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

// --- Binary Data ---

impl<'a> From<&'a [u8]> for PostgresValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, [u8]>> for PostgresValue<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        PostgresValue::Bytea(value)
    }
}

impl From<Vec<u8>> for PostgresValue<'_> {
    fn from(value: Vec<u8>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value))
    }
}

impl From<Box<Vec<u8>>> for PostgresValue<'_> {
    fn from(value: Box<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: &'a Box<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value.as_slice()))
    }
}

impl From<Rc<Vec<u8>>> for PostgresValue<'_> {
    fn from(value: Rc<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: &'a Rc<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value.as_slice()))
    }
}

impl From<Arc<Vec<u8>>> for PostgresValue<'_> {
    fn from(value: Arc<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: &'a Arc<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value.as_slice()))
    }
}

// --- ArrayVec<u8, N> ---

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for PostgresValue<'_> {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<&arrayvec::ArrayVec<u8, N>> for PostgresValue<'_> {
    fn from(value: &arrayvec::ArrayVec<u8, N>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::Bytes> for PostgresValue<'_> {
    fn from(value: bytes::Bytes) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl<'a> From<&'a bytes::Bytes> for PostgresValue<'a> {
    fn from(value: &'a bytes::Bytes) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::BytesMut> for PostgresValue<'_> {
    fn from(value: bytes::BytesMut) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl<'a> From<&'a bytes::BytesMut> for PostgresValue<'a> {
    fn from(value: &'a bytes::BytesMut) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<smallvec::SmallVec<[u8; N]>> for PostgresValue<'_> {
    fn from(value: smallvec::SmallVec<[u8; N]>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.into_vec()))
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<&smallvec::SmallVec<[u8; N]>> for PostgresValue<'_> {
    fn from(value: &smallvec::SmallVec<[u8; N]>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl From<Uuid> for PostgresValue<'_> {
    fn from(value: Uuid) -> Self {
        PostgresValue::Uuid(value)
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for PostgresValue<'a> {
    fn from(value: &'a Uuid) -> Self {
        PostgresValue::Uuid(*value)
    }
}

// --- JSON ---

#[cfg(feature = "serde")]
impl From<serde_json::Value> for PostgresValue<'_> {
    fn from(value: serde_json::Value) -> Self {
        PostgresValue::Json(value)
    }
}

#[cfg(feature = "serde")]
impl<'a> From<&'a serde_json::Value> for PostgresValue<'a> {
    fn from(value: &'a serde_json::Value) -> Self {
        PostgresValue::Json(value.clone())
    }
}

// --- Date/Time Types ---

#[cfg(feature = "chrono")]
impl From<NaiveDate> for PostgresValue<'_> {
    fn from(value: NaiveDate) -> Self {
        PostgresValue::Date(value)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<&'a NaiveDate> for PostgresValue<'a> {
    fn from(value: &'a NaiveDate) -> Self {
        PostgresValue::Date(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<NaiveTime> for PostgresValue<'_> {
    fn from(value: NaiveTime) -> Self {
        PostgresValue::Time(value)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<&'a NaiveTime> for PostgresValue<'a> {
    fn from(value: &'a NaiveTime) -> Self {
        PostgresValue::Time(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<NaiveDateTime> for PostgresValue<'_> {
    fn from(value: NaiveDateTime) -> Self {
        PostgresValue::Timestamp(value)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<&'a NaiveDateTime> for PostgresValue<'a> {
    fn from(value: &'a NaiveDateTime) -> Self {
        PostgresValue::Timestamp(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<DateTime<FixedOffset>> for PostgresValue<'_> {
    fn from(value: DateTime<FixedOffset>) -> Self {
        PostgresValue::TimestampTz(value)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<&'a DateTime<FixedOffset>> for PostgresValue<'a> {
    fn from(value: &'a DateTime<FixedOffset>) -> Self {
        PostgresValue::TimestampTz(*value)
    }
}

#[cfg(feature = "chrono")]
impl From<DateTime<Utc>> for PostgresValue<'_> {
    fn from(value: DateTime<Utc>) -> Self {
        PostgresValue::TimestampTz(value.into())
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<&'a DateTime<Utc>> for PostgresValue<'a> {
    fn from(value: &'a DateTime<Utc>) -> Self {
        PostgresValue::TimestampTz((*value).into())
    }
}

#[cfg(feature = "chrono")]
impl From<Duration> for PostgresValue<'_> {
    fn from(value: Duration) -> Self {
        PostgresValue::Interval(value)
    }
}

#[cfg(feature = "chrono")]
impl<'a> From<&'a Duration> for PostgresValue<'a> {
    fn from(value: &'a Duration) -> Self {
        PostgresValue::Interval(*value)
    }
}

// --- Date/Time Types (time crate) ---

#[cfg(feature = "time")]
impl From<TimeDate> for PostgresValue<'_> {
    fn from(value: TimeDate) -> Self {
        PostgresValue::TimeDate(value)
    }
}

#[cfg(feature = "time")]
impl<'a> From<&'a TimeDate> for PostgresValue<'a> {
    fn from(value: &'a TimeDate) -> Self {
        PostgresValue::TimeDate(*value)
    }
}

#[cfg(feature = "time")]
impl From<TimeTime> for PostgresValue<'_> {
    fn from(value: TimeTime) -> Self {
        PostgresValue::TimeTime(value)
    }
}

#[cfg(feature = "time")]
impl<'a> From<&'a TimeTime> for PostgresValue<'a> {
    fn from(value: &'a TimeTime) -> Self {
        PostgresValue::TimeTime(*value)
    }
}

#[cfg(feature = "time")]
impl From<PrimitiveDateTime> for PostgresValue<'_> {
    fn from(value: PrimitiveDateTime) -> Self {
        PostgresValue::TimeTimestamp(value)
    }
}

#[cfg(feature = "time")]
impl<'a> From<&'a PrimitiveDateTime> for PostgresValue<'a> {
    fn from(value: &'a PrimitiveDateTime) -> Self {
        PostgresValue::TimeTimestamp(*value)
    }
}

#[cfg(feature = "time")]
impl From<OffsetDateTime> for PostgresValue<'_> {
    fn from(value: OffsetDateTime) -> Self {
        PostgresValue::TimeTimestampTz(value)
    }
}

#[cfg(feature = "time")]
impl<'a> From<&'a OffsetDateTime> for PostgresValue<'a> {
    fn from(value: &'a OffsetDateTime) -> Self {
        PostgresValue::TimeTimestampTz(*value)
    }
}

#[cfg(feature = "time")]
impl From<TimeDuration> for PostgresValue<'_> {
    fn from(value: TimeDuration) -> Self {
        PostgresValue::TimeInterval(value)
    }
}

#[cfg(feature = "time")]
impl<'a> From<&'a TimeDuration> for PostgresValue<'a> {
    fn from(value: &'a TimeDuration) -> Self {
        PostgresValue::TimeInterval(*value)
    }
}

// --- Network Address Types ---

#[cfg(feature = "cidr")]
impl From<IpInet> for PostgresValue<'_> {
    fn from(value: IpInet) -> Self {
        PostgresValue::Inet(value)
    }
}

#[cfg(feature = "cidr")]
impl<'a> From<&'a IpInet> for PostgresValue<'a> {
    fn from(value: &'a IpInet) -> Self {
        PostgresValue::Inet(*value)
    }
}

#[cfg(feature = "cidr")]
impl From<IpCidr> for PostgresValue<'_> {
    fn from(value: IpCidr) -> Self {
        PostgresValue::Cidr(value)
    }
}

#[cfg(feature = "cidr")]
impl<'a> From<&'a IpCidr> for PostgresValue<'a> {
    fn from(value: &'a IpCidr) -> Self {
        PostgresValue::Cidr(*value)
    }
}

#[cfg(feature = "cidr")]
impl From<[u8; 6]> for PostgresValue<'_> {
    fn from(value: [u8; 6]) -> Self {
        PostgresValue::MacAddr(value)
    }
}

#[cfg(feature = "cidr")]
impl<'a> From<&'a [u8; 6]> for PostgresValue<'a> {
    fn from(value: &'a [u8; 6]) -> Self {
        PostgresValue::MacAddr(*value)
    }
}

#[cfg(feature = "cidr")]
impl From<[u8; 8]> for PostgresValue<'_> {
    fn from(value: [u8; 8]) -> Self {
        PostgresValue::MacAddr8(value)
    }
}

#[cfg(feature = "cidr")]
impl<'a> From<&'a [u8; 8]> for PostgresValue<'a> {
    fn from(value: &'a [u8; 8]) -> Self {
        PostgresValue::MacAddr8(*value)
    }
}

// --- Geometric Types ---

#[cfg(feature = "geo-types")]
impl From<Point<f64>> for PostgresValue<'_> {
    fn from(value: Point<f64>) -> Self {
        PostgresValue::Point(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a Point<f64>> for PostgresValue<'a> {
    fn from(value: &'a Point<f64>) -> Self {
        PostgresValue::Point(*value)
    }
}

#[cfg(feature = "geo-types")]
impl From<LineString<f64>> for PostgresValue<'_> {
    fn from(value: LineString<f64>) -> Self {
        PostgresValue::LineString(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a LineString<f64>> for PostgresValue<'a> {
    fn from(value: &'a LineString<f64>) -> Self {
        PostgresValue::LineString(value.clone())
    }
}

#[cfg(feature = "geo-types")]
impl From<Rect<f64>> for PostgresValue<'_> {
    fn from(value: Rect<f64>) -> Self {
        PostgresValue::Rect(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a Rect<f64>> for PostgresValue<'a> {
    fn from(value: &'a Rect<f64>) -> Self {
        PostgresValue::Rect(*value)
    }
}

// --- Bit String Types ---

#[cfg(feature = "bit-vec")]
impl From<BitVec> for PostgresValue<'_> {
    fn from(value: BitVec) -> Self {
        PostgresValue::BitVec(value)
    }
}

#[cfg(feature = "bit-vec")]
impl<'a> From<&'a BitVec> for PostgresValue<'a> {
    fn from(value: &'a BitVec) -> Self {
        PostgresValue::BitVec(value.clone())
    }
}

// --- Array Types ---

impl From<Vec<Self>> for PostgresValue<'_> {
    fn from(value: Vec<Self>) -> Self {
        PostgresValue::Array(value)
    }
}

impl<'a> From<&'a [Self]> for PostgresValue<'a> {
    fn from(value: &'a [Self]) -> Self {
        PostgresValue::Array(value.to_vec())
    }
}

impl From<Vec<String>> for PostgresValue<'_> {
    fn from(value: Vec<String>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<&'a str>> for PostgresValue<'a> {
    fn from(value: Vec<&'a str>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl From<Vec<i16>> for PostgresValue<'_> {
    fn from(value: Vec<i16>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl From<Vec<i32>> for PostgresValue<'_> {
    fn from(value: Vec<i32>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl From<Vec<i64>> for PostgresValue<'_> {
    fn from(value: Vec<i64>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl From<Vec<f32>> for PostgresValue<'_> {
    fn from(value: Vec<f32>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl From<Vec<f64>> for PostgresValue<'_> {
    fn from(value: Vec<f64>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl From<Vec<bool>> for PostgresValue<'_> {
    fn from(value: Vec<bool>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

// --- Extended Array Types ---

#[cfg(feature = "uuid")]
impl From<Vec<uuid::Uuid>> for PostgresValue<'_> {
    fn from(value: Vec<uuid::Uuid>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

#[cfg(feature = "chrono")]
impl From<Vec<NaiveDate>> for PostgresValue<'_> {
    fn from(value: Vec<NaiveDate>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

#[cfg(feature = "chrono")]
impl From<Vec<NaiveTime>> for PostgresValue<'_> {
    fn from(value: Vec<NaiveTime>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

#[cfg(feature = "chrono")]
impl From<Vec<NaiveDateTime>> for PostgresValue<'_> {
    fn from(value: Vec<NaiveDateTime>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

#[cfg(feature = "chrono")]
impl From<Vec<DateTime<Utc>>> for PostgresValue<'_> {
    fn from(value: Vec<DateTime<Utc>>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

#[cfg(feature = "rust-decimal")]
impl From<Vec<Decimal>> for PostgresValue<'_> {
    fn from(value: Vec<Decimal>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

// --- Option Types ---
impl<T> From<Option<T>> for PostgresValue<'_>
where
    T: TryInto<Self>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(PostgresValue::Null, |v| {
            v.try_into().unwrap_or(PostgresValue::Null)
        })
    }
}

//------------------------------------------------------------------------------
// TryFrom<PostgresValue> implementations
//------------------------------------------------------------------------------

// --- Integer Types ---

impl<'a> TryFrom<PostgresValue<'a>> for i16 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Smallint(i) => Ok(i),
            PostgresValue::Integer(i) => Ok(i.try_into()?),
            PostgresValue::Bigint(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to i16").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for i32 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Smallint(i) => Ok(i.into()),
            PostgresValue::Integer(i) => Ok(i),
            PostgresValue::Bigint(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to i32").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for i64 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Smallint(i) => Ok(i.into()),
            PostgresValue::Integer(i) => Ok(i.into()),
            PostgresValue::Bigint(i) => Ok(i),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to i64").into(),
            )),
        }
    }
}

// --- Floating Point Types ---

impl<'a> TryFrom<PostgresValue<'a>> for f32 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        fn parse_float<N: core::fmt::Display>(value: N) -> Result<f32, DrizzleError> {
            let s = format!("{value}");
            s.parse::<f32>().map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to convert {s} to f32: {e}").into())
            })
        }

        match value {
            PostgresValue::Real(f) => Ok(f),
            PostgresValue::DoublePrecision(f) => parse_float(f),
            PostgresValue::Smallint(i) => Ok(Self::from(i)),
            PostgresValue::Integer(i) => parse_float(i),
            PostgresValue::Bigint(i) => parse_float(i),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to f32").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for f64 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Real(f) => Ok(Self::from(f)),
            PostgresValue::DoublePrecision(f) => Ok(f),
            PostgresValue::Smallint(i) => Ok(Self::from(i)),
            PostgresValue::Integer(i) => Ok(Self::from(i)),
            PostgresValue::Bigint(i) => {
                let s = format!("{i}");
                s.parse::<Self>().map_err(|e| {
                    DrizzleError::ConversionError(
                        format!("Failed to convert {s} to f64: {e}").into(),
                    )
                })
            }
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to f64").into(),
            )),
        }
    }
}

#[cfg(feature = "rust-decimal")]
impl<'a> TryFrom<PostgresValue<'a>> for Decimal {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Numeric(d) => Ok(d),
            PostgresValue::Text(cow) => Self::from_str_exact(cow.as_ref()).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse DECIMAL: {e}").into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Decimal").into(),
            )),
        }
    }
}

#[cfg(feature = "rust-decimal")]
impl<'a> TryFrom<&'a PostgresValue<'a>> for Decimal {
    type Error = DrizzleError;

    fn try_from(value: &'a PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Numeric(d) => Ok(*d),
            PostgresValue::Text(cow) => Self::from_str_exact(cow.as_ref()).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse DECIMAL: {e}").into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Decimal").into(),
            )),
        }
    }
}

// --- Boolean ---

impl<'a> TryFrom<PostgresValue<'a>> for bool {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Boolean(b) => Ok(b),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to bool").into(),
            )),
        }
    }
}

// --- String Types ---

impl<'a> TryFrom<PostgresValue<'a>> for String {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(cow.into_owned()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to String").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Box<String> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Self::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Rc<String> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Self::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Arc<String> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Self::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Box<str> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(cow.into_owned().into_boxed_str()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Box<str>").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Rc<str> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(Self::from(cow.into_owned())),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Rc<str>").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Arc<str> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(Self::from(cow.into_owned())),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Arc<str>").into(),
            )),
        }
    }
}

#[cfg(feature = "compact-str")]
impl<'a> TryFrom<PostgresValue<'a>> for compact_str::CompactString {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Self::new)
    }
}

// --- Binary Data ---

impl<'a> TryFrom<PostgresValue<'a>> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Bytea(cow) => Ok(cow.into_owned()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Vec<u8>").into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Box<Vec<u8>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Self::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Rc<Vec<u8>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Self::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Arc<Vec<u8>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Self::new)
    }
}

#[cfg(feature = "bytes")]
impl<'a> TryFrom<PostgresValue<'a>> for bytes::Bytes {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Self::from)
    }
}

#[cfg(feature = "bytes")]
impl<'a> TryFrom<PostgresValue<'a>> for bytes::BytesMut {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(|v| Self::from(v.as_slice()))
    }
}

#[cfg(feature = "smallvec")]
impl<'a, const N: usize> TryFrom<PostgresValue<'a>> for smallvec::SmallVec<[u8; N]> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(|v| {
            let mut out = Self::new();
            out.extend_from_slice(&v);
            out
        })
    }
}

impl<'a> TryFrom<&'a PostgresValue<'a>> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to &str").into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a PostgresValue<'a>> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Bytea(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to &[u8]").into(),
            )),
        }
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> TryFrom<PostgresValue<'a>> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Uuid(uuid) => Ok(uuid),
            PostgresValue::Text(cow) => Self::parse_str(cow.as_ref()).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse UUID: {e}").into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to UUID").into(),
            )),
        }
    }
}

// --- JSON ---

#[cfg(feature = "serde")]
impl<'a> TryFrom<PostgresValue<'a>> for serde_json::Value {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Json(json) | PostgresValue::Jsonb(json) => Ok(json),
            PostgresValue::Text(cow) => serde_json::from_str(cow.as_ref()).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse JSON: {e}").into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to JSON").into(),
            )),
        }
    }
}

// --- Date/Time TryFrom implementations ---

#[cfg(feature = "chrono")]
impl<'a> TryFrom<PostgresValue<'a>> for NaiveDate {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Date(date) => Ok(date),
            PostgresValue::Timestamp(ts) => Ok(ts.date()),
            PostgresValue::TimestampTz(ts) => Ok(ts.date_naive()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to NaiveDate").into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<'a> TryFrom<PostgresValue<'a>> for NaiveTime {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Time(time) => Ok(time),
            PostgresValue::Timestamp(ts) => Ok(ts.time()),
            PostgresValue::TimestampTz(ts) => Ok(ts.time()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to NaiveTime").into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<'a> TryFrom<PostgresValue<'a>> for NaiveDateTime {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Timestamp(ts) => Ok(ts),
            PostgresValue::TimestampTz(ts) => Ok(ts.naive_utc()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to NaiveDateTime").into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<'a> TryFrom<PostgresValue<'a>> for DateTime<FixedOffset> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimestampTz(ts) => Ok(ts),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to DateTime<FixedOffset>").into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<'a> TryFrom<PostgresValue<'a>> for DateTime<Utc> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimestampTz(ts) => Ok(ts.with_timezone(&Utc)),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to DateTime<Utc>").into(),
            )),
        }
    }
}

#[cfg(feature = "chrono")]
impl<'a> TryFrom<PostgresValue<'a>> for Duration {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Interval(duration) => Ok(duration),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Duration").into(),
            )),
        }
    }
}

// --- Date/Time TryFrom implementations (time crate) ---

#[cfg(feature = "time")]
impl<'a> TryFrom<PostgresValue<'a>> for TimeDate {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimeDate(date) => Ok(date),
            PostgresValue::TimeTimestamp(ts) => Ok(ts.date()),
            PostgresValue::TimeTimestampTz(ts) => Ok(ts.date()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to time::Date").into(),
            )),
        }
    }
}

#[cfg(feature = "time")]
impl<'a> TryFrom<PostgresValue<'a>> for TimeTime {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimeTime(time) => Ok(time),
            PostgresValue::TimeTimestamp(ts) => Ok(ts.time()),
            PostgresValue::TimeTimestampTz(ts) => Ok(ts.time()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to time::Time").into(),
            )),
        }
    }
}

#[cfg(feature = "time")]
impl<'a> TryFrom<PostgresValue<'a>> for PrimitiveDateTime {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimeTimestamp(ts) => Ok(ts),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to time::PrimitiveDateTime").into(),
            )),
        }
    }
}

#[cfg(feature = "time")]
impl<'a> TryFrom<PostgresValue<'a>> for OffsetDateTime {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimeTimestampTz(ts) => Ok(ts),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to time::OffsetDateTime").into(),
            )),
        }
    }
}

#[cfg(feature = "time")]
impl<'a> TryFrom<PostgresValue<'a>> for TimeDuration {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::TimeInterval(dur) => Ok(dur),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to time::Duration").into(),
            )),
        }
    }
}

// --- Network Address TryFrom implementations ---

#[cfg(feature = "cidr")]
impl<'a> TryFrom<PostgresValue<'a>> for IpInet {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Inet(net) => Ok(net),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to IpInet").into(),
            )),
        }
    }
}

#[cfg(feature = "cidr")]
impl<'a> TryFrom<PostgresValue<'a>> for IpCidr {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Cidr(net) => Ok(net),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to IpCidr").into(),
            )),
        }
    }
}

#[cfg(feature = "cidr")]
impl<'a> TryFrom<PostgresValue<'a>> for [u8; 6] {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::MacAddr(mac) => Ok(mac),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to [u8; 6]").into(),
            )),
        }
    }
}

#[cfg(feature = "cidr")]
impl<'a> TryFrom<PostgresValue<'a>> for [u8; 8] {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::MacAddr8(mac) => Ok(mac),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to [u8; 8]").into(),
            )),
        }
    }
}

// --- Geometric TryFrom implementations ---

#[cfg(feature = "geo-types")]
impl<'a> TryFrom<PostgresValue<'a>> for Point<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Point(point) => Ok(point),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Point").into(),
            )),
        }
    }
}

#[cfg(feature = "geo-types")]
impl<'a> TryFrom<PostgresValue<'a>> for LineString<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::LineString(line) => Ok(line),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to LineString").into(),
            )),
        }
    }
}

#[cfg(feature = "geo-types")]
impl<'a> TryFrom<PostgresValue<'a>> for Rect<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Rect(rect) => Ok(rect),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Rect").into(),
            )),
        }
    }
}

// --- Bit String TryFrom implementations ---

#[cfg(feature = "bit-vec")]
impl<'a> TryFrom<PostgresValue<'a>> for BitVec {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::BitVec(bv) => Ok(bv),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to BitVec").into(),
            )),
        }
    }
}

// --- ArrayVec TryFrom implementations ---

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> TryFrom<PostgresValue<'a>> for arrayvec::ArrayString<N> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow_str) => Self::from(cow_str.as_ref()).map_err(|_| {
                DrizzleError::ConversionError(
                    format!(
                        "Text length {} exceeds ArrayString capacity {}",
                        cow_str.len(),
                        N
                    )
                    .into(),
                )
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to ArrayString").into(),
            )),
        }
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> TryFrom<PostgresValue<'a>> for arrayvec::ArrayVec<u8, N> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Bytea(cow_bytes) => Self::try_from(cow_bytes.as_ref()).map_err(|_| {
                DrizzleError::ConversionError(
                    format!(
                        "Bytea length {} exceeds ArrayVec capacity {}",
                        cow_bytes.len(),
                        N
                    )
                    .into(),
                )
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to ArrayVec<u8>").into(),
            )),
        }
    }
}

// --- Array TryFrom implementations ---

impl<'a> TryFrom<PostgresValue<'a>> for Vec<PostgresValue<'a>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Array(arr) => Ok(arr),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to Vec<PostgresValue>").into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a PostgresValue<'a>> for &'a [PostgresValue<'a>] {
    type Error = DrizzleError;

    fn try_from(value: &'a PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Array(arr) => Ok(arr),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {value:?} to &[PostgresValue]").into(),
            )),
        }
    }
}
