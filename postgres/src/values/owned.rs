//! Owned PostgreSQL value types for static lifetime scenarios

use super::PostgresValue;
use crate::traits::FromPostgresValue;
use drizzle_core::{SQLParam, error::DrizzleError, sql::SQL};
use std::borrow::Cow;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

#[cfg(feature = "cidr")]
use cidr::{IpCidr, IpInet};

#[cfg(feature = "geo-types")]
use geo_types::{LineString, Point, Rect};

#[cfg(feature = "bit-vec")]
use bit_vec::BitVec;

/// Owned version of PostgresValue that doesn't borrow data
#[derive(Debug, Clone, PartialEq, Default)]
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
    /// JSON values (stored as text in PostgreSQL)
    #[cfg(feature = "serde")]
    Json(serde_json::Value),
    /// JSONB values (stored as binary in PostgreSQL)
    #[cfg(feature = "serde")]
    Jsonb(serde_json::Value),

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

    // Network address types
    /// INET values (host address with optional netmask)
    #[cfg(feature = "cidr")]
    Inet(IpInet),
    /// CIDR values (network specification)
    #[cfg(feature = "cidr")]
    Cidr(IpCidr),
    /// MACADDR values (MAC addresses)
    #[cfg(feature = "cidr")]
    MacAddr([u8; 6]),
    /// MACADDR8 values (EUI-64 MAC addresses)
    #[cfg(feature = "cidr")]
    MacAddr8([u8; 8]),

    // Geometric types (native PostgreSQL support via postgres-rs)
    /// POINT values
    #[cfg(feature = "geo-types")]
    Point(Point<f64>),
    /// PATH values (open path from LineString)
    #[cfg(feature = "geo-types")]
    LineString(LineString<f64>),
    /// BOX values (bounding rectangle)
    #[cfg(feature = "geo-types")]
    Rect(Rect<f64>),

    // Bit string types
    /// BIT, BIT VARYING values
    #[cfg(feature = "bit-vec")]
    BitVec(BitVec),

    // Array types (using Vec for simplicity)
    /// Array of any PostgreSQL type
    Array(Vec<OwnedPostgresValue>),

    /// NULL value
    #[default]
    Null,
}

impl SQLParam for OwnedPostgresValue {
    const DIALECT: drizzle_core::Dialect = drizzle_core::Dialect::PostgreSQL;
}

impl<'a> From<OwnedPostgresValue> for SQL<'a, OwnedPostgresValue> {
    fn from(value: OwnedPostgresValue) -> Self {
        SQL::param(value)
    }
}

impl From<OwnedPostgresValue> for Cow<'_, OwnedPostgresValue> {
    fn from(value: OwnedPostgresValue) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a OwnedPostgresValue> for Cow<'a, OwnedPostgresValue> {
    fn from(value: &'a OwnedPostgresValue) -> Self {
        Cow::Borrowed(value)
    }
}

