//! Shared contracts between the MySQL dialect and wire-driver adapters.
//!
//! This module deliberately contains no connection, pool, runtime, or stream
//! abstraction. It owns only the data that crosses that boundary: an ordered
//! prepared request, a borrowed row view, and checked value decoding.

use crate::{
    prelude::*,
    values::{MySQLValue, OwnedMySQLValue},
};
use drizzle_core::{FromDrizzleRow, error::DrizzleError};

#[cfg(not(feature = "std"))]
use alloc::format;

/// Positional SQL and parameters ready for a wire driver's prepared protocol.
///
/// Construct this after [`drizzle_core::prepared::PreparedStatement::bind`]. Keeping the
/// parameters in a vector avoids the tuple-arity limits in MySQL client APIs.
#[derive(Debug, Clone, PartialEq)]
pub struct MySQLPreparedRequest<'a> {
    sql: &'a str,
    params: Vec<MySQLValue<'a>>,
}

impl<'a> MySQLPreparedRequest<'a> {
    /// Collects bound values in placeholder order.
    #[must_use]
    pub fn new(sql: &'a str, params: impl IntoIterator<Item = MySQLValue<'a>>) -> Self {
        Self {
            sql,
            params: params.into_iter().collect(),
        }
    }

    /// SQL rendered with MySQL `?` placeholders.
    #[must_use]
    pub const fn sql(&self) -> &str {
        self.sql
    }

    /// Bound values in placeholder order.
    #[must_use]
    pub fn params(&self) -> &[MySQLValue<'a>] {
        &self.params
    }
}

/// Adapter-owned row storage exposed as client-neutral MySQL values.
///
/// Implementations must return `Ok(None)` for a missing or previously consumed
/// cell. They must never panic for an invalid offset or conversion.
pub trait MySQLRowAccess {
    /// Returns a borrowed value at `offset`, if present.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when the underlying row cannot be inspected.
    fn value_at(&self, offset: usize) -> Result<Option<MySQLValue<'_>>, DrizzleError>;

    /// Returns whether the cell uses MySQL's packed `BIT` wire representation.
    ///
    /// Client-neutral rows return `false`; wire adapters override this from
    /// their column metadata so the requested Rust decoder decides whether to
    /// interpret the bytes numerically or preserve them as bytes.
    fn is_bit_at(&self, _offset: usize) -> bool {
        false
    }
}

/// Borrowed, non-owning view used by [`FromDrizzleRow`] implementations.
#[derive(Debug, Clone, Copy)]
pub struct MySQLRow<'row, R: ?Sized> {
    inner: &'row R,
}

impl<'row, R: MySQLRowAccess + ?Sized> MySQLRow<'row, R> {
    /// Wraps adapter-owned row storage without copying it.
    #[must_use]
    pub const fn new(inner: &'row R) -> Self {
        Self { inner }
    }

    fn value_at(&self, offset: usize) -> Result<MySQLValue<'_>, DrizzleError> {
        self.inner.value_at(offset)?.ok_or_else(|| {
            DrizzleError::ConversionError(
                format!("MySQL row has no available column at offset {offset}").into(),
            )
        })
    }

    fn decode_at<T: DecodeMySQLValue>(&self, offset: usize) -> Result<T, DrizzleError> {
        let value = self.value_at(offset)?;
        if self.inner.is_bit_at(offset) {
            T::decode_bit(value)
        } else {
            T::decode(value)
        }
    }

    /// Returns whether the cell at `offset` is SQL `NULL`.
    ///
    /// # Errors
    ///
    /// Returns an error when the offset is missing or the adapter cannot
    /// inspect the row.
    pub fn is_null_at(&self, offset: usize) -> Result<bool, DrizzleError> {
        Ok(self.value_at(offset)?.is_null())
    }
}

impl MySQLRowAccess for [OwnedMySQLValue] {
    fn value_at(&self, offset: usize) -> Result<Option<MySQLValue<'_>>, DrizzleError> {
        Ok(self.get(offset).map(MySQLValue::from))
    }
}

impl MySQLRowAccess for Vec<OwnedMySQLValue> {
    fn value_at(&self, offset: usize) -> Result<Option<MySQLValue<'_>>, DrizzleError> {
        self.as_slice().value_at(offset)
    }
}

impl MySQLRowAccess for [MySQLValue<'_>] {
    fn value_at(&self, offset: usize) -> Result<Option<MySQLValue<'_>>, DrizzleError> {
        Ok(self.get(offset).map(MySQLValue::as_ref))
    }
}

