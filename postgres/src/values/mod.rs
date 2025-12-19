//! PostgreSQL value conversion traits and types

mod conversions;
mod drivers;
mod insert;
mod owned;

pub use insert::*;
pub use owned::*;

use drizzle_core::{sql::SQL, traits::SQLParam};

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

use std::borrow::Cow;

use crate::PostgresEnum;

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

impl<'a> std::fmt::Display for PostgresValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl<'a> From<SQL<'a, PostgresValue<'a>>> for PostgresValue<'a> {
    fn from(_value: SQL<'a, PostgresValue<'a>>) -> Self {
        unimplemented!()
    }
}

// Implement core traits required by Drizzle
impl<'a> SQLParam for PostgresValue<'a> {
    const DIALECT: drizzle_core::dialect::Dialect = drizzle_core::dialect::Dialect::PostgreSQL;
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
