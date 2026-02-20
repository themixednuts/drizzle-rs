//! From<T> and TryFrom<PostgresValue> implementations

use super::PostgresValue;
use crate::prelude::*;
use drizzle_core::error::DrizzleError;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};

#[cfg(feature = "cidr")]
use cidr::{IpCidr, IpInet};

#[cfg(feature = "geo-types")]
use geo_types::{LineString, Point, Rect};

#[cfg(feature = "bit-vec")]
use bit_vec::BitVec;

//------------------------------------------------------------------------------
// From<T> implementations
//------------------------------------------------------------------------------

// --- Integer Types ---

// i8 → SMALLINT (PostgreSQL doesn't have a native i8 type)
impl<'a> From<i8> for PostgresValue<'a> {
    fn from(value: i8) -> Self {
        PostgresValue::Smallint(value as i16)
    }
}

impl<'a> From<&'a i8> for PostgresValue<'a> {
    fn from(value: &'a i8) -> Self {
        PostgresValue::Smallint(*value as i16)
    }
}

// i16 (SMALLINT)
impl<'a> From<i16> for PostgresValue<'a> {
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
impl<'a> From<i32> for PostgresValue<'a> {
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
impl<'a> From<i64> for PostgresValue<'a> {
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
impl<'a> From<u8> for PostgresValue<'a> {
    fn from(value: u8) -> Self {
        PostgresValue::Smallint(value as i16)
    }
}

impl<'a> From<&'a u8> for PostgresValue<'a> {
    fn from(value: &'a u8) -> Self {
        PostgresValue::Smallint(*value as i16)
    }
}

// u16 → INTEGER (cast to larger signed type)
impl<'a> From<u16> for PostgresValue<'a> {
    fn from(value: u16) -> Self {
        PostgresValue::Integer(value as i32)
    }
}

impl<'a> From<&'a u16> for PostgresValue<'a> {
    fn from(value: &'a u16) -> Self {
        PostgresValue::Integer(*value as i32)
    }
}

// u32 → BIGINT (cast to larger signed type since u32 max > i32 max)
impl<'a> From<u32> for PostgresValue<'a> {
    fn from(value: u32) -> Self {
        PostgresValue::Bigint(value as i64)
    }
}

impl<'a> From<&'a u32> for PostgresValue<'a> {
    fn from(value: &'a u32) -> Self {
        PostgresValue::Bigint(*value as i64)
    }
}

// u64 → BIGINT (cast with potential overflow since u64 max > i64 max)
impl<'a> From<u64> for PostgresValue<'a> {
    fn from(value: u64) -> Self {
        PostgresValue::Bigint(value as i64)
    }
}

impl<'a> From<&'a u64> for PostgresValue<'a> {
    fn from(value: &'a u64) -> Self {
        PostgresValue::Bigint(*value as i64)
    }
}

// isize → BIGINT (platform-dependent size)
impl<'a> From<isize> for PostgresValue<'a> {
    fn from(value: isize) -> Self {
        PostgresValue::Bigint(value as i64)
    }
}

impl<'a> From<&'a isize> for PostgresValue<'a> {
    fn from(value: &'a isize) -> Self {
        PostgresValue::Bigint(*value as i64)
    }
}

// usize → BIGINT (platform-dependent size)
impl<'a> From<usize> for PostgresValue<'a> {
    fn from(value: usize) -> Self {
        PostgresValue::Bigint(value as i64)
    }
}

impl<'a> From<&'a usize> for PostgresValue<'a> {
    fn from(value: &'a usize) -> Self {
        PostgresValue::Bigint(*value as i64)
    }
}

// --- Floating Point Types ---

// f32 (REAL)
impl<'a> From<f32> for PostgresValue<'a> {
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
impl<'a> From<f64> for PostgresValue<'a> {
    fn from(value: f64) -> Self {
        PostgresValue::DoublePrecision(value)
    }
}

impl<'a> From<&'a f64> for PostgresValue<'a> {
    fn from(value: &'a f64) -> Self {
        PostgresValue::DoublePrecision(*value)
    }
}