impl OwnedPostgresValue {
    /// Returns true if this value is NULL.
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, OwnedPostgresValue::Null)
    }

    /// Returns the boolean value if this is BOOLEAN.
    #[inline]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            OwnedPostgresValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the i16 value if this is SMALLINT.
    #[inline]
    pub const fn as_i16(&self) -> Option<i16> {
        match self {
            OwnedPostgresValue::Smallint(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the i32 value if this is INTEGER.
    #[inline]
    pub const fn as_i32(&self) -> Option<i32> {
        match self {
            OwnedPostgresValue::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the i64 value if this is BIGINT.
    #[inline]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            OwnedPostgresValue::Bigint(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the f32 value if this is REAL.
    #[inline]
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            OwnedPostgresValue::Real(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the f64 value if this is DOUBLE PRECISION.
    #[inline]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            OwnedPostgresValue::DoublePrecision(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the text value if this is TEXT.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            OwnedPostgresValue::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// Returns the bytea value if this is BYTEA.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            OwnedPostgresValue::Bytea(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns the UUID value if this is UUID.
    #[inline]
    #[cfg(feature = "uuid")]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            OwnedPostgresValue::Uuid(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the JSON value if this is JSON.
    #[inline]
    #[cfg(feature = "serde")]
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            OwnedPostgresValue::Json(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the JSONB value if this is JSONB.
    #[inline]
    #[cfg(feature = "serde")]
    pub fn as_jsonb(&self) -> Option<&serde_json::Value> {
        match self {
            OwnedPostgresValue::Jsonb(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the date value if this is DATE.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_date(&self) -> Option<&NaiveDate> {
        match self {
            OwnedPostgresValue::Date(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the time value if this is TIME.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_time(&self) -> Option<&NaiveTime> {
        match self {
            OwnedPostgresValue::Time(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the timestamp value if this is TIMESTAMP.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_timestamp(&self) -> Option<&NaiveDateTime> {
        match self {
            OwnedPostgresValue::Timestamp(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the timestamp with timezone value if this is TIMESTAMPTZ.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_timestamp_tz(&self) -> Option<&DateTime<FixedOffset>> {
        match self {
            OwnedPostgresValue::TimestampTz(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the interval value if this is INTERVAL.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_interval(&self) -> Option<&Duration> {
        match self {
            OwnedPostgresValue::Interval(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the inet value if this is INET.
    #[inline]
    #[cfg(feature = "cidr")]
    pub fn as_inet(&self) -> Option<&IpInet> {
        match self {
            OwnedPostgresValue::Inet(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the cidr value if this is CIDR.
    #[inline]
    #[cfg(feature = "cidr")]
    pub fn as_cidr(&self) -> Option<&IpCidr> {
        match self {
            OwnedPostgresValue::Cidr(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the MAC address if this is MACADDR.
    #[inline]
    #[cfg(feature = "cidr")]
    pub const fn as_macaddr(&self) -> Option<[u8; 6]> {
        match self {
            OwnedPostgresValue::MacAddr(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the MAC address if this is MACADDR8.
    #[inline]
    #[cfg(feature = "cidr")]
    pub const fn as_macaddr8(&self) -> Option<[u8; 8]> {
        match self {
            OwnedPostgresValue::MacAddr8(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the point value if this is POINT.
    #[inline]
    #[cfg(feature = "geo-types")]
    pub fn as_point(&self) -> Option<&Point<f64>> {
        match self {
            OwnedPostgresValue::Point(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the line string value if this is PATH.
    #[inline]
    #[cfg(feature = "geo-types")]
    pub fn as_line_string(&self) -> Option<&LineString<f64>> {
        match self {
            OwnedPostgresValue::LineString(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the rect value if this is BOX.
    #[inline]
    #[cfg(feature = "geo-types")]
    pub fn as_rect(&self) -> Option<&Rect<f64>> {
        match self {
            OwnedPostgresValue::Rect(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the bit vector if this is BIT/VARBIT.
    #[inline]
    #[cfg(feature = "bit-vec")]
    pub fn as_bitvec(&self) -> Option<&BitVec> {
        match self {
            OwnedPostgresValue::BitVec(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the array elements if this is an ARRAY.
    #[inline]
    pub fn as_array(&self) -> Option<&[OwnedPostgresValue]> {
        match self {
            OwnedPostgresValue::Array(values) => Some(values),
            _ => None,
        }
    }

    /// Returns a borrowed PostgresValue view of this owned value.
    #[inline]
    pub fn as_value(&self) -> PostgresValue<'_> {
        match self {
            OwnedPostgresValue::Smallint(value) => PostgresValue::Smallint(*value),
            OwnedPostgresValue::Integer(value) => PostgresValue::Integer(*value),
            OwnedPostgresValue::Bigint(value) => PostgresValue::Bigint(*value),
            OwnedPostgresValue::Real(value) => PostgresValue::Real(*value),
            OwnedPostgresValue::DoublePrecision(value) => PostgresValue::DoublePrecision(*value),
            OwnedPostgresValue::Text(value) => PostgresValue::Text(Cow::Borrowed(value)),
            OwnedPostgresValue::Bytea(value) => PostgresValue::Bytea(Cow::Borrowed(value)),
            OwnedPostgresValue::Boolean(value) => PostgresValue::Boolean(*value),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(value) => PostgresValue::Uuid(*value),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(value) => PostgresValue::Json(value.clone()),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Jsonb(value) => PostgresValue::Jsonb(value.clone()),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Date(value) => PostgresValue::Date(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Time(value) => PostgresValue::Time(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Timestamp(value) => PostgresValue::Timestamp(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::TimestampTz(value) => PostgresValue::TimestampTz(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Interval(value) => PostgresValue::Interval(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Inet(value) => PostgresValue::Inet(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Cidr(value) => PostgresValue::Cidr(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr(value) => PostgresValue::MacAddr(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr8(value) => PostgresValue::MacAddr8(*value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(value) => PostgresValue::Point(*value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(value) => PostgresValue::LineString(value.clone()),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Rect(value) => PostgresValue::Rect(*value),
            #[cfg(feature = "bit-vec")]
            OwnedPostgresValue::BitVec(value) => PostgresValue::BitVec(value.clone()),
            OwnedPostgresValue::Array(values) => {
                PostgresValue::Array(values.iter().map(OwnedPostgresValue::as_value).collect())
            }
            OwnedPostgresValue::Null => PostgresValue::Null,
        }
    }

    /// Convert this PostgreSQL value to a Rust type using the `FromPostgresValue` trait.
    pub fn convert<T: FromPostgresValue>(self) -> Result<T, DrizzleError> {
        match self {
            OwnedPostgresValue::Boolean(value) => T::from_postgres_bool(value),
            OwnedPostgresValue::Smallint(value) => T::from_postgres_i16(value),
            OwnedPostgresValue::Integer(value) => T::from_postgres_i32(value),
            OwnedPostgresValue::Bigint(value) => T::from_postgres_i64(value),
            OwnedPostgresValue::Real(value) => T::from_postgres_f32(value),
            OwnedPostgresValue::DoublePrecision(value) => T::from_postgres_f64(value),
            OwnedPostgresValue::Text(value) => T::from_postgres_text(&value),
            OwnedPostgresValue::Bytea(value) => T::from_postgres_bytes(&value),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(value) => T::from_postgres_uuid(value),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(value) => T::from_postgres_json(value),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Jsonb(value) => T::from_postgres_jsonb(value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Date(value) => T::from_postgres_date(value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Time(value) => T::from_postgres_time(value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Timestamp(value) => T::from_postgres_timestamp(value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::TimestampTz(value) => T::from_postgres_timestamptz(value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Interval(value) => T::from_postgres_interval(value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Inet(value) => T::from_postgres_inet(value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Cidr(value) => T::from_postgres_cidr(value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr(value) => T::from_postgres_macaddr(value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr8(value) => T::from_postgres_macaddr8(value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(value) => T::from_postgres_point(value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(value) => T::from_postgres_linestring(value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Rect(value) => T::from_postgres_rect(value),
            #[cfg(feature = "bit-vec")]
            OwnedPostgresValue::BitVec(value) => T::from_postgres_bitvec(value),
            OwnedPostgresValue::Array(values) => {
                let values = values.into_iter().map(PostgresValue::from).collect();
                T::from_postgres_array(values)
            }
            OwnedPostgresValue::Null => T::from_postgres_null(),
        }
    }

    /// Convert a reference to this PostgreSQL value to a Rust type.
    pub fn convert_ref<T: FromPostgresValue>(&self) -> Result<T, DrizzleError> {
        match self {
            OwnedPostgresValue::Boolean(value) => T::from_postgres_bool(*value),
            OwnedPostgresValue::Smallint(value) => T::from_postgres_i16(*value),
            OwnedPostgresValue::Integer(value) => T::from_postgres_i32(*value),
            OwnedPostgresValue::Bigint(value) => T::from_postgres_i64(*value),
            OwnedPostgresValue::Real(value) => T::from_postgres_f32(*value),
            OwnedPostgresValue::DoublePrecision(value) => T::from_postgres_f64(*value),
            OwnedPostgresValue::Text(value) => T::from_postgres_text(value),
            OwnedPostgresValue::Bytea(value) => T::from_postgres_bytes(value),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(value) => T::from_postgres_uuid(*value),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(value) => T::from_postgres_json(value.clone()),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Jsonb(value) => T::from_postgres_jsonb(value.clone()),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Date(value) => T::from_postgres_date(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Time(value) => T::from_postgres_time(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Timestamp(value) => T::from_postgres_timestamp(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::TimestampTz(value) => T::from_postgres_timestamptz(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Interval(value) => T::from_postgres_interval(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Inet(value) => T::from_postgres_inet(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Cidr(value) => T::from_postgres_cidr(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr(value) => T::from_postgres_macaddr(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr8(value) => T::from_postgres_macaddr8(*value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(value) => T::from_postgres_point(*value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(value) => T::from_postgres_linestring(value.clone()),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Rect(value) => T::from_postgres_rect(*value),
            #[cfg(feature = "bit-vec")]
            OwnedPostgresValue::BitVec(value) => T::from_postgres_bitvec(value.clone()),
            OwnedPostgresValue::Array(values) => {
                let values = values.iter().map(OwnedPostgresValue::as_value).collect();
                T::from_postgres_array(values)
            }
            OwnedPostgresValue::Null => T::from_postgres_null(),
        }
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
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Jsonb(json) => json.to_string(),

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

            // Network address types
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Inet(net) => net.to_string(),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Cidr(net) => net.to_string(),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr(mac) => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            ),
            #[cfg(feature = "cidr")]
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
            OwnedPostgresValue::Rect(rect) => {
                format!(
                    "(({},{}),({},{}))",
                    rect.min().x,
                    rect.min().y,
                    rect.max().x,
                    rect.max().y
                )
            }

            // Bit string types
            #[cfg(feature = "bit-vec")]
            OwnedPostgresValue::BitVec(bv) => bv
                .iter()
                .map(|b| if b { '1' } else { '0' })
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
            #[cfg(feature = "serde")]
            PostgresValue::Jsonb(json) => OwnedPostgresValue::Jsonb(json),
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
            #[cfg(feature = "cidr")]
            PostgresValue::Inet(net) => OwnedPostgresValue::Inet(net),
            #[cfg(feature = "cidr")]
            PostgresValue::Cidr(net) => OwnedPostgresValue::Cidr(net),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr(mac) => OwnedPostgresValue::MacAddr(mac),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr8(mac) => OwnedPostgresValue::MacAddr8(mac),
            #[cfg(feature = "geo-types")]
            PostgresValue::Point(point) => OwnedPostgresValue::Point(point),
            #[cfg(feature = "geo-types")]
            PostgresValue::LineString(line) => OwnedPostgresValue::LineString(line),
            #[cfg(feature = "geo-types")]
            PostgresValue::Rect(rect) => OwnedPostgresValue::Rect(rect),
            #[cfg(feature = "bit-vec")]
            PostgresValue::BitVec(bv) => OwnedPostgresValue::BitVec(bv),
            PostgresValue::Array(arr) => {
                let owned_arr = arr.into_iter().map(OwnedPostgresValue::from).collect();
                OwnedPostgresValue::Array(owned_arr)
            }
        }
    }
}

impl<'a> From<&PostgresValue<'a>> for OwnedPostgresValue {
    fn from(value: &PostgresValue<'a>) -> Self {
        match value {
            PostgresValue::Smallint(i) => OwnedPostgresValue::Smallint(*i),
            PostgresValue::Integer(i) => OwnedPostgresValue::Integer(*i),
            PostgresValue::Bigint(i) => OwnedPostgresValue::Bigint(*i),
            PostgresValue::Real(r) => OwnedPostgresValue::Real(*r),
            PostgresValue::DoublePrecision(r) => OwnedPostgresValue::DoublePrecision(*r),
            PostgresValue::Text(cow) => OwnedPostgresValue::Text(cow.clone().into_owned()),
            PostgresValue::Bytea(cow) => OwnedPostgresValue::Bytea(cow.clone().into_owned()),
            PostgresValue::Boolean(b) => OwnedPostgresValue::Boolean(*b),
            #[cfg(feature = "uuid")]
            PostgresValue::Uuid(uuid) => OwnedPostgresValue::Uuid(*uuid),
            #[cfg(feature = "serde")]
            PostgresValue::Json(json) => OwnedPostgresValue::Json(json.clone()),
            #[cfg(feature = "serde")]
            PostgresValue::Jsonb(json) => OwnedPostgresValue::Jsonb(json.clone()),
            PostgresValue::Enum(enum_val) => {
                OwnedPostgresValue::Text(enum_val.variant_name().to_string())
            }
            #[cfg(feature = "chrono")]
            PostgresValue::Date(value) => OwnedPostgresValue::Date(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::Time(value) => OwnedPostgresValue::Time(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::Timestamp(value) => OwnedPostgresValue::Timestamp(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::TimestampTz(value) => OwnedPostgresValue::TimestampTz(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::Interval(value) => OwnedPostgresValue::Interval(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::Inet(value) => OwnedPostgresValue::Inet(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::Cidr(value) => OwnedPostgresValue::Cidr(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr(value) => OwnedPostgresValue::MacAddr(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr8(value) => OwnedPostgresValue::MacAddr8(*value),
            #[cfg(feature = "geo-types")]
            PostgresValue::Point(value) => OwnedPostgresValue::Point(*value),
            #[cfg(feature = "geo-types")]
            PostgresValue::LineString(value) => OwnedPostgresValue::LineString(value.clone()),
            #[cfg(feature = "geo-types")]
            PostgresValue::Rect(value) => OwnedPostgresValue::Rect(*value),
            #[cfg(feature = "bit-vec")]
            PostgresValue::BitVec(value) => OwnedPostgresValue::BitVec(value.clone()),
            PostgresValue::Array(arr) => {
                let owned_arr = arr.iter().map(OwnedPostgresValue::from).collect();
                OwnedPostgresValue::Array(owned_arr)
            }
            PostgresValue::Null => OwnedPostgresValue::Null,
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
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Jsonb(json) => PostgresValue::Jsonb(json),

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
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Inet(net) => PostgresValue::Inet(net),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Cidr(net) => PostgresValue::Cidr(net),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr(mac) => PostgresValue::MacAddr(mac),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr8(mac) => PostgresValue::MacAddr8(mac),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(point) => PostgresValue::Point(point),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(line) => PostgresValue::LineString(line),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Rect(rect) => PostgresValue::Rect(rect),
            #[cfg(feature = "bit-vec")]
            OwnedPostgresValue::BitVec(bv) => PostgresValue::BitVec(bv),
            OwnedPostgresValue::Array(arr) => {
                let postgres_arr = arr.into_iter().map(PostgresValue::from).collect();
                PostgresValue::Array(postgres_arr)
            }

            OwnedPostgresValue::Null => PostgresValue::Null,
        }
    }
}

impl<'a> From<&'a OwnedPostgresValue> for PostgresValue<'a> {
    fn from(value: &'a OwnedPostgresValue) -> Self {
        match value {
            OwnedPostgresValue::Smallint(i) => PostgresValue::Smallint(*i),
            OwnedPostgresValue::Integer(i) => PostgresValue::Integer(*i),
            OwnedPostgresValue::Bigint(i) => PostgresValue::Bigint(*i),
            OwnedPostgresValue::Real(r) => PostgresValue::Real(*r),
            OwnedPostgresValue::DoublePrecision(r) => PostgresValue::DoublePrecision(*r),
            OwnedPostgresValue::Text(s) => PostgresValue::Text(Cow::Borrowed(s)),
            OwnedPostgresValue::Bytea(b) => PostgresValue::Bytea(Cow::Borrowed(b)),
            OwnedPostgresValue::Boolean(b) => PostgresValue::Boolean(*b),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(uuid) => PostgresValue::Uuid(*uuid),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(json) => PostgresValue::Json(json.clone()),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Jsonb(json) => PostgresValue::Jsonb(json.clone()),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Date(value) => PostgresValue::Date(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Time(value) => PostgresValue::Time(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Timestamp(value) => PostgresValue::Timestamp(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::TimestampTz(value) => PostgresValue::TimestampTz(*value),
            #[cfg(feature = "chrono")]
            OwnedPostgresValue::Interval(value) => PostgresValue::Interval(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Inet(value) => PostgresValue::Inet(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::Cidr(value) => PostgresValue::Cidr(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr(value) => PostgresValue::MacAddr(*value),
            #[cfg(feature = "cidr")]
            OwnedPostgresValue::MacAddr8(value) => PostgresValue::MacAddr8(*value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Point(value) => PostgresValue::Point(*value),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::LineString(value) => PostgresValue::LineString(value.clone()),
            #[cfg(feature = "geo-types")]
            OwnedPostgresValue::Rect(value) => PostgresValue::Rect(*value),
            #[cfg(feature = "bit-vec")]
            OwnedPostgresValue::BitVec(value) => PostgresValue::BitVec(value.clone()),
            OwnedPostgresValue::Array(values) => {
                PostgresValue::Array(values.iter().map(PostgresValue::from).collect())
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

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayString<N>> for OwnedPostgresValue {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        OwnedPostgresValue::Text(value.to_string())
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for OwnedPostgresValue {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        OwnedPostgresValue::Bytea(value.to_vec())
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
            OwnedPostgresValue::Text(s) => Uuid::parse_str(&s).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse UUID: {}", e).into())
            }),
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
            OwnedPostgresValue::Jsonb(json) => Ok(json),
            OwnedPostgresValue::Text(s) => serde_json::from_str(&s).map_err(|e| {
                DrizzleError::ConversionError(format!("Failed to parse JSON: {}", e).into())
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to JSON", value).into(),
            )),
        }
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> TryFrom<OwnedPostgresValue> for arrayvec::ArrayString<N> {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Text(s) => arrayvec::ArrayString::from(&s).map_err(|_| {
                DrizzleError::ConversionError(
                    format!("Text length {} exceeds ArrayString capacity {}", s.len(), N).into(),
                )
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to ArrayString", value).into(),
            )),
        }
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> TryFrom<OwnedPostgresValue> for arrayvec::ArrayVec<u8, N> {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Bytea(bytes) => arrayvec::ArrayVec::try_from(bytes.as_slice())
                .map_err(|_| {
                    DrizzleError::ConversionError(
                        format!(
                            "Bytea length {} exceeds ArrayVec capacity {}",
                            bytes.len(),
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
