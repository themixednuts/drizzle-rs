//! PostgreSQL value conversion traits and types

use drizzle_core::{Placeholder, SQL, SQLParam, error::DrizzleError};

mod owned;
pub use owned::OwnedPostgresValue;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};

#[cfg(feature = "rust_decimal")]
use rust_decimal::Decimal;

#[cfg(feature = "ipnet")]
use ipnet::{IpNet, Ipv4Net, Ipv6Net};

#[cfg(feature = "geo-types")]
use geo_types::{LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

#[cfg(feature = "bitvec")]
use bitvec::prelude::*;

use std::borrow::Cow;
use std::marker::PhantomData;

use crate::{PostgresEnum, ToPostgresSQL};

//------------------------------------------------------------------------------
// InsertValue Definition - SQL-based value for inserts
//------------------------------------------------------------------------------

/// Wrapper for SQL with type information
#[derive(Debug, Clone)]
pub struct ValueWrapper<'a, V: SQLParam, T> {
    pub value: SQL<'a, V>,
    pub _phantom: PhantomData<T>,
}

impl<'a, V: SQLParam, T> ValueWrapper<'a, V, T> {
    pub const fn new<U>(value: SQL<'a, V>) -> ValueWrapper<'a, V, U> {
        ValueWrapper {
            value,
            _phantom: PhantomData,
        }
    }
}

/// Represents a value for INSERT operations that can be omitted, null, or a SQL expression
#[derive(Debug, Clone, Default)]
pub enum PostgresInsertValue<'a, V: SQLParam, T> {
    /// Omit this column from the INSERT (use database default)
    #[default]
    Omit,
    /// Explicitly insert NULL
    Null,
    /// Insert a SQL expression (value, placeholder, etc.)
    Value(ValueWrapper<'a, V, T>),
}

impl<'a, T> PostgresInsertValue<'a, PostgresValue<'a>, T> {
    /// Converts this InsertValue to an owned version with 'static lifetime
    pub fn into_owned(self) -> PostgresInsertValue<'static, PostgresValue<'static>, T> {
        match self {
            PostgresInsertValue::Omit => PostgresInsertValue::Omit,
            PostgresInsertValue::Null => PostgresInsertValue::Null,
            PostgresInsertValue::Value(wrapper) => {
                // Convert PostgresValue parameters to owned values
                if let Some(drizzle_core::SQLChunk::Param(param)) = wrapper.value.chunks.first() {
                    if let Some(ref postgres_val) = param.value {
                        let postgres_val = postgres_val.as_ref();
                        let owned_postgres_val = OwnedPostgresValue::from(postgres_val.clone());
                        let static_postgres_val = PostgresValue::from(owned_postgres_val);
                        let static_sql = drizzle_core::SQL::param(static_postgres_val);
                        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                            static_sql,
                        ))
                    } else {
                        // NULL parameter
                        let static_sql = drizzle_core::SQL::param(PostgresValue::Null);
                        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                            static_sql,
                        ))
                    }
                } else {
                    // Non-parameter chunk, convert to NULL for simplicity
                    let static_sql = drizzle_core::SQL::param(PostgresValue::Null);
                    PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'static>, T>::new(
                        static_sql,
                    ))
                }
            }
        }
    }
}

// Conversion implementations for PostgresValue-based InsertValue

// Generic conversion from any type T to InsertValue (for same type T)
impl<'a, T> From<T> for PostgresInsertValue<'a, PostgresValue<'a>, T>
where
    T: TryInto<PostgresValue<'a>>,
{
    fn from(value: T) -> Self {
        let sql = value
            .try_into()
            .map(|v: PostgresValue<'a>| SQL::from(v))
            .unwrap_or_else(|_| SQL::from(PostgresValue::Null));
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
    }
}

// Specific conversion for &str to String InsertValue
impl<'a> From<&str> for PostgresInsertValue<'a, PostgresValue<'a>, String> {
    fn from(value: &str) -> Self {
        let postgres_value = SQL::param(Cow::Owned(PostgresValue::from(value.to_string())));
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(
            postgres_value,
        ))
    }
}