#[doc(hidden)]
pub trait DecodeMySQLValue: Sized {
    const EXPECTED: &'static str;

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError>;

    fn decode_bit(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        Self::decode(value)
    }
}

/// Decodes a binary value emitted by the MySQL relational JSON projection.
///
/// This is a macro/runtime seam, not an application-level codec API. The SQL
/// renderer tags binary values and carries their bytes as hexadecimal text so
/// MySQL's JSON constructors never reinterpret the binary character set.
#[cfg(feature = "query")]
#[doc(hidden)]
pub fn decode_projected_mysql_value<T: DecodeMySQLValue>(
    projected: &serde_json::Value,
) -> Result<T, DrizzleError> {
    let object = projected.as_object().ok_or_else(|| {
        DrizzleError::ConversionError("projected MySQL binary value is not an object".into())
    })?;
    if object
        .get("$drizzle_storage")
        .and_then(serde_json::Value::as_str)
        != Some("blob")
    {
        return Err(DrizzleError::ConversionError(
            "projected MySQL binary value has an invalid storage tag".into(),
        ));
    }
    let hex = object
        .get("$drizzle_value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            DrizzleError::ConversionError(
                "projected MySQL binary value is missing its hexadecimal payload".into(),
            )
        })?;
    if hex.len() % 2 != 0 {
        return Err(DrizzleError::ConversionError(
            "projected MySQL binary value has an odd-length hexadecimal payload".into(),
        ));
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for pair in hex.as_bytes().chunks(2) {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        bytes.push((high << 4) | low);
    }
    T::decode(MySQLValue::from(bytes))
}

/// Decodes a text scalar emitted by MySQL's JSON projection using the same
/// codec as a value returned by the binary protocol.
#[cfg(feature = "query")]
#[doc(hidden)]
pub fn decode_projected_mysql_text<T: DecodeMySQLValue>(
    projected: &serde_json::Value,
) -> Result<T, DrizzleError> {
    let text = projected.as_str().ok_or_else(|| {
        DrizzleError::ConversionError("projected MySQL text value is not a string".into())
    })?;
    T::decode(MySQLValue::from(text))
}

#[cfg(feature = "query")]
fn decode_hex_nibble(value: u8) -> Result<u8, DrizzleError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(DrizzleError::ConversionError(
            "projected MySQL binary value contains non-hexadecimal text".into(),
        )),
    }
}

fn conversion_error(expected: &str, value: &MySQLValue<'_>) -> DrizzleError {
    DrizzleError::ConversionError(
        format!(
            "expected {expected}, got MySQL {}",
            match value {
                MySQLValue::Null => "NULL",
                MySQLValue::Bytes(_) => "bytes",
                MySQLValue::Int(_) => "signed integer",
                MySQLValue::UInt(_) => "unsigned integer",
                MySQLValue::Float(_) => "float",
                MySQLValue::Double(_) => "double",
                MySQLValue::Date { .. } => "date/datetime",
                MySQLValue::Time { .. } => "time",
            }
        )
        .into(),
    )
}

fn utf8(value: &[u8]) -> Result<&str, DrizzleError> {
    core::str::from_utf8(value)
        .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
}

macro_rules! impl_integer_decode {
    ($($ty:ty),+ $(,)?) => {$(
        impl DecodeMySQLValue for $ty {
            const EXPECTED: &'static str = stringify!($ty);

            fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
                match value {
                    MySQLValue::Int(value) => Self::try_from(value).map_err(Into::into),
                    MySQLValue::UInt(value) => Self::try_from(value).map_err(Into::into),
                    MySQLValue::Bytes(value) => utf8(value.as_ref())?
                        .parse::<Self>()
                        .map_err(Into::into),
                    other => Err(conversion_error(Self::EXPECTED, &other)),
                }
            }

            fn decode_bit(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
                match value {
                    MySQLValue::Bytes(value) => Self::try_from(mysql_bit_to_u64(value.as_ref())?)
                        .map_err(Into::into),
                    other => Self::decode(other),
                }
            }
        }
    )+};
}

