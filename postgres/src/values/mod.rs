//! PostgreSQL value conversion traits and types

mod conversions;
mod drivers;
mod insert;
mod owned;
mod update;

pub use insert::*;
pub use owned::*;
pub use update::*;

use drizzle_core::{error::DrizzleError, sql::SQL, traits::SQLParam};

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

use crate::prelude::*;

use crate::traits::{FromPostgresValue, PostgresEnum};

//------------------------------------------------------------------------------
// PostgresValue Definition
//------------------------------------------------------------------------------

/// Represents a PostgreSQL value.
///
/// This enum provides type-safe value handling for PostgreSQL operations.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::values::PostgresValue;
///
/// // Integer conversion
/// let int_val: PostgresValue<'_> = 42i32.into();
/// assert!(matches!(int_val, PostgresValue::Integer(42)));
///
/// // String conversion
/// let str_val: PostgresValue<'_> = "hello".into();
/// assert!(matches!(str_val, PostgresValue::Text(_)));
///
/// // Boolean conversion
/// let bool_val: PostgresValue<'_> = true.into();
/// assert!(matches!(bool_val, PostgresValue::Boolean(true)));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PostgresValue<'a> {
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
    /// TEXT, VARCHAR, CHAR values
    Text(Cow<'a, str>),
    /// BYTEA values (binary data)
    Bytea(Cow<'a, [u8]>),
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
    /// Native PostgreSQL ENUM values
    Enum(Box<dyn PostgresEnum>),

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
    Array(Vec<PostgresValue<'a>>),

    /// NULL value
    #[default]
    Null,
}

impl<'a> core::fmt::Display for PostgresValue<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = match self {
            PostgresValue::Smallint(i) => i.to_string(),
            PostgresValue::Integer(i) => i.to_string(),
            PostgresValue::Bigint(i) => i.to_string(),
            PostgresValue::Real(r) => r.to_string(),
            PostgresValue::DoublePrecision(r) => r.to_string(),
            PostgresValue::Text(cow) => cow.to_string(),
            PostgresValue::Bytea(cow) => format!(
                "\\x{}",
                cow.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            ),
            PostgresValue::Boolean(b) => b.to_string(),
            #[cfg(feature = "uuid")]
            PostgresValue::Uuid(uuid) => uuid.to_string(),
            #[cfg(feature = "serde")]
            PostgresValue::Json(json) => json.to_string(),
            #[cfg(feature = "serde")]
            PostgresValue::Jsonb(json) => json.to_string(),
            PostgresValue::Enum(enum_val) => enum_val.variant_name().to_string(),

            // Date and time types
            #[cfg(feature = "chrono")]
            PostgresValue::Date(date) => date.format("%Y-%m-%d").to_string(),
            #[cfg(feature = "chrono")]
            PostgresValue::Time(time) => time.format("%H:%M:%S%.f").to_string(),
            #[cfg(feature = "chrono")]
            PostgresValue::Timestamp(ts) => ts.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
            #[cfg(feature = "chrono")]
            PostgresValue::TimestampTz(ts) => ts.format("%Y-%m-%d %H:%M:%S%.f %:z").to_string(),
            #[cfg(feature = "chrono")]
            PostgresValue::Interval(dur) => format!("{} seconds", dur.num_seconds()),

            // Network address types
            #[cfg(feature = "cidr")]
            PostgresValue::Inet(net) => net.to_string(),
            #[cfg(feature = "cidr")]
            PostgresValue::Cidr(net) => net.to_string(),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr(mac) => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            ),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr8(mac) => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[6], mac[7]
            ),

            // Geometric types
            #[cfg(feature = "geo-types")]
            PostgresValue::Point(point) => format!("({},{})", point.x(), point.y()),
            #[cfg(feature = "geo-types")]
            PostgresValue::LineString(line) => {
                let coords: Vec<String> = line
                    .coords()
                    .map(|coord| format!("({},{})", coord.x, coord.y))
                    .collect();
                format!("[{}]", coords.join(","))
            }
            #[cfg(feature = "geo-types")]
            PostgresValue::Rect(rect) => {
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
            PostgresValue::BitVec(bv) => bv
                .iter()
                .map(|b| if b { '1' } else { '0' })
                .collect::<String>(),

            // Array types
            PostgresValue::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("{{{}}}", elements.join(","))
            }

            PostgresValue::Null => String::new(),
        };
        write!(f, "{value}")
    }
}