// Placeholder conversion
impl<'a, T> From<Placeholder> for PostgresInsertValue<'a, PostgresValue<'a>, T> {
    fn from(placeholder: Placeholder) -> Self {
        use drizzle_core::{Param, SQLChunk};
        let chunk = SQLChunk::Param(Param {
            placeholder,
            value: None,
        });
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(
            std::iter::once(chunk).collect(),
        ))
    }
}

// Option conversion
impl<'a, T> From<Option<T>> for PostgresInsertValue<'a, PostgresValue<'a>, T>
where
    T: ToPostgresSQL<'a>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => {
                let sql = v.to_sql();
                PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, T>::new(sql))
            }
            None => PostgresInsertValue::Omit,
        }
    }
}

// UUID conversion for String InsertValue (for text columns)
#[cfg(feature = "uuid")]
impl<'a> From<Uuid> for PostgresInsertValue<'a, PostgresValue<'a>, String> {
    fn from(value: Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(value);
        let sql = SQL::param(postgres_value);
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for PostgresInsertValue<'a, PostgresValue<'a>, String> {
    fn from(value: &'a Uuid) -> Self {
        let postgres_value = PostgresValue::Uuid(*value);
        let sql = SQL::param(postgres_value);
        PostgresInsertValue::Value(ValueWrapper::<PostgresValue<'a>, String>::new(sql))
    }
}

//------------------------------------------------------------------------------
// PostgresValue Definition
//------------------------------------------------------------------------------

/// Represents a PostgreSQL value
#[derive(Debug, Clone, PartialEq)]
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
    /// JSON/JSONB values
    #[cfg(feature = "serde")]
    Json(serde_json::Value),
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
    Array(Vec<PostgresValue<'a>>),

    /// NULL value
    Null,
}

impl<'a> Default for PostgresValue<'a> {
    fn default() -> Self {
        PostgresValue::Null
    }
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

            // Numeric types
            #[cfg(feature = "rust_decimal")]
            PostgresValue::Decimal(dec) => dec.to_string(),

            // Network address types
            #[cfg(feature = "ipnet")]
            PostgresValue::Inet(net) => net.to_string(),
            #[cfg(feature = "ipnet")]
            PostgresValue::Cidr(net) => net.to_string(),
            #[cfg(feature = "ipnet")]
            PostgresValue::MacAddr(mac) => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            ),
            #[cfg(feature = "ipnet")]
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
            PostgresValue::Polygon(poly) => format!(
                "POLYGON({})",
                poly.exterior()
                    .coords()
                    .map(|c| format!("({},{})", c.x, c.y))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            #[cfg(feature = "geo-types")]
            PostgresValue::MultiPoint(mp) => format!(
                "MULTIPOINT({})",
                mp.iter()
                    .map(|p| format!("({},{})", p.x(), p.y()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            #[cfg(feature = "geo-types")]
            PostgresValue::MultiLineString(mls) => format!(
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
            PostgresValue::MultiPolygon(mp) => format!(
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
            PostgresValue::BitVec(bv) => bv
                .iter()
                .map(|b| if *b { '1' } else { '0' })
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
impl<'a> SQLParam for PostgresValue<'a> {}

impl<'a> From<PostgresValue<'a>> for SQL<'a, PostgresValue<'a>> {
    fn from(value: PostgresValue<'a>) -> Self {
        SQL::param(value)
    }
}

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
        PostgresValue::Text(Cow::Owned(value.as_str().to_owned()))
    }
}

// --- Binary Data ---

impl<'a> From<&'a [u8]> for PostgresValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        PostgresValue::Bytea(Cow::Borrowed(value))
    }
}

impl<'a> From<Vec<u8>> for PostgresValue<'a> {
    fn from(value: Vec<u8>) -> Self {
        PostgresValue::Bytea(Cow::Owned(value))
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

// --- Numeric Types ---

#[cfg(feature = "rust_decimal")]
impl<'a> From<Decimal> for PostgresValue<'a> {
    fn from(value: Decimal) -> Self {
        PostgresValue::Decimal(value)
    }
}

#[cfg(feature = "rust_decimal")]
impl<'a> From<&'a Decimal> for PostgresValue<'a> {
    fn from(value: &'a Decimal) -> Self {
        PostgresValue::Decimal(*value)
    }
}

// --- Network Address Types ---

#[cfg(feature = "ipnet")]
impl<'a> From<IpNet> for PostgresValue<'a> {
    fn from(value: IpNet) -> Self {
        PostgresValue::Inet(value)
    }
}

#[cfg(feature = "ipnet")]
impl<'a> From<&'a IpNet> for PostgresValue<'a> {
    fn from(value: &'a IpNet) -> Self {
        PostgresValue::Inet(*value)
    }
}

#[cfg(feature = "ipnet")]
impl<'a> From<Ipv4Net> for PostgresValue<'a> {
    fn from(value: Ipv4Net) -> Self {
        PostgresValue::Inet(value.into())
    }
}

#[cfg(feature = "ipnet")]
impl<'a> From<Ipv6Net> for PostgresValue<'a> {
    fn from(value: Ipv6Net) -> Self {
        PostgresValue::Inet(value.into())
    }
}

#[cfg(feature = "ipnet")]
impl<'a> From<[u8; 6]> for PostgresValue<'a> {
    fn from(value: [u8; 6]) -> Self {
        PostgresValue::MacAddr(value)
    }
}

#[cfg(feature = "ipnet")]
impl<'a> From<&'a [u8; 6]> for PostgresValue<'a> {
    fn from(value: &'a [u8; 6]) -> Self {
        PostgresValue::MacAddr(*value)
    }
}

#[cfg(feature = "ipnet")]
impl<'a> From<[u8; 8]> for PostgresValue<'a> {
    fn from(value: [u8; 8]) -> Self {
        PostgresValue::MacAddr8(value)
    }
}

#[cfg(feature = "ipnet")]
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
impl<'a> From<Polygon<f64>> for PostgresValue<'a> {
    fn from(value: Polygon<f64>) -> Self {
        PostgresValue::Polygon(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a Polygon<f64>> for PostgresValue<'a> {
    fn from(value: &'a Polygon<f64>) -> Self {
        PostgresValue::Polygon(value.clone())
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<MultiPoint<f64>> for PostgresValue<'a> {
    fn from(value: MultiPoint<f64>) -> Self {
        PostgresValue::MultiPoint(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a MultiPoint<f64>> for PostgresValue<'a> {
    fn from(value: &'a MultiPoint<f64>) -> Self {
        PostgresValue::MultiPoint(value.clone())
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<MultiLineString<f64>> for PostgresValue<'a> {
    fn from(value: MultiLineString<f64>) -> Self {
        PostgresValue::MultiLineString(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a MultiLineString<f64>> for PostgresValue<'a> {
    fn from(value: &'a MultiLineString<f64>) -> Self {
        PostgresValue::MultiLineString(value.clone())
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<MultiPolygon<f64>> for PostgresValue<'a> {
    fn from(value: MultiPolygon<f64>) -> Self {
        PostgresValue::MultiPolygon(value)
    }
}

#[cfg(feature = "geo-types")]
impl<'a> From<&'a MultiPolygon<f64>> for PostgresValue<'a> {
    fn from(value: &'a MultiPolygon<f64>) -> Self {
        PostgresValue::MultiPolygon(value.clone())
    }
}

// --- Bit String Types ---

#[cfg(feature = "bitvec")]
impl<'a> From<BitVec> for PostgresValue<'a> {
    fn from(value: BitVec) -> Self {
        PostgresValue::BitVec(value)
    }
}

#[cfg(feature = "bitvec")]
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

// --- Cow integration for SQL struct ---
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

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> TryFrom<PostgresValue<'a>> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Uuid(uuid) => Ok(uuid),
            PostgresValue::Text(cow) => Ok(Uuid::parse_str(cow.as_ref())?),
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

// --- Numeric TryFrom implementations ---

#[cfg(feature = "rust_decimal")]
impl<'a> TryFrom<PostgresValue<'a>> for Decimal {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Decimal(dec) => Ok(dec),
            PostgresValue::Smallint(i) => Ok(Decimal::from(i)),
            PostgresValue::Integer(i) => Ok(Decimal::from(i)),
            PostgresValue::Bigint(i) => Ok(Decimal::from(i)),
            PostgresValue::Real(f) => Decimal::try_from(f).map_err(|e| {
                DrizzleError::ConversionError(
                    format!("Failed to convert float to decimal: {}", e).into(),
                )
            }),
            PostgresValue::DoublePrecision(f) => Decimal::try_from(f).map_err(|e| {
                DrizzleError::ConversionError(
                    format!("Failed to convert float to decimal: {}", e).into(),
                )
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Decimal", value).into(),
            )),
        }
    }
}

// --- Network Address TryFrom implementations ---

#[cfg(feature = "ipnet")]
impl<'a> TryFrom<PostgresValue<'a>> for IpNet {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Inet(net) => Ok(net),
            PostgresValue::Cidr(net) => Ok(net),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to IpNet", value).into(),
            )),
        }
    }
}

#[cfg(feature = "ipnet")]
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

#[cfg(feature = "ipnet")]
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
impl<'a> TryFrom<PostgresValue<'a>> for Polygon<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::Polygon(poly) => Ok(poly),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to Polygon", value).into(),
            )),
        }
    }
}