impl_integer_decode!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl DecodeMySQLValue for f32 {
    const EXPECTED: &'static str = "f32";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Float(value) => Ok(value),
            MySQLValue::Bytes(value) => utf8(value.as_ref())?.parse().map_err(Into::into),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

impl DecodeMySQLValue for f64 {
    const EXPECTED: &'static str = "f64";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Double(value) => Ok(value),
            MySQLValue::Float(value) => Ok(value.into()),
            MySQLValue::Bytes(value) => utf8(value.as_ref())?.parse().map_err(Into::into),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

impl DecodeMySQLValue for bool {
    const EXPECTED: &'static str = "bool";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Int(value) => Ok(value != 0),
            MySQLValue::UInt(value) => Ok(value != 0),
            MySQLValue::Bytes(value) if value.as_ref() == b"0" => Ok(false),
            MySQLValue::Bytes(value) if value.as_ref() == b"1" => Ok(true),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }

    fn decode_bit(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(value) => Ok(mysql_bit_to_u64(value.as_ref())? != 0),
            other => Self::decode(other),
        }
    }
}

impl DecodeMySQLValue for String {
    const EXPECTED: &'static str = "UTF-8 text";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(value) => Ok(String::from(utf8(value.as_ref())?)),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

impl DecodeMySQLValue for char {
    const EXPECTED: &'static str = "one Unicode scalar";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let text = String::decode(value)?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(value), None) => Ok(value),
            _ => Err(DrizzleError::ConversionError(
                "expected exactly one Unicode scalar".into(),
            )),
        }
    }
}

impl DecodeMySQLValue for Vec<u8> {
    const EXPECTED: &'static str = "bytes";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(value) => Ok(value.into_owned()),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

impl<const N: usize> DecodeMySQLValue for [u8; N] {
    const EXPECTED: &'static str = "fixed-size byte array";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let value = Vec::<u8>::decode(value)?;
        value.try_into().map_err(|value: Vec<u8>| {
            DrizzleError::ConversionError(
                format!("expected {N} bytes, got {} bytes", value.len(),).into(),
            )
        })
    }
}

impl<const N: usize> DecodeMySQLValue for [char; N] {
    const EXPECTED: &'static str = "fixed-size character array";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let value = String::decode(value)?;
        let chars: Vec<char> = value.chars().collect();
        chars.try_into().map_err(|chars: Vec<char>| {
            DrizzleError::ConversionError(
                format!("expected {N} characters, got {} characters", chars.len(),).into(),
            )
        })
    }
}

macro_rules! impl_row_decode {
    ($($ty:ty),+ $(,)?) => {$(
        impl<'row, R: MySQLRowAccess + ?Sized> FromDrizzleRow<MySQLRow<'row, R>> for $ty {
            const COLUMN_COUNT: usize = 1;

            fn from_row_at(
                row: &MySQLRow<'row, R>,
                offset: usize,
            ) -> Result<Self, DrizzleError> {
                row.decode_at::<$ty>(offset)
            }
        }
    )+};
}

impl_row_decode!(
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
    char,
    String,
    Vec<u8>,
);

impl<'row, R: MySQLRowAccess + ?Sized, const N: usize> FromDrizzleRow<MySQLRow<'row, R>>
    for [u8; N]
{
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        row.decode_at::<Self>(offset)
    }
}

impl<'row, R: MySQLRowAccess + ?Sized, const N: usize> FromDrizzleRow<MySQLRow<'row, R>>
    for [char; N]
{
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        row.decode_at::<Self>(offset)
    }
}

impl<'row, R, T> FromDrizzleRow<MySQLRow<'row, R>> for Option<T>
where
    R: MySQLRowAccess + ?Sized,
    T: FromDrizzleRow<MySQLRow<'row, R>>,
{
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        if row.is_null_at(offset)? {
            Ok(None)
        } else {
            T::from_row_at(row, offset).map(Some)
        }
    }
}

#[cfg(feature = "compact-str")]
impl DecodeMySQLValue for compact_str::CompactString {
    const EXPECTED: &'static str = "UTF-8 text";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        String::decode(value).map(Into::into)
    }
}

#[cfg(feature = "compact-str")]
impl_row_decode!(compact_str::CompactString);

