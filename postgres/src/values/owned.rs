//! Owned PostgreSQL value types for static lifetime scenarios

use crate::PostgresValue;
use drizzle_core::{SQLParam, error::DrizzleError};
use std::borrow::Cow;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};

#[cfg(feature = "rust_decimal")]
use rust_decimal::Decimal;

#[cfg(feature = "ipnet")]
use ipnet::IpNet;

#[cfg(feature = "geo-types")]
use geo_types::{LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

#[cfg(feature = "bitvec")]
use bitvec::prelude::*;

/// Owned version of PostgresValue that doesn't borrow data
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedPostgresValue {
    /// SMALLINT values (16-bit signed integer)
    Smallint(i16),
    /// INTEGER values (32-bit signed integer)
    Integer(i32),
    /// BIGINT values (64-bit signed integer)
    Bigint(i64),
    /// REAL values (32-bit floating point)
    Real(f32),
    /// DOUBLE PRECISION values (64-bit floating point)
    DoublePrecision(f64),
    /// TEXT, VARCHAR, CHAR values (owned)
    Text(String),
    /// BYTEA values (owned binary data)
    Bytea(Vec<u8>),
    /// BOOLEAN values
    Boolean(bool),
    /// UUID values
    #[cfg(feature = "uuid")]
    Uuid(Uuid),
    /// JSON/JSONB values
    #[cfg(feature = "serde")]
    Json(serde_json::Value),

    // Date and time types
    /// DATE values
    #[cfg(feature = "chrono")]
    Date(NaiveDate),
    /// TIME values
    #[cfg(feature = "chrono")]
    Time(NaiveTime),
    /// TIMESTAMP values (without timezone)
    #[cfg(feature = "chrono")]
    Timestamp(NaiveDateTime),
    /// TIMESTAMPTZ values (with timezone)
    #[cfg(feature = "chrono")]
    TimestampTz(DateTime<FixedOffset>),
    /// INTERVAL values
    #[cfg(feature = "chrono")]
    Interval(Duration),

    // Numeric types
    /// NUMERIC, DECIMAL values (arbitrary precision)
    #[cfg(feature = "rust_decimal")]
    Decimal(Decimal),

    // Network address types
    /// INET values (IPv4 or IPv6 networks)
    #[cfg(feature = "ipnet")]
    Inet(IpNet),
    /// CIDR values (IPv4 or IPv6 networks)
    #[cfg(feature = "ipnet")]
    Cidr(IpNet),
    /// MACADDR values (MAC addresses)
    #[cfg(feature = "ipnet")]
    MacAddr([u8; 6]),
    /// MACADDR8 values (EUI-64 MAC addresses)
    #[cfg(feature = "ipnet")]
    MacAddr8([u8; 8]),

    // Geometric types
    /// POINT values
    #[cfg(feature = "geo-types")]
    Point(Point<f64>),
    /// LINESTRING values
    #[cfg(feature = "geo-types")]
    LineString(LineString<f64>),
    /// POLYGON values
    #[cfg(feature = "geo-types")]
    Polygon(Polygon<f64>),
    /// MULTIPOINT values
    #[cfg(feature = "geo-types")]
    MultiPoint(MultiPoint<f64>),
    /// MULTILINESTRING values
    #[cfg(feature = "geo-types")]
    MultiLineString(MultiLineString<f64>),
    /// MULTIPOLYGON values
    #[cfg(feature = "geo-types")]
    MultiPolygon(MultiPolygon<f64>),

    // Bit string types
    /// BIT, BIT VARYING values
    #[cfg(feature = "bitvec")]
    BitVec(BitVec),

    // Array types (using Vec for simplicity)
    /// Array of any PostgreSQL type
    Array(Vec<OwnedPostgresValue>),

    /// NULL value
    Null,
}

impl SQLParam for OwnedPostgresValue {}

impl Default for OwnedPostgresValue {
    fn default() -> Self {
        OwnedPostgresValue::Null
    }
}

impl std::fmt::Display for OwnedPostgresValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            OwnedPostgresValue::Smallint(i) => i.to_string(),
            OwnedPostgresValue::Integer(i) => i.to_string(),
            OwnedPostgresValue::Bigint(i) => i.to_string(),
            OwnedPostgresValue::Real(r) => r.to_string(),
            OwnedPostgresValue::DoublePrecision(r) => r.to_string(),
            OwnedPostgresValue::Text(s) => s.clone(),
            OwnedPostgresValue::Bytea(b) => format!(
                "\\x{}",
                b.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            ),
            OwnedPostgresValue::Boolean(b) => b.to_string(),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(uuid) => uuid.to_string(),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(json) => json.to_string(),

            // Date and time types
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Date(date) => date.format("%Y-%m-%d").to_string(),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Time(time) => time.format("%H:%M:%S%.f").to_string(),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Timestamp(ts) => ts.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::TimestampTz(ts) => {
                ts.format("%Y-%m-%d %H:%M:%S%.f %:z").to_string()
            }
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Interval(dur) => format!("{} seconds", dur.num_seconds()),

            // Numeric types
            #[cfg(feature = "rust_decimal")]
            OwnedPostgresValue::Decimal(dec) => dec.to_string(),

            // Network address types
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::Inet(net) => net.to_string(),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::Cidr(net) => net.to_string(),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::MacAddr(mac) => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            ),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::MacAddr8(mac) => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[6], mac[7]
            ),

            // Geometric types
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(point) => format!("({},{})", point.x(), point.y()),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(line) => {
                let coords: Vec<String> = line
                    .coords()
                    .map(|coord| format!("({},{})", coord.x, coord.y))
                    .collect();
                format!("[{}]", coords.join(","))
            }
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Polygon(poly) => format!(
                "POLYGON({})",
                poly.exterior()
                    .coords()
                    .map(|c| format!("({},{})", c.x, c.y))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::MultiPoint(mp) => format!(
                "MULTIPOINT({})",
                mp.iter()
                    .map(|p| format!("({},{})", p.x(), p.y()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::MultiLineString(mls) => format!(
                "MULTILINESTRING({})",
                mls.iter()
                    .map(|ls| format!(
                        "[{}]",
                        ls.coords()
                            .map(|c| format!("({},{})", c.x, c.y))
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::MultiPolygon(mp) => format!(
                "MULTIPOLYGON({})",
                mp.iter()
                    .map(|p| format!(
                        "POLYGON({})",
                        p.exterior()
                            .coords()
                            .map(|c| format!("({},{})", c.x, c.y))
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),

            // Bit string types
            #[cfg(feature = "bitvec")]
            OwnedPostgresValue::BitVec(bv) => bv
                .iter()
                .map(|b| if *b { '1' } else { '0' })
                .collect::<String>(),

            // Array types
            OwnedPostgresValue::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("{{{}}}", elements.join(","))
            }

            OwnedPostgresValue::Null => String::new(),
        };
        write!(f, "{value}")
    }
}

// Conversions from PostgresValue to OwnedPostgresValue
impl<'a> From<PostgresValue<'a>> for OwnedPostgresValue {
    fn from(value: PostgresValue<'a>) -> Self {
        match value {
            PostgresValue::Smallint(i) => OwnedPostgresValue::Smallint(i),
            PostgresValue::Integer(i) => OwnedPostgresValue::Integer(i),
            PostgresValue::Bigint(i) => OwnedPostgresValue::Bigint(i),
            PostgresValue::Real(r) => OwnedPostgresValue::Real(r),
            PostgresValue::DoublePrecision(r) => OwnedPostgresValue::DoublePrecision(r),
            PostgresValue::Text(cow) => OwnedPostgresValue::Text(cow.into_owned()),
            PostgresValue::Bytea(cow) => OwnedPostgresValue::Bytea(cow.into_owned()),
            PostgresValue::Boolean(b) => OwnedPostgresValue::Boolean(b),
            #[cfg(feature = "uuid")]
            PostgresValue::Uuid(uuid) => OwnedPostgresValue::Uuid(uuid),
            #[cfg(feature = "serde")]
            PostgresValue::Json(json) => OwnedPostgresValue::Json(json),
            PostgresValue::Enum(enum_val) => {
                OwnedPostgresValue::Text(enum_val.variant_name().to_string())
            }
            PostgresValue::Null => OwnedPostgresValue::Null,
            #[cfg(feature = "chrono")]
            PostgresValue::Date(date) => OwnedPostgresValue::Date(date),
            #[cfg(feature = "chrono")]
            PostgresValue::Time(time) => OwnedPostgresValue::Time(time),
            #[cfg(feature = "chrono")]
            PostgresValue::Timestamp(ts) => OwnedPostgresValue::Timestamp(ts),
            #[cfg(feature = "chrono")]
            PostgresValue::TimestampTz(ts) => OwnedPostgresValue::TimestampTz(ts),
            #[cfg(feature = "chrono")]
            PostgresValue::Interval(dur) => OwnedPostgresValue::Interval(dur),
            #[cfg(feature = "rust_decimal")]
            PostgresValue::Decimal(dec) => OwnedPostgresValue::Decimal(dec),
            #[cfg(feature = "ipnet")]
            PostgresValue::Inet(net) => OwnedPostgresValue::Inet(net),
            #[cfg(feature = "ipnet")]
            PostgresValue::Cidr(net) => OwnedPostgresValue::Cidr(net),
            #[cfg(feature = "ipnet")]
            PostgresValue::MacAddr(mac) => OwnedPostgresValue::MacAddr(mac),
            #[cfg(feature = "ipnet")]
            PostgresValue::MacAddr8(mac) => OwnedPostgresValue::MacAddr8(mac),
            #[cfg(feature = "geo-types")]
            PostgresValue::Point(point) => OwnedPostgresValue::Point(point),
            #[cfg(feature = "geo-types")]
            PostgresValue::LineString(line) => OwnedPostgresValue::LineString(line),
            #[cfg(feature = "geo-types")]
            PostgresValue::Polygon(poly) => OwnedPostgresValue::Polygon(poly),
            #[cfg(feature = "geo-types")]
            PostgresValue::MultiPoint(mp) => OwnedPostgresValue::MultiPoint(mp),
            #[cfg(feature = "geo-types")]
            PostgresValue::MultiLineString(mls) => OwnedPostgresValue::MultiLineString(mls),
            #[cfg(feature = "geo-types")]
            PostgresValue::MultiPolygon(mp) => OwnedPostgresValue::MultiPolygon(mp),
            #[cfg(feature = "bitvec")]
            PostgresValue::BitVec(bv) => OwnedPostgresValue::BitVec(bv),
            PostgresValue::Array(arr) => {
                let owned_arr = arr
                    .into_iter()
                    .map(|v| OwnedPostgresValue::from(v))
                    .collect();
                OwnedPostgresValue::Array(owned_arr)
            }
        }
    }
}

// Conversions from OwnedPostgresValue to PostgresValue
impl<'a> From<OwnedPostgresValue> for PostgresValue<'a> {
    fn from(value: OwnedPostgresValue) -> Self {
        match value {
            OwnedPostgresValue::Smallint(i) => PostgresValue::Smallint(i),
            OwnedPostgresValue::Integer(i) => PostgresValue::Integer(i),
            OwnedPostgresValue::Bigint(i) => PostgresValue::Bigint(i),
            OwnedPostgresValue::Real(r) => PostgresValue::Real(r),
            OwnedPostgresValue::DoublePrecision(r) => PostgresValue::DoublePrecision(r),
            OwnedPostgresValue::Text(s) => PostgresValue::Text(Cow::Owned(s)),
            OwnedPostgresValue::Bytea(b) => PostgresValue::Bytea(Cow::Owned(b)),
            OwnedPostgresValue::Boolean(b) => PostgresValue::Boolean(b),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(uuid) => PostgresValue::Uuid(uuid),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(json) => PostgresValue::Json(json),

            // Date and time types
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Date(date) => PostgresValue::Date(date),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Time(time) => PostgresValue::Time(time),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Timestamp(ts) => PostgresValue::Timestamp(ts),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::TimestampTz(ts) => PostgresValue::TimestampTz(ts),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Interval(dur) => PostgresValue::Interval(dur),
            #[cfg(feature = "rust_decimal")]
            OwnedPostgresValue::Decimal(dec) => PostgresValue::Decimal(dec),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::Inet(net) => PostgresValue::Inet(net),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::Cidr(net) => PostgresValue::Cidr(net),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::MacAddr(mac) => PostgresValue::MacAddr(mac),
            #[cfg(feature = "ipnet")]
            OwnedPostgresValue::MacAddr8(mac) => PostgresValue::MacAddr8(mac),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(point) => PostgresValue::Point(point),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(line) => PostgresValue::LineString(line),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Polygon(poly) => PostgresValue::Polygon(poly),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::MultiPoint(mp) => PostgresValue::MultiPoint(mp),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::MultiLineString(mls) => PostgresValue::MultiLineString(mls),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::MultiPolygon(mp) => PostgresValue::MultiPolygon(mp),
            #[cfg(feature = "bitvec")]
            OwnedPostgresValue::BitVec(bv) => PostgresValue::BitVec(bv),
            OwnedPostgresValue::Array(arr) => {
                let postgres_arr = arr.into_iter().map(|v| PostgresValue::from(v)).collect();
                PostgresValue::Array(postgres_arr)
            }

            OwnedPostgresValue::Null => PostgresValue::Null,
        }
    }
}

// Direct conversions from Rust types to OwnedPostgresValue
impl From<i16> for OwnedPostgresValue {
    fn from(value: i16) -> Self {
        OwnedPostgresValue::Smallint(value)
    }
}

impl From<i32> for OwnedPostgresValue {
    fn from(value: i32) -> Self {
        OwnedPostgresValue::Integer(value)
    }
}

impl From<i64> for OwnedPostgresValue {
    fn from(value: i64) -> Self {
        OwnedPostgresValue::Bigint(value)
    }
}

impl From<f32> for OwnedPostgresValue {
    fn from(value: f32) -> Self {
        OwnedPostgresValue::Real(value)
    }
}

impl From<f64> for OwnedPostgresValue {
    fn from(value: f64) -> Self {
        OwnedPostgresValue::DoublePrecision(value)
    }
}

impl From<String> for OwnedPostgresValue {
    fn from(value: String) -> Self {
        OwnedPostgresValue::Text(value)
    }
}

impl From<Vec<u8>> for OwnedPostgresValue {
    fn from(value: Vec<u8>) -> Self {
        OwnedPostgresValue::Bytea(value)
    }
}

impl From<bool> for OwnedPostgresValue {
    fn from(value: bool) -> Self {
        OwnedPostgresValue::Boolean(value)
    }
}

#[cfg(feature = "uuid")]
impl From<Uuid> for OwnedPostgresValue {
    fn from(value: Uuid) -> Self {
        OwnedPostgresValue::Uuid(value)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Value> for OwnedPostgresValue {
    fn from(value: serde_json::Value) -> Self {
        OwnedPostgresValue::Json(value)
    }
}

// TryFrom conversions back to Rust types
impl TryFrom<OwnedPostgresValue> for i16 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Smallint(i) => Ok(i),
            OwnedPostgresValue::Integer(i) => Ok(i.try_into()?),
            OwnedPostgresValue::Bigint(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to i16", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for i32 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Smallint(i) => Ok(i.into()),
            OwnedPostgresValue::Integer(i) => Ok(i),
            OwnedPostgresValue::Bigint(i) => Ok(i.try_into()?),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to i32", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for i64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Smallint(i) => Ok(i.into()),
            OwnedPostgresValue::Integer(i) => Ok(i.into()),
            OwnedPostgresValue::Bigint(i) => Ok(i),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to i64", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for f32 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Real(r) => Ok(r),
            OwnedPostgresValue::DoublePrecision(r) => Ok(r as f32),
            OwnedPostgresValue::Smallint(i) => Ok(i as f32),
            OwnedPostgresValue::Integer(i) => Ok(i as f32),
            OwnedPostgresValue::Bigint(i) => Ok(i as f32),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to f32", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for f64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Real(r) => Ok(r as f64),
            OwnedPostgresValue::DoublePrecision(r) => Ok(r),
            OwnedPostgresValue::Smallint(i) => Ok(i as f64),
            OwnedPostgresValue::Integer(i) => Ok(i as f64),
            OwnedPostgresValue::Bigint(i) => Ok(i as f64),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to f64", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for String {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Text(s) => Ok(s),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to String", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Bytea(b) => Ok(b),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Vec<u8>", value).into(),
            )),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for bool {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Boolean(b) => Ok(b),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to bool", value).into(),
            )),
        }
    }
}

#[cfg(feature = "uuid")]
impl TryFrom<OwnedPostgresValue> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Uuid(uuid) => Ok(uuid),
            OwnedPostgresValue::Text(s) => Ok(Uuid::parse_str(&s)?),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to UUID", value).into(),
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<OwnedPostgresValue> for serde_json::Value {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Json(json) => Ok(json),
            OwnedPostgresValue::Text(s) => serde_json::from_str(&s).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse JSON: {}", e).into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to JSON", value).into(),
            )),
        }
    }
}