#[cfg(feature = "geo-types")]
impl<'a> TryFrom<PostgresValue<'a>> for MultiPoint<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::MultiPoint(mp) => Ok(mp),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to MultiPoint", value).into(),
            )),
        }
    }
}

#[cfg(feature = "geo-types")]
impl<'a> TryFrom<PostgresValue<'a>> for MultiLineString<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::MultiLineString(mls) => Ok(mls),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to MultiLineString", value).into(),
            )),
        }
    }
}

#[cfg(feature = "geo-types")]
impl<'a> TryFrom<PostgresValue<'a>> for MultiPolygon<f64> {
    type Error = DrizzleError;

    fn try_from(value: PostgresValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PostgresValue::MultiPolygon(mp) => Ok(mp),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to MultiPolygon", value).into(),
            )),
        }
    }
}

// --- Bit String TryFrom implementations ---

#[cfg(feature = "bitvec")]
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

//------------------------------------------------------------------------------
// Database Driver Implementations
//------------------------------------------------------------------------------

// SQLx implementations when the sqlx-postgres feature is enabled
#[cfg(feature = "sqlx-postgres")]
mod sqlx_impls {
    use super::*;
    use sqlx::{Encode, Postgres, Type, postgres::PgArgumentBuffer};

    impl<'a> Type<Postgres> for PostgresValue<'a> {
        fn type_info() -> sqlx::postgres::PgTypeInfo {
            // This is a placeholder - in practice you'd need to handle different types
            <String as Type<Postgres>>::type_info()
        }
    }

    impl<'a> Encode<'_, Postgres> for PostgresValue<'a> {
        fn encode_by_ref(
            &self,
            buf: &mut PgArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
            match self {
                PostgresValue::Null => Ok(sqlx::encode::IsNull::Yes),
                PostgresValue::Integer(i) => i.encode_by_ref(buf),
                PostgresValue::Real(f) => f.encode_by_ref(buf),
                PostgresValue::Text(cow) => cow.as_ref().encode_by_ref(buf),
                PostgresValue::Bytea(cow) => cow.as_ref().encode_by_ref(buf),
                PostgresValue::Boolean(b) => b.encode_by_ref(buf),
                #[cfg(feature = "uuid")]
                PostgresValue::Uuid(uuid) => uuid.encode_by_ref(buf),
                #[cfg(feature = "serde")]
                PostgresValue::Json(json) => json.encode_by_ref(buf),
                PostgresValue::Enum(enum_val) => enum_val.variant_name().encode_by_ref(buf),
            }
        }
    }
}