#[cfg(feature = "arrayvec")]
impl<const N: usize> DecodeMySQLValue for arrayvec::ArrayString<N> {
    const EXPECTED: &'static str = "bounded UTF-8 text";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let value = String::decode(value)?;
        Self::try_from(value.as_str())
            .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'row, R: MySQLRowAccess + ?Sized, const N: usize> FromDrizzleRow<MySQLRow<'row, R>>
    for arrayvec::ArrayString<N>
{
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        row.decode_at::<Self>(offset)
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> DecodeMySQLValue for arrayvec::ArrayVec<u8, N> {
    const EXPECTED: &'static str = "bounded bytes";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let value = Vec::<u8>::decode(value)?;
        Self::try_from(value.as_slice())
            .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'row, R: MySQLRowAccess + ?Sized, const N: usize> FromDrizzleRow<MySQLRow<'row, R>>
    for arrayvec::ArrayVec<u8, N>
{
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        row.decode_at::<Self>(offset)
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> DecodeMySQLValue for smallvec::SmallVec<[u8; N]> {
    const EXPECTED: &'static str = "bytes";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        Ok(Vec::<u8>::decode(value)?.into())
    }
}

#[cfg(feature = "smallvec")]
impl<'row, R: MySQLRowAccess + ?Sized, const N: usize> FromDrizzleRow<MySQLRow<'row, R>>
    for smallvec::SmallVec<[u8; N]>
{
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        row.decode_at::<Self>(offset)
    }
}

#[cfg(feature = "bytes")]
impl DecodeMySQLValue for bytes::Bytes {
    const EXPECTED: &'static str = "bytes";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        Vec::<u8>::decode(value).map(Into::into)
    }
}

#[cfg(feature = "bytes")]
impl DecodeMySQLValue for bytes::BytesMut {
    const EXPECTED: &'static str = "bytes";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        Vec::<u8>::decode(value).map(|value| Self::from(value.as_slice()))
    }
}

#[cfg(feature = "bytes")]
impl_row_decode!(bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "serde")]
impl DecodeMySQLValue for serde_json::Value {
    const EXPECTED: &'static str = "JSON text";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(value) => serde_json::from_slice(value.as_ref()).map_err(Into::into),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "serde")]
impl_row_decode!(serde_json::Value);

#[cfg(feature = "uuid")]
impl DecodeMySQLValue for uuid::Uuid {
    const EXPECTED: &'static str = "16-byte or textual UUID";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(value) if value.len() == 16 => {
                Self::from_slice(value.as_ref()).map_err(Into::into)
            }
            MySQLValue::Bytes(value) => Self::parse_str(utf8(value.as_ref())?).map_err(Into::into),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "uuid")]
impl_row_decode!(uuid::Uuid);

#[cfg(feature = "rust-decimal")]
impl DecodeMySQLValue for rust_decimal::Decimal {
    const EXPECTED: &'static str = "decimal text within rust_decimal range";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(value) => {
                utf8(value.as_ref())?
                    .parse()
                    .map_err(|error: rust_decimal::Error| {
                        DrizzleError::ConversionError(error.to_string().into())
                    })
            }
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "rust-decimal")]
impl_row_decode!(rust_decimal::Decimal);

#[cfg(feature = "chrono")]
fn chrono_date(year: u16, month: u8, day: u8) -> Result<chrono::NaiveDate, DrizzleError> {
    chrono::NaiveDate::from_ymd_opt(i32::from(year), u32::from(month), u32::from(day)).ok_or_else(
        || {
            DrizzleError::ConversionError(
                format!("invalid MySQL date {year:04}-{month:02}-{day:02}").into(),
            )
        },
    )
}

#[cfg(feature = "chrono")]
fn chrono_time(
    negative: bool,
    days: u32,
    hours: u8,
    minutes: u8,
    seconds: u8,
    microseconds: u32,
) -> Result<chrono::NaiveTime, DrizzleError> {
    if negative || days != 0 {
        return Err(DrizzleError::ConversionError(
            "MySQL duration outside a single non-negative day cannot decode as chrono::NaiveTime"
                .into(),
        ));
    }
    chrono::NaiveTime::from_hms_micro_opt(
        u32::from(hours),
        u32::from(minutes),
        u32::from(seconds),
        microseconds,
    )
    .ok_or_else(|| DrizzleError::ConversionError("invalid MySQL time".into()))
}