// --- Boolean ---

impl<'a> From<bool> for PostgresValue<'a> {
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

impl<'a> From<String> for PostgresValue<'a> {
    fn from(value: String) -> Self {
        PostgresValue::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a String> for PostgresValue<'a> {
    fn from(value: &'a String) -> Self {
        PostgresValue::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<Box<String>> for PostgresValue<'a> {
    fn from(value: Box<String>) -> Self {
        PostgresValue::Text(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<String>> for PostgresValue<'a> {
    fn from(value: &'a Box<String>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<Rc<String>> for PostgresValue<'a> {
    fn from(value: Rc<String>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<String>> for PostgresValue<'a> {
    fn from(value: &'a Rc<String>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<Arc<String>> for PostgresValue<'a> {
    fn from(value: Arc<String>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<String>> for PostgresValue<'a> {
    fn from(value: &'a Arc<String>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<Box<str>> for PostgresValue<'a> {
    fn from(value: Box<str>) -> Self {
        PostgresValue::Text(Cow::Owned(value.into()))
    }
}

impl<'a> From<&'a Box<str>> for PostgresValue<'a> {
    fn from(value: &'a Box<str>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl<'a> From<Rc<str>> for PostgresValue<'a> {
    fn from(value: Rc<str>) -> Self {
        PostgresValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Rc<str>> for PostgresValue<'a> {
    fn from(value: &'a Rc<str>) -> Self {
        PostgresValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl<'a> From<Arc<str>> for PostgresValue<'a> {
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
impl<'a, const N: usize> From<arrayvec::ArrayString<N>> for PostgresValue<'a> {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        PostgresValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&arrayvec::ArrayString<N>> for PostgresValue<'a> {
    fn from(value: &arrayvec::ArrayString<N>) -> Self {
        PostgresValue::Text(Cow::Owned(String::from(value.as_str())))
    }
}

#[cfg(feature = "compact-str")]
impl<'a> From<compact_str::CompactString> for PostgresValue<'a> {
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

impl<'a> From<Vec<u8>> for PostgresValue<'a> {
    fn from(value: Vec<u8>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value))
    }
}

impl<'a> From<Box<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: Box<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: &'a Box<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value.as_slice()))
    }
}

impl<'a> From<Rc<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: Rc<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<Vec<u8>>> for PostgresValue<'a> {
    fn from(value: &'a Rc<Vec<u8>>) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value.as_slice()))
    }
}

impl<'a> From<Arc<Vec<u8>>> for PostgresValue<'a> {
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
impl<'a, const N: usize> From<arrayvec::ArrayVec<u8, N>> for PostgresValue<'a> {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&arrayvec::ArrayVec<u8, N>> for PostgresValue<'a> {
    fn from(value: &arrayvec::ArrayVec<u8, N>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "bytes")]
impl<'a> From<bytes::Bytes> for PostgresValue<'a> {
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
impl<'a> From<bytes::BytesMut> for PostgresValue<'a> {
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
impl<'a, const N: usize> From<smallvec::SmallVec<[u8; N]>> for PostgresValue<'a> {
    fn from(value: smallvec::SmallVec<[u8; N]>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.into_vec()))
    }
}

#[cfg(feature = "smallvec")]
impl<'a, const N: usize> From<&smallvec::SmallVec<[u8; N]>> for PostgresValue<'a> {
    fn from(value: &smallvec::SmallVec<[u8; N]>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value.to_vec()))
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> From<Uuid> for PostgresValue<'a> {
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
impl<'a> From<serde_json::Value> for PostgresValue<'a> {
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
impl<'a> From<NaiveDate> for PostgresValue<'a> {
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
impl<'a> From<NaiveTime> for PostgresValue<'a> {
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
impl<'a> From<NaiveDateTime> for PostgresValue<'a> {
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
impl<'a> From<DateTime<FixedOffset>> for PostgresValue<'a> {
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
impl<'a> From<DateTime<Utc>> for PostgresValue<'a> {
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
impl<'a> From<Duration> for PostgresValue<'a> {
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

// --- Network Address Types ---

#[cfg(feature = "cidr")]
impl<'a> From<IpInet> for PostgresValue<'a> {
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
impl<'a> From<IpCidr> for PostgresValue<'a> {
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
impl<'a> From<[u8; 6]> for PostgresValue<'a> {
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
impl<'a> From<[u8; 8]> for PostgresValue<'a> {
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
impl<'a> From<Point<f64>> for PostgresValue<'a> {
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
impl<'a> From<LineString<f64>> for PostgresValue<'a> {
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
impl<'a> From<Rect<f64>> for PostgresValue<'a> {
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
impl<'a> From<BitVec> for PostgresValue<'a> {
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

impl<'a> From<Vec<PostgresValue<'a>>> for PostgresValue<'a> {
    fn from(value: Vec<PostgresValue<'a>>) -> Self {
        PostgresValue::Array(value)
    }
}

impl<'a> From<&'a [PostgresValue<'a>]> for PostgresValue<'a> {
    fn from(value: &'a [PostgresValue<'a>]) -> Self {
        PostgresValue::Array(value.to_vec())
    }
}

impl<'a> From<Vec<String>> for PostgresValue<'a> {
    fn from(value: Vec<String>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<&'a str>> for PostgresValue<'a> {
    fn from(value: Vec<&'a str>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<i16>> for PostgresValue<'a> {
    fn from(value: Vec<i16>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<i32>> for PostgresValue<'a> {
    fn from(value: Vec<i32>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<i64>> for PostgresValue<'a> {
    fn from(value: Vec<i64>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<f32>> for PostgresValue<'a> {
    fn from(value: Vec<f32>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<f64>> for PostgresValue<'a> {
    fn from(value: Vec<f64>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

impl<'a> From<Vec<bool>> for PostgresValue<'a> {
    fn from(value: Vec<bool>) -> Self {
        PostgresValue::Array(value.into_iter().map(PostgresValue::from).collect())
    }
}

// --- Option Types ---
impl<'a, T> From<Option<T>> for PostgresValue<'a>
where
    T: TryInto<PostgresValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.try_into().unwrap_or(PostgresValue::Null),
            None => PostgresValue::Null,
        }
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
                format!("Cannot convert {:?} to i16", value).into(),
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
                format!("Cannot convert {:?} to i32", value).into(),
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
                format!("Cannot convert {:?} to i64", value).into(),
            )),
        }
    }
}

// --- Floating Point Types ---

impl<'a> TryFrom<PostgresValue<'a>> for f32 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Real(f) => Ok(f),
            PostgresValue::DoublePrecision(f) => Ok(f as f32),
            PostgresValue::Smallint(i) => Ok(i as f32),
            PostgresValue::Integer(i) => Ok(i as f32),
            PostgresValue::Bigint(i) => Ok(i as f32),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to f32", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for f64 {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Real(f) => Ok(f as f64),
            PostgresValue::DoublePrecision(f) => Ok(f),
            PostgresValue::Smallint(i) => Ok(i as f64),
            PostgresValue::Integer(i) => Ok(i as f64),
            PostgresValue::Bigint(i) => Ok(i as f64),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to f64", value).into(),
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
                format!("Cannot convert {:?} to bool", value).into(),
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
                format!("Cannot convert {:?} to String", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Box<String> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Box::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Rc<String> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Rc::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Arc<String> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(Arc::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Box<str> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(cow.into_owned().into_boxed_str()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Box<str>", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Rc<str> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(Rc::from(cow.into_owned())),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Rc<str>", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Arc<str> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Text(cow) => Ok(Arc::from(cow.into_owned())),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Arc<str>", value).into(),
            )),
        }
    }
}

#[cfg(feature = "compact-str")]
impl<'a> TryFrom<PostgresValue<'a>> for compact_str::CompactString {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        String::try_from(value).map(compact_str::CompactString::new)
    }
}

// --- Binary Data ---

impl<'a> TryFrom<PostgresValue<'a>> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Bytea(cow) => Ok(cow.into_owned()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Vec<u8>", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Box<Vec<u8>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Box::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Rc<Vec<u8>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Rc::new)
    }
}

impl<'a> TryFrom<PostgresValue<'a>> for Arc<Vec<u8>> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(Arc::new)
    }
}

#[cfg(feature = "bytes")]
impl<'a> TryFrom<PostgresValue<'a>> for bytes::Bytes {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(bytes::Bytes::from)
    }
}

#[cfg(feature = "bytes")]
impl<'a> TryFrom<PostgresValue<'a>> for bytes::BytesMut {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(|v| bytes::BytesMut::from(v.as_slice()))
    }
}

#[cfg(feature = "smallvec")]
impl<'a, const N: usize> TryFrom<PostgresValue<'a>> for smallvec::SmallVec<[u8; N]> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(|v| {
            let mut out = smallvec::SmallVec::<[u8; N]>::new();
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
                format!("Cannot convert {:?} to &str", value).into(),
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
                format!("Cannot convert {:?} to &[u8]", value).into(),
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
            PostgresValue::Text(cow) => Uuid::parse_str(cow.as_ref()).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse UUID: {}", e).into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to UUID", value).into(),
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
            PostgresValue::Json(json) => Ok(json),
            PostgresValue::Jsonb(json) => Ok(json),
            PostgresValue::Text(cow) => serde_json::from_str(cow.as_ref()).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse JSON: {}", e).into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to JSON", value).into(),
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
                format!("Cannot convert {:?} to NaiveDate", value).into(),
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
                format!("Cannot convert {:?} to NaiveTime", value).into(),
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
                format!("Cannot convert {:?} to NaiveDateTime", value).into(),
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
                format!("Cannot convert {:?} to DateTime<FixedOffset>", value).into(),
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
                format!("Cannot convert {:?} to DateTime<Utc>", value).into(),
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
                format!("Cannot convert {:?} to Duration", value).into(),
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
                format!("Cannot convert {:?} to IpInet", value).into(),
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
                format!("Cannot convert {:?} to IpCidr", value).into(),
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
                format!("Cannot convert {:?} to [u8; 6]", value).into(),
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
                format!("Cannot convert {:?} to [u8; 8]", value).into(),
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
                format!("Cannot convert {:?} to Point", value).into(),
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
                format!("Cannot convert {:?} to LineString", value).into(),
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
                format!("Cannot convert {:?} to Rect", value).into(),
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
                format!("Cannot convert {:?} to BitVec", value).into(),
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
            PostgresValue::Text(cow_str) => {
                arrayvec::ArrayString::from(cow_str.as_ref()).map_err(|_| {
                    DrizzleError::ConversionError(
                        format!(
                            "Text length {} exceeds ArrayString capacity {}",
                            cow_str.len(),
                            N
                        )
                        .into(),
                    )
                })
            }
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to ArrayString", value).into(),
            )),
        }
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> TryFrom<PostgresValue<'a>> for arrayvec::ArrayVec<u8, N> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Bytea(cow_bytes) => arrayvec::ArrayVec::try_from(cow_bytes.as_ref())
                .map_err(|_| {
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
                format!("Cannot convert {:?} to ArrayVec<u8>", value).into(),
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
                format!("Cannot convert {:?} to Vec<PostgresValue>", value).into(),
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
                format!("Cannot convert {:?} to &[PostgresValue]", value).into(),
            )),
        }
    }
}