impl<'a> PostgresValue<'a> {
    /// Returns true if this value is NULL.
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, PostgresValue::Null)
    }

    /// Returns the boolean value if this is BOOLEAN.
    #[inline]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            PostgresValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the i16 value if this is SMALLINT.
    #[inline]
    pub const fn as_i16(&self) -> Option<i16> {
        match self {
            PostgresValue::Smallint(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the i32 value if this is INTEGER.
    #[inline]
    pub const fn as_i32(&self) -> Option<i32> {
        match self {
            PostgresValue::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the i64 value if this is BIGINT.
    #[inline]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            PostgresValue::Bigint(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the f32 value if this is REAL.
    #[inline]
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            PostgresValue::Real(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the f64 value if this is DOUBLE PRECISION.
    #[inline]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            PostgresValue::DoublePrecision(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the text value if this is TEXT.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PostgresValue::Text(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns the bytea value if this is BYTEA.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            PostgresValue::Bytea(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns the UUID value if this is UUID.
    #[inline]
    #[cfg(feature = "uuid")]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            PostgresValue::Uuid(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the JSON value if this is JSON.
    #[inline]
    #[cfg(feature = "serde")]
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            PostgresValue::Json(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the JSONB value if this is JSONB.
    #[inline]
    #[cfg(feature = "serde")]
    pub fn as_jsonb(&self) -> Option<&serde_json::Value> {
        match self {
            PostgresValue::Jsonb(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the enum value if this is a PostgreSQL enum.
    #[inline]
    pub fn as_enum(&self) -> Option<&dyn PostgresEnum> {
        match self {
            PostgresValue::Enum(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Returns the date value if this is DATE.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_date(&self) -> Option<&NaiveDate> {
        match self {
            PostgresValue::Date(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the time value if this is TIME.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_time(&self) -> Option<&NaiveTime> {
        match self {
            PostgresValue::Time(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the timestamp value if this is TIMESTAMP.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_timestamp(&self) -> Option<&NaiveDateTime> {
        match self {
            PostgresValue::Timestamp(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the timestamp with timezone value if this is TIMESTAMPTZ.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_timestamp_tz(&self) -> Option<&DateTime<FixedOffset>> {
        match self {
            PostgresValue::TimestampTz(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the interval value if this is INTERVAL.
    #[inline]
    #[cfg(feature = "chrono")]
    pub fn as_interval(&self) -> Option<&Duration> {
        match self {
            PostgresValue::Interval(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the inet value if this is INET.
    #[inline]
    #[cfg(feature = "cidr")]
    pub fn as_inet(&self) -> Option<&IpInet> {
        match self {
            PostgresValue::Inet(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the cidr value if this is CIDR.
    #[inline]
    #[cfg(feature = "cidr")]
    pub fn as_cidr(&self) -> Option<&IpCidr> {
        match self {
            PostgresValue::Cidr(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the MAC address if this is MACADDR.
    #[inline]
    #[cfg(feature = "cidr")]
    pub const fn as_macaddr(&self) -> Option<[u8; 6]> {
        match self {
            PostgresValue::MacAddr(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the MAC address if this is MACADDR8.
    #[inline]
    #[cfg(feature = "cidr")]
    pub const fn as_macaddr8(&self) -> Option<[u8; 8]> {
        match self {
            PostgresValue::MacAddr8(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the point value if this is POINT.
    #[inline]
    #[cfg(feature = "geo-types")]
    pub fn as_point(&self) -> Option<&Point<f64>> {
        match self {
            PostgresValue::Point(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the line string value if this is PATH.
    #[inline]
    #[cfg(feature = "geo-types")]
    pub fn as_line_string(&self) -> Option<&LineString<f64>> {
        match self {
            PostgresValue::LineString(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the rect value if this is BOX.
    #[inline]
    #[cfg(feature = "geo-types")]
    pub fn as_rect(&self) -> Option<&Rect<f64>> {
        match self {
            PostgresValue::Rect(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the bit vector if this is BIT/VARBIT.
    #[inline]
    #[cfg(feature = "bit-vec")]
    pub fn as_bitvec(&self) -> Option<&BitVec> {
        match self {
            PostgresValue::BitVec(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the array elements if this is an ARRAY.
    #[inline]
    pub fn as_array(&self) -> Option<&[PostgresValue<'a>]> {
        match self {
            PostgresValue::Array(values) => Some(values),
            _ => None,
        }
    }

    /// Converts this value into an owned representation.
    #[inline]
    pub fn into_owned(self) -> OwnedPostgresValue {
        self.into()
    }

    /// Convert this PostgreSQL value to a Rust type using the `FromPostgresValue` trait.
    pub fn convert<T: FromPostgresValue>(self) -> Result<T, DrizzleError> {
        match self {
            PostgresValue::Boolean(value) => T::from_postgres_bool(value),
            PostgresValue::Smallint(value) => T::from_postgres_i16(value),
            PostgresValue::Integer(value) => T::from_postgres_i32(value),
            PostgresValue::Bigint(value) => T::from_postgres_i64(value),
            PostgresValue::Real(value) => T::from_postgres_f32(value),
            PostgresValue::DoublePrecision(value) => T::from_postgres_f64(value),
            PostgresValue::Text(value) => T::from_postgres_text(&value),
            PostgresValue::Bytea(value) => T::from_postgres_bytes(&value),
            #[cfg(feature = "uuid")]
            PostgresValue::Uuid(value) => T::from_postgres_uuid(value),
            #[cfg(feature = "serde")]
            PostgresValue::Json(value) => T::from_postgres_json(value),
            #[cfg(feature = "serde")]
            PostgresValue::Jsonb(value) => T::from_postgres_jsonb(value),
            PostgresValue::Enum(value) => T::from_postgres_text(value.variant_name()),
            #[cfg(feature = "chrono")]
            PostgresValue::Date(value) => T::from_postgres_date(value),
            #[cfg(feature = "chrono")]
            PostgresValue::Time(value) => T::from_postgres_time(value),
            #[cfg(feature = "chrono")]
            PostgresValue::Timestamp(value) => T::from_postgres_timestamp(value),
            #[cfg(feature = "chrono")]
            PostgresValue::TimestampTz(value) => T::from_postgres_timestamptz(value),
            #[cfg(feature = "chrono")]
            PostgresValue::Interval(value) => T::from_postgres_interval(value),
            #[cfg(feature = "cidr")]
            PostgresValue::Inet(value) => T::from_postgres_inet(value),
            #[cfg(feature = "cidr")]
            PostgresValue::Cidr(value) => T::from_postgres_cidr(value),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr(value) => T::from_postgres_macaddr(value),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr8(value) => T::from_postgres_macaddr8(value),
            #[cfg(feature = "geo-types")]
            PostgresValue::Point(value) => T::from_postgres_point(value),
            #[cfg(feature = "geo-types")]
            PostgresValue::LineString(value) => T::from_postgres_linestring(value),
            #[cfg(feature = "geo-types")]
            PostgresValue::Rect(value) => T::from_postgres_rect(value),
            #[cfg(feature = "bit-vec")]
            PostgresValue::BitVec(value) => T::from_postgres_bitvec(value),
            PostgresValue::Array(value) => T::from_postgres_array(value),
            PostgresValue::Null => T::from_postgres_null(),
        }
    }

    /// Convert a reference to this PostgreSQL value to a Rust type.
    pub fn convert_ref<T: FromPostgresValue>(&self) -> Result<T, DrizzleError> {
        match self {
            PostgresValue::Boolean(value) => T::from_postgres_bool(*value),
            PostgresValue::Smallint(value) => T::from_postgres_i16(*value),
            PostgresValue::Integer(value) => T::from_postgres_i32(*value),
            PostgresValue::Bigint(value) => T::from_postgres_i64(*value),
            PostgresValue::Real(value) => T::from_postgres_f32(*value),
            PostgresValue::DoublePrecision(value) => T::from_postgres_f64(*value),
            PostgresValue::Text(value) => T::from_postgres_text(value),
            PostgresValue::Bytea(value) => T::from_postgres_bytes(value),
            #[cfg(feature = "uuid")]
            PostgresValue::Uuid(value) => T::from_postgres_uuid(*value),
            #[cfg(feature = "serde")]
            PostgresValue::Json(value) => T::from_postgres_json(value.clone()),
            #[cfg(feature = "serde")]
            PostgresValue::Jsonb(value) => T::from_postgres_jsonb(value.clone()),
            PostgresValue::Enum(value) => T::from_postgres_text(value.variant_name()),
            #[cfg(feature = "chrono")]
            PostgresValue::Date(value) => T::from_postgres_date(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::Time(value) => T::from_postgres_time(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::Timestamp(value) => T::from_postgres_timestamp(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::TimestampTz(value) => T::from_postgres_timestamptz(*value),
            #[cfg(feature = "chrono")]
            PostgresValue::Interval(value) => T::from_postgres_interval(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::Inet(value) => T::from_postgres_inet(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::Cidr(value) => T::from_postgres_cidr(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr(value) => T::from_postgres_macaddr(*value),
            #[cfg(feature = "cidr")]
            PostgresValue::MacAddr8(value) => T::from_postgres_macaddr8(*value),
            #[cfg(feature = "geo-types")]
            PostgresValue::Point(value) => T::from_postgres_point(*value),
            #[cfg(feature = "geo-types")]
            PostgresValue::LineString(value) => T::from_postgres_linestring(value.clone()),
            #[cfg(feature = "geo-types")]
            PostgresValue::Rect(value) => T::from_postgres_rect(*value),
            #[cfg(feature = "bit-vec")]
            PostgresValue::BitVec(value) => T::from_postgres_bitvec(value.clone()),
            PostgresValue::Array(value) => T::from_postgres_array(value.clone()),
            PostgresValue::Null => T::from_postgres_null(),
        }
    }
}

// Implement core traits required by Drizzle
impl<'a> SQLParam for PostgresValue<'a> {
    const DIALECT: drizzle_core::dialect::Dialect = drizzle_core::dialect::Dialect::PostgreSQL;
    type DialectMarker = drizzle_core::dialect::PostgresDialect;
}

impl<'a> From<PostgresValue<'a>> for SQL<'a, PostgresValue<'a>> {
    fn from(value: PostgresValue<'a>) -> Self {
        SQL::param(value)
    }
}

// Cow integration for SQL struct
impl<'a> From<PostgresValue<'a>> for Cow<'a, PostgresValue<'a>> {
    fn from(value: PostgresValue<'a>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a PostgresValue<'a>> for Cow<'a, PostgresValue<'a>> {
    fn from(value: &'a PostgresValue<'a>) -> Self {
        Cow::Borrowed(value)
    }
}