#[cfg(feature = "chrono")]
impl DecodeMySQLValue for chrono::NaiveDate {
    const EXPECTED: &'static str = "MySQL DATE";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Date {
                year, month, day, ..
            } => chrono_date(year, month, day),
            MySQLValue::Bytes(value) => {
                utf8(value.as_ref())?
                    .parse()
                    .map_err(|error: chrono::ParseError| {
                        DrizzleError::ConversionError(error.to_string().into())
                    })
            }
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "chrono")]
impl DecodeMySQLValue for chrono::NaiveTime {
    const EXPECTED: &'static str = "MySQL TIME within one day";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => chrono_time(negative, days, hours, minutes, seconds, microseconds),
            MySQLValue::Bytes(value) => {
                utf8(value.as_ref())?
                    .parse()
                    .map_err(|error: chrono::ParseError| {
                        DrizzleError::ConversionError(error.to_string().into())
                    })
            }
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "chrono")]
impl DecodeMySQLValue for chrono::NaiveDateTime {
    const EXPECTED: &'static str = "MySQL DATETIME/TIMESTAMP";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => chrono_date(year, month, day)?
                .and_hms_micro_opt(
                    u32::from(hour),
                    u32::from(minute),
                    u32::from(second),
                    microseconds,
                )
                .ok_or_else(|| DrizzleError::ConversionError("invalid MySQL datetime".into())),
            MySQLValue::Bytes(value) => {
                let value = utf8(value.as_ref())?;
                chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f")
                    .or_else(|_| value.parse())
                    .map_err(|error: chrono::ParseError| {
                        DrizzleError::ConversionError(error.to_string().into())
                    })
            }
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

fn mysql_bit_to_u64(value: &[u8]) -> Result<u64, DrizzleError> {
    if value.len() > core::mem::size_of::<u64>() {
        return Err(DrizzleError::ConversionError(
            format!("MySQL BIT value exceeds 64 bits: {} bytes", value.len()).into(),
        ));
    }
    Ok(value
        .iter()
        .fold(0_u64, |result, byte| (result << 8) | u64::from(*byte)))
}

#[cfg(feature = "chrono")]
impl DecodeMySQLValue for chrono::DateTime<chrono::Utc> {
    const EXPECTED: &'static str = "UTC MySQL TIMESTAMP";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        chrono::NaiveDateTime::decode(value)
            .map(|value| Self::from_naive_utc_and_offset(value, chrono::Utc))
    }
}

#[cfg(feature = "chrono")]
impl DecodeMySQLValue for chrono::DateTime<chrono::FixedOffset> {
    const EXPECTED: &'static str = "UTC MySQL TIMESTAMP";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        let offset = chrono::FixedOffset::east_opt(0)
            .ok_or_else(|| DrizzleError::ConversionError("invalid UTC offset".into()))?;
        chrono::NaiveDateTime::decode(value)
            .map(|value| Self::from_naive_utc_and_offset(value, offset))
    }
}

#[cfg(feature = "chrono")]
impl_row_decode!(
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::FixedOffset>,
);

#[cfg(feature = "time")]
fn time_date(year: u16, month: u8, day: u8) -> Result<time::Date, DrizzleError> {
    let month = time::Month::try_from(month)
        .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))?;
    time::Date::from_calendar_date(i32::from(year), month, day)
        .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
}

#[cfg(feature = "time")]
fn time_time(
    negative: bool,
    days: u32,
    hours: u8,
    minutes: u8,
    seconds: u8,
    microseconds: u32,
) -> Result<time::Time, DrizzleError> {
    if negative || days != 0 {
        return Err(DrizzleError::ConversionError(
            "MySQL duration outside a single non-negative day cannot decode as time::Time".into(),
        ));
    }
    time::Time::from_hms_micro(hours, minutes, seconds, microseconds)
        .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
}

#[cfg(feature = "time")]
impl DecodeMySQLValue for time::Date {
    const EXPECTED: &'static str = "MySQL DATE";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Date {
                year, month, day, ..
            } => time_date(year, month, day),
            MySQLValue::Bytes(value) => time::Date::parse(
                utf8(value.as_ref())?,
                &time::format_description::well_known::Iso8601::DATE,
            )
            .map_err(|error| DrizzleError::ConversionError(error.to_string().into())),
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "time")]
impl DecodeMySQLValue for time::Time {
    const EXPECTED: &'static str = "MySQL TIME within one day";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => time_time(negative, days, hours, minutes, seconds, microseconds),
            MySQLValue::Bytes(value) => {
                let text = utf8(value.as_ref())?;
                let description =
                    time::macros::format_description!("[hour]:[minute]:[second].[subsecond]");
                time::Time::parse(text, description)
                    .or_else(|_| {
                        time::Time::parse(
                            text,
                            time::macros::format_description!("[hour]:[minute]:[second]"),
                        )
                    })
                    .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
            }
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "time")]
impl DecodeMySQLValue for time::PrimitiveDateTime {
    const EXPECTED: &'static str = "MySQL DATETIME/TIMESTAMP";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                microseconds,
            } => Ok(Self::new(
                time_date(year, month, day)?,
                time::Time::from_hms_micro(hour, minute, second, microseconds)
                    .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))?,
            )),
            MySQLValue::Bytes(value) => {
                let text = utf8(value.as_ref())?;
                Self::parse(
                    text,
                    time::macros::format_description!(
                        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
                    ),
                )
                .or_else(|_| {
                    Self::parse(
                        text,
                        time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]"
                        ),
                    )
                })
                .map_err(|error| DrizzleError::ConversionError(error.to_string().into()))
            }
            other => Err(conversion_error(Self::EXPECTED, &other)),
        }
    }
}

#[cfg(feature = "time")]
impl DecodeMySQLValue for time::OffsetDateTime {
    const EXPECTED: &'static str = "UTC MySQL TIMESTAMP";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        time::PrimitiveDateTime::decode(value).map(|value| value.assume_utc())
    }
}

#[cfg(feature = "time")]
impl_row_decode!(
    time::Date,
    time::Time,
    time::PrimitiveDateTime,
    time::OffsetDateTime,
);

#[cfg(feature = "mysql-common")]
impl MySQLRowAccess for mysql_common::Row {
    fn value_at(&self, offset: usize) -> Result<Option<MySQLValue<'_>>, DrizzleError> {
        let Some(value) = self.as_ref(offset) else {
            return Ok(None);
        };
        Ok(Some(match value {
            mysql_common::Value::NULL => MySQLValue::Null,
            mysql_common::Value::Bytes(value) => MySQLValue::from(value.as_slice()),
            mysql_common::Value::Int(value) => MySQLValue::Int(*value),
            mysql_common::Value::UInt(value) => MySQLValue::UInt(*value),
            mysql_common::Value::Float(value) => MySQLValue::Float(*value),
            mysql_common::Value::Double(value) => MySQLValue::Double(*value),
            mysql_common::Value::Date(year, month, day, hour, minute, second, micros) => {
                MySQLValue::Date {
                    year: *year,
                    month: *month,
                    day: *day,
                    hour: *hour,
                    minute: *minute,
                    second: *second,
                    microseconds: *micros,
                }
            }
            mysql_common::Value::Time(negative, days, hours, minutes, seconds, micros) => {
                MySQLValue::Time {
                    negative: *negative,
                    days: *days,
                    hours: *hours,
                    minutes: *minutes,
                    seconds: *seconds,
                    microseconds: *micros,
                }
            }
        }))
    }

    fn is_bit_at(&self, offset: usize) -> bool {
        self.columns_ref().get(offset).is_some_and(|column| {
            column.column_type() == mysql_common::constants::ColumnType::MYSQL_TYPE_BIT
        })
    }
}

#[cfg(feature = "mysql-common")]
impl From<MySQLValue<'_>> for mysql_common::Value {
    fn from(value: MySQLValue<'_>) -> Self {
        match value {
            MySQLValue::Null => Self::NULL,
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
            } => Self::Date(year, month, day, hour, minute, second, microseconds),
            MySQLValue::Time {
                negative,
                days,
                hours,
                minutes,
                seconds,
                microseconds,
            } => Self::Time(negative, days, hours, minutes, seconds, microseconds),
        }
    }
}

#[cfg(feature = "mysql-common")]
impl<'a> MySQLPreparedRequest<'a> {
    /// Converts this request to the shared protocol representation used by
    /// both `mysql` and `mysql_async`.
    #[must_use]
    pub fn into_common_parts(self) -> (&'a str, mysql_common::params::Params) {
        let values: Vec<mysql_common::Value> = self.params.into_iter().map(Into::into).collect();
        (self.sql, values.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode<T: for<'row> FromDrizzleRow<MySQLRow<'row, [OwnedMySQLValue]>>>(
        values: &[OwnedMySQLValue],
    ) -> Result<T, DrizzleError> {
        T::from_row(&MySQLRow::new(values))
    }

    #[test]
    fn positional_request_preserves_order_without_tuple_arity() {
        let params = (0_u64..32).map(MySQLValue::UInt);
        let request = MySQLPreparedRequest::new("SELECT ?", params);
        assert_eq!(request.params().len(), 32);
        assert_eq!(request.params()[0], MySQLValue::UInt(0));
        assert_eq!(request.params()[31], MySQLValue::UInt(31));
    }

    #[test]
    fn signed_unsigned_null_and_offsets_decode_without_panics() {
        let values = [
            OwnedMySQLValue::Int(-1),
            OwnedMySQLValue::UInt(u64::MAX),
            OwnedMySQLValue::Null,
        ];
        assert_eq!(
            decode::<(i64, u64, Option<i32>)>(&values).unwrap(),
            (-1, u64::MAX, None)
        );
        assert!(decode::<(u64,)>(&values).is_err());
        assert!(decode::<(i64, u64, i32)>(&values).is_err());
        assert!(decode::<(i64, u64, Option<i32>, i32)>(&values).is_err());
    }

    #[test]
    fn text_protocol_numbers_are_checked_for_overflow() {
        let valid = [OwnedMySQLValue::Bytes(b"255".to_vec())];
        assert_eq!(decode::<u8>(&valid).unwrap(), 255);
        let overflow = [OwnedMySQLValue::Bytes(b"256".to_vec())];
        assert!(decode::<u8>(&overflow).is_err());
        let negative = [OwnedMySQLValue::Bytes(b"-1".to_vec())];
        assert!(decode::<u64>(&negative).is_err());
    }

    #[test]
    fn invalid_utf8_is_a_conversion_error() {
        let values = [OwnedMySQLValue::Bytes(vec![0xff])];
        assert!(decode::<String>(&values).is_err());
    }

    #[cfg(feature = "query")]
    #[test]
    fn projected_binary_values_follow_the_checked_wire_codec() {
        let tagged = serde_json::json!({
            "$drizzle_storage": "blob",
            "$drizzle_value": "0001FEFF"
        });
        assert_eq!(
            decode_projected_mysql_value::<Vec<u8>>(&tagged).unwrap(),
            [0, 1, 254, 255]
        );
        assert!(
            decode_projected_mysql_value::<Vec<u8>>(&serde_json::json!({
                "$drizzle_storage": "text",
                "$drizzle_value": "00"
            }))
            .is_err()
        );
        assert!(
            decode_projected_mysql_value::<Vec<u8>>(&serde_json::json!({
                "$drizzle_storage": "blob",
                "$drizzle_value": "0"
            }))
            .is_err()
        );
    }

    #[cfg(all(
        feature = "arrayvec",
        feature = "bytes",
        feature = "compact-str",
        feature = "smallvec"
    ))]
    #[test]
    fn feature_forwarded_text_and_binary_types_decode_through_the_same_row_contract() {
        let text = [OwnedMySQLValue::Bytes(b"drizzle".to_vec())];
        assert_eq!(
            decode::<compact_str::CompactString>(&text).unwrap(),
            "drizzle"
        );
        assert_eq!(
            decode::<arrayvec::ArrayString<8>>(&text).unwrap().as_str(),
            "drizzle"
        );
        assert!(decode::<arrayvec::ArrayString<3>>(&text).is_err());

        let binary = [OwnedMySQLValue::Bytes(vec![1, 2, 3])];
        assert_eq!(
            decode::<arrayvec::ArrayVec<u8, 3>>(&binary)
                .unwrap()
                .as_slice(),
            &[1, 2, 3]
        );
        assert!(decode::<arrayvec::ArrayVec<u8, 2>>(&binary).is_err());
        assert_eq!(
            decode::<smallvec::SmallVec<[u8; 2]>>(&binary)
                .unwrap()
                .as_slice(),
            &[1, 2, 3]
        );
        assert_eq!(
            decode::<bytes::Bytes>(&binary).unwrap().as_ref(),
            &[1, 2, 3]
        );
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn uuid_accepts_binary_and_text_forms_but_rejects_malformed_bytes() {
        let expected = uuid::Uuid::from_u128(0x12345678_90ab_cdef_1234_567890abcdef);
        assert_eq!(
            decode::<uuid::Uuid>(&[OwnedMySQLValue::Bytes(expected.as_bytes().to_vec())]).unwrap(),
            expected
        );
        assert_eq!(
            decode::<uuid::Uuid>(&[OwnedMySQLValue::Bytes(expected.to_string().into_bytes())])
                .unwrap(),
            expected
        );
        assert!(decode::<uuid::Uuid>(&[OwnedMySQLValue::Bytes(vec![1, 2, 3])]).is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_decode_reports_invalid_payloads() {
        let value =
            decode::<serde_json::Value>(&[OwnedMySQLValue::Bytes(br#"{"ok":true}"#.to_vec())])
                .unwrap();
        assert_eq!(value["ok"], true);
        assert!(decode::<serde_json::Value>(&[OwnedMySQLValue::Bytes(b"{".to_vec())]).is_err());
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_round_trips_binary_dates_and_rejects_zero_dates() {
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 8, 26)
            .unwrap()
            .and_hms_micro_opt(12, 34, 56, 789)
            .unwrap();
        assert_eq!(
            decode::<chrono::NaiveDateTime>(&[OwnedMySQLValue::from(expected)]).unwrap(),
            expected
        );
        assert_eq!(
            decode::<chrono::NaiveDateTime>(&[OwnedMySQLValue::Bytes(
                b"2026-08-26 12:34:56.000789".to_vec(),
            )])
            .unwrap(),
            expected
        );
        assert!(
            decode::<chrono::NaiveDate>(&[OwnedMySQLValue::Date {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                microseconds: 0,
            }])
            .is_err()
        );
    }

    #[cfg(feature = "time")]
    #[test]
    fn time_round_trips_binary_dates_and_rejects_duration_as_clock_time() {
        let expected = time::Date::from_calendar_date(2026, time::Month::August, 26)
            .unwrap()
            .with_hms_micro(12, 34, 56, 789)
            .unwrap();
        assert_eq!(
            decode::<time::PrimitiveDateTime>(&[OwnedMySQLValue::from(expected)]).unwrap(),
            expected
        );
        assert!(
            decode::<time::Time>(&[OwnedMySQLValue::Time {
                negative: false,
                days: 1,
                hours: 0,
                minutes: 0,
                seconds: 0,
                microseconds: 0,
            }])
            .is_err()
        );
    }

    #[cfg(feature = "rust-decimal")]
    #[test]
    fn decimal_boundaries_fail_instead_of_truncating() {
        let max = [OwnedMySQLValue::Bytes(
            b"79228162514264337593543950335".to_vec(),
        )];
        assert_eq!(
            decode::<rust_decimal::Decimal>(&max).unwrap(),
            rust_decimal::Decimal::MAX
        );
        let overflow = [OwnedMySQLValue::Bytes(
            b"79228162514264337593543950336".to_vec(),
        )];
        assert!(decode::<rust_decimal::Decimal>(&overflow).is_err());
    }

    #[cfg(feature = "mysql-common")]
    #[test]
    fn prepared_request_uses_shared_positional_params() {
        let request = MySQLPreparedRequest::new(
            "SELECT ?, ?",
            [MySQLValue::Int(-1), MySQLValue::UInt(u64::MAX)],
        );
        let (sql, params) = request.into_common_parts();
        assert_eq!(sql, "SELECT ?, ?");
        assert_eq!(
            params,
            mysql_common::params::Params::Positional(vec![
                mysql_common::Value::Int(-1),
                mysql_common::Value::UInt(u64::MAX),
            ])
        );

        let (_, empty) = MySQLPreparedRequest::new("SELECT 1", []).into_common_parts();
        assert_eq!(empty, mysql_common::params::Params::Empty);
    }

    #[cfg(feature = "mysql-common")]
    #[test]
    fn mysql_bit_bytes_decode_as_big_endian_unsigned_values() {
        assert_eq!(mysql_bit_to_u64(&[]).unwrap(), 0);
        assert_eq!(mysql_bit_to_u64(&[0xff]).unwrap(), 255);
        assert_eq!(mysql_bit_to_u64(&[0x01, 0x00]).unwrap(), 256);
        assert_eq!(mysql_bit_to_u64(&[0xff; 8]).unwrap(), u64::MAX);
        assert!(mysql_bit_to_u64(&[0; 9]).is_err());
    }

    #[test]
    fn bit_metadata_preserves_binary_targets_and_enables_numeric_targets() {
        struct PackedBitRow(OwnedMySQLValue);

        impl MySQLRowAccess for PackedBitRow {
            fn value_at(&self, offset: usize) -> Result<Option<MySQLValue<'_>>, DrizzleError> {
                Ok((offset == 0).then(|| MySQLValue::from(&self.0)))
            }

            fn is_bit_at(&self, offset: usize) -> bool {
                offset == 0
            }
        }

        let row = PackedBitRow(OwnedMySQLValue::Bytes(vec![0xa5]));
        let row = MySQLRow::new(&row);
        assert_eq!(u64::from_row(&row).unwrap(), 0xa5);
        assert!(bool::from_row(&row).unwrap());
        assert_eq!(Vec::<u8>::from_row(&row).unwrap(), [0xa5]);
    }
}
