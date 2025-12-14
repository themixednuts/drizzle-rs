//! PostgreSQL type category definitions
//!
//! Provides type classification for both Rust type mapping and SQL parsing.

use super::PostgreSQLType;

// =============================================================================
// TypeCategory - Rust type classification for code generation
// =============================================================================

/// Categorizes Rust types for consistent handling across the PostgreSQL macro system.
///
/// This enum provides a single source of truth for type detection, eliminating
/// fragile string matching scattered across multiple files.
///
/// # Examples
///
/// ```
/// use drizzle_types::postgres::TypeCategory;
///
/// let category = TypeCategory::from_type_string("String");
/// assert_eq!(category, TypeCategory::String);
///
/// let i32_cat = TypeCategory::from_type_string("i32");
/// assert_eq!(i32_cat, TypeCategory::I32);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TypeCategory {
    /// `arrayvec::ArrayString<N>` - Fixed-capacity string on the stack
    ArrayString,
    /// `arrayvec::ArrayVec<u8, N>` - Fixed-capacity byte array on the stack
    ArrayVec,
    /// `std::string::String` - Heap-allocated string
    String,
    /// `Vec<u8>` - Heap-allocated byte array
    Blob,
    /// `[u8; N]` - Fixed-size byte array
    ByteArray,
    /// `[char; N]` - Fixed-size char array
    CharArray,
    /// `uuid::Uuid` - UUID type
    Uuid,
    /// `serde_json::Value` - JSON type
    Json,
    /// Any type with enum flag
    Enum,
    /// `i16`
    I16,
    /// `i32`
    I32,
    /// `i64`
    I64,
    /// `f32`
    F32,
    /// `f64`
    F64,
    /// `bool`
    Bool,
    // ========== Chrono types (with-chrono-0_4) ==========
    /// `chrono::NaiveDate` -> DATE
    NaiveDate,
    /// `chrono::NaiveTime` -> TIME
    NaiveTime,
    /// `chrono::NaiveDateTime` -> TIMESTAMP
    NaiveDateTime,
    /// `chrono::DateTime<Tz>` -> TIMESTAMPTZ
    DateTimeTz,
    // ========== Time crate types (with-time-0_3) ==========
    /// `time::Date` -> DATE
    TimeDate,
    /// `time::Time` -> TIME
    TimeTime,
    /// `time::PrimitiveDateTime` -> TIMESTAMP
    TimePrimitiveDateTime,
    /// `time::OffsetDateTime` -> TIMESTAMPTZ
    TimeOffsetDateTime,
    // ========== Geo types (with-geo-types-0_7) ==========
    /// `geo_types::Point<f64>` -> POINT
    GeoPoint,
    /// `geo_types::Rect<f64>` -> BOX
    GeoRect,
    /// `geo_types::LineString<f64>` -> PATH
    GeoLineString,
    // ========== Network types (with-cidr-0_3) ==========
    /// `std::net::IpAddr` or `cidr::IpInet` -> INET
    IpAddr,
    /// `cidr::IpCidr` -> CIDR
    Cidr,
    // ========== MAC address (with-eui48-1) ==========
    /// `eui48::MacAddress` -> MACADDR
    MacAddr,
    // ========== Bit types (with-bit-vec-0_8) ==========
    /// `bit_vec::BitVec` -> BIT VARYING
    BitVec,
    /// Unknown type - will result in compile error
    Unknown,
}

impl TypeCategory {
    /// Detect the category from a type string representation.
    ///
    /// Order matters: more specific types (ArrayString) must be checked
    /// before more general types (String).
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_type_string(type_str: &str) -> Self {
        // Remove whitespace for consistent matching
        let type_str = type_str.replace(' ', "");

        // Handle Option<T> wrapper - recurse into inner type
        if type_str.starts_with("Option<") && type_str.ends_with('>') {
            let inner = &type_str[7..type_str.len() - 1];
            return Self::from_type_string(inner);
        }

        // Fixed-size arrays first
        if type_str.starts_with("[u8;") || type_str.contains("[u8;") {
            return TypeCategory::ByteArray;
        }
        if type_str.starts_with("[char;") || type_str.contains("[char;") {
            return TypeCategory::CharArray;
        }

        // ArrayVec/ArrayString before generic checks
        if type_str.contains("ArrayString") {
            return TypeCategory::ArrayString;
        }
        if type_str.contains("ArrayVec") && type_str.contains("u8") {
            return TypeCategory::ArrayVec;
        }

        // UUID
        if type_str.contains("Uuid") {
            return TypeCategory::Uuid;
        }

        // JSON (serde_json::Value)
        if type_str.contains("serde_json::Value") || type_str == "Value" {
            return TypeCategory::Json;
        }

        // Chrono types (check specific types before generic DateTime)
        if type_str.contains("NaiveDate") && !type_str.contains("NaiveDateTime") {
            return TypeCategory::NaiveDate;
        }
        if type_str.contains("NaiveTime") {
            return TypeCategory::NaiveTime;
        }
        if type_str.contains("NaiveDateTime") {
            return TypeCategory::NaiveDateTime;
        }
        if type_str.contains("DateTime<") {
            return TypeCategory::DateTimeTz;
        }

        // Time crate types
        if type_str.contains("time::Date") || type_str == "Date" {
            return TypeCategory::TimeDate;
        }
        if type_str.contains("time::Time") {
            return TypeCategory::TimeTime;
        }
        if type_str.contains("PrimitiveDateTime") {
            return TypeCategory::TimePrimitiveDateTime;
        }
        if type_str.contains("OffsetDateTime") {
            return TypeCategory::TimeOffsetDateTime;
        }

        // Geo types
        if type_str.contains("Point<") || type_str.contains("geo_types::Point") {
            return TypeCategory::GeoPoint;
        }
        if type_str.contains("Rect<") || type_str.contains("geo_types::Rect") {
            return TypeCategory::GeoRect;
        }
        if type_str.contains("LineString<") || type_str.contains("geo_types::LineString") {
            return TypeCategory::GeoLineString;
        }

        // Network types (cidr crate)
        if type_str.contains("IpInet") || type_str.contains("IpAddr") {
            return TypeCategory::IpAddr;
        }
        if type_str.contains("IpCidr") {
            return TypeCategory::Cidr;
        }

        // MAC address
        if type_str.contains("MacAddress") || type_str.contains("eui48") {
            return TypeCategory::MacAddr;
        }

        // Bit vector
        if type_str.contains("BitVec") {
            return TypeCategory::BitVec;
        }

        // String types
        if type_str.contains("String") {
            return TypeCategory::String;
        }

        // Vec<u8>
        if type_str.contains("Vec<u8>") {
            return TypeCategory::Blob;
        }

        // Primitives - check exact matches for simple types
        match type_str.as_str() {
            "i16" => TypeCategory::I16,
            "i32" => TypeCategory::I32,
            "i64" => TypeCategory::I64,
            "f32" => TypeCategory::F32,
            "f64" => TypeCategory::F64,
            "bool" => TypeCategory::Bool,
            _ => TypeCategory::Unknown,
        }
    }

    /// Infer the PostgreSQL type from this category.
    ///
    /// Returns `None` for Unknown types (should trigger compile error).
    #[must_use]
    pub const fn to_postgres_type(&self) -> Option<PostgreSQLType> {
        match self {
            // Numeric types
            TypeCategory::I16 => Some(PostgreSQLType::Smallint),
            TypeCategory::I32 => Some(PostgreSQLType::Integer),
            TypeCategory::I64 => Some(PostgreSQLType::Bigint),
            TypeCategory::F32 => Some(PostgreSQLType::Real),
            TypeCategory::F64 => Some(PostgreSQLType::DoublePrecision),
            TypeCategory::Bool => Some(PostgreSQLType::Boolean),

            // String/text types
            TypeCategory::String => Some(PostgreSQLType::Text),
            TypeCategory::ArrayString => Some(PostgreSQLType::Varchar),
            TypeCategory::CharArray => Some(PostgreSQLType::Char),

            // Binary types
            TypeCategory::Blob | TypeCategory::ByteArray | TypeCategory::ArrayVec => {
                Some(PostgreSQLType::Bytea)
            }

            // UUID
            #[cfg(feature = "uuid")]
            TypeCategory::Uuid => Some(PostgreSQLType::Uuid),
            #[cfg(not(feature = "uuid"))]
            TypeCategory::Uuid => None,

            // JSON
            #[cfg(feature = "serde")]
            TypeCategory::Json => Some(PostgreSQLType::Jsonb),
            #[cfg(not(feature = "serde"))]
            TypeCategory::Json => None,

            // Chrono date/time types
            TypeCategory::NaiveDate => Some(PostgreSQLType::Date),
            TypeCategory::NaiveTime => Some(PostgreSQLType::Time),
            TypeCategory::NaiveDateTime => Some(PostgreSQLType::Timestamp),
            TypeCategory::DateTimeTz => Some(PostgreSQLType::Timestamptz),

            // Time crate types
            TypeCategory::TimeDate => Some(PostgreSQLType::Date),
            TypeCategory::TimeTime => Some(PostgreSQLType::Time),
            TypeCategory::TimePrimitiveDateTime => Some(PostgreSQLType::Timestamp),
            TypeCategory::TimeOffsetDateTime => Some(PostgreSQLType::Timestamptz),

            // Geo types
            #[cfg(feature = "geo-types")]
            TypeCategory::GeoPoint => Some(PostgreSQLType::Point),
            #[cfg(feature = "geo-types")]
            TypeCategory::GeoRect => Some(PostgreSQLType::Box),
            #[cfg(feature = "geo-types")]
            TypeCategory::GeoLineString => Some(PostgreSQLType::Path),
            #[cfg(not(feature = "geo-types"))]
            TypeCategory::GeoPoint | TypeCategory::GeoRect | TypeCategory::GeoLineString => None,

            // Network types
            #[cfg(feature = "cidr")]
            TypeCategory::IpAddr => Some(PostgreSQLType::Inet),
            #[cfg(feature = "cidr")]
            TypeCategory::Cidr => Some(PostgreSQLType::Cidr),
            #[cfg(not(feature = "cidr"))]
            TypeCategory::IpAddr | TypeCategory::Cidr => None,

            // MAC address
            #[cfg(feature = "cidr")]
            TypeCategory::MacAddr => Some(PostgreSQLType::MacAddr),
            #[cfg(not(feature = "cidr"))]
            TypeCategory::MacAddr => None,

            // Bit types
            #[cfg(feature = "bit-vec")]
            TypeCategory::BitVec => Some(PostgreSQLType::Varbit),
            #[cfg(not(feature = "bit-vec"))]
            TypeCategory::BitVec => None,

            // Enums handled separately
            TypeCategory::Enum => None,
            TypeCategory::Unknown => None,
        }
    }

    /// Check if a constraint is valid for this type category.
    #[must_use]
    pub fn is_valid_constraint(&self, constraint: &str) -> bool {
        if constraint.eq_ignore_ascii_case("serial") {
            matches!(self, TypeCategory::I32)
        } else if constraint.eq_ignore_ascii_case("bigserial") {
            matches!(self, TypeCategory::I64)
        } else {
            true // Most constraints are valid for all types
        }
    }
}

// =============================================================================
// PgTypeCategory - SQL type categories for parsing
// =============================================================================

/// PostgreSQL SQL type category for parsing SQL type strings.
///
/// This categorizes SQL type declarations for migration/introspection purposes.
///
/// # Examples
///
/// ```
/// use drizzle_types::postgres::PgTypeCategory;
///
/// assert_eq!(PgTypeCategory::from_sql_type("integer"), PgTypeCategory::Integer);
/// assert_eq!(PgTypeCategory::from_sql_type("varchar(255)"), PgTypeCategory::Varchar);
/// assert_eq!(PgTypeCategory::from_sql_type("serial"), PgTypeCategory::Serial);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PgTypeCategory {
    SmallInt,
    Integer,
    BigInt,
    Numeric,
    Real,
    DoublePrecision,
    Boolean,
    Char,
    Varchar,
    Text,
    Json,
    Jsonb,
    Time,
    TimeTz,
    Timestamp,
    TimestampTz,
    Date,
    Uuid,
    Interval,
    Inet,
    Cidr,
    MacAddr,
    MacAddr8,
    Vector,
    HalfVec,
    SparseVec,
    Bit,
    Point,
    Line,
    Geometry,
    Serial,
    SmallSerial,
    BigSerial,
    Enum,
    Custom,
}

impl PgTypeCategory {
    /// Helper: case-insensitive prefix check
    fn starts_with_ci(s: &str, prefix: &str) -> bool {
        s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
    }

    /// Determine the type category for a SQL type string
    #[must_use]
    pub fn from_sql_type(sql_type: &str) -> Self {
        let normalized = sql_type.trim();

        // Serial types (must check before integer)
        if Self::starts_with_ci(normalized, "smallserial") {
            return Self::SmallSerial;
        }
        if Self::starts_with_ci(normalized, "bigserial") {
            return Self::BigSerial;
        }
        if Self::starts_with_ci(normalized, "serial") {
            return Self::Serial;
        }

        // Integer types
        if Self::starts_with_ci(normalized, "smallint") {
            return Self::SmallInt;
        }
        if Self::starts_with_ci(normalized, "integer") || normalized.eq_ignore_ascii_case("int") {
            return Self::Integer;
        }
        if Self::starts_with_ci(normalized, "bigint") {
            return Self::BigInt;
        }

        // Numeric types
        if Self::starts_with_ci(normalized, "numeric")
            || Self::starts_with_ci(normalized, "decimal")
        {
            return Self::Numeric;
        }
        if Self::starts_with_ci(normalized, "real") {
            return Self::Real;
        }
        if Self::starts_with_ci(normalized, "double") {
            return Self::DoublePrecision;
        }

        // Boolean
        if Self::starts_with_ci(normalized, "boolean") || normalized.eq_ignore_ascii_case("bool") {
            return Self::Boolean;
        }

        // String types (order matters - varchar before char)
        if Self::starts_with_ci(normalized, "varchar")
            || Self::starts_with_ci(normalized, "character varying")
        {
            return Self::Varchar;
        }
        if Self::starts_with_ci(normalized, "char") || Self::starts_with_ci(normalized, "character")
        {
            return Self::Char;
        }
        if Self::starts_with_ci(normalized, "text") {
            return Self::Text;
        }

        // JSON types (jsonb before json)
        if Self::starts_with_ci(normalized, "jsonb") {
            return Self::Jsonb;
        }
        if Self::starts_with_ci(normalized, "json") {
            return Self::Json;
        }

        // Time/Date types - check "with time zone" case-insensitively
        let has_tz = normalized.len() >= 14
            && normalized[normalized.len().saturating_sub(14)..]
                .eq_ignore_ascii_case("with time zone");

        if Self::starts_with_ci(normalized, "timestamp") {
            return if has_tz {
                Self::TimestampTz
            } else {
                Self::Timestamp
            };
        }
        if Self::starts_with_ci(normalized, "time") {
            return if has_tz { Self::TimeTz } else { Self::Time };
        }
        if Self::starts_with_ci(normalized, "date") {
            return Self::Date;
        }
        if Self::starts_with_ci(normalized, "interval") {
            return Self::Interval;
        }

        // Other types
        if Self::starts_with_ci(normalized, "uuid") {
            return Self::Uuid;
        }
        if Self::starts_with_ci(normalized, "inet") {
            return Self::Inet;
        }
        if Self::starts_with_ci(normalized, "cidr") {
            return Self::Cidr;
        }
        if Self::starts_with_ci(normalized, "macaddr8") {
            return Self::MacAddr8;
        }
        if Self::starts_with_ci(normalized, "macaddr") {
            return Self::MacAddr;
        }

        // Vector types
        if Self::starts_with_ci(normalized, "vector") {
            return Self::Vector;
        }
        if Self::starts_with_ci(normalized, "halfvec") {
            return Self::HalfVec;
        }
        if Self::starts_with_ci(normalized, "sparsevec") {
            return Self::SparseVec;
        }

        // Bit types
        if Self::starts_with_ci(normalized, "bit") {
            return Self::Bit;
        }

        // Geometric types
        if Self::starts_with_ci(normalized, "point") {
            return Self::Point;
        }
        if Self::starts_with_ci(normalized, "line") {
            return Self::Line;
        }
        if Self::starts_with_ci(normalized, "geometry") {
            return Self::Geometry;
        }

        Self::Custom
    }

    /// Get the drizzle import name for this type
    #[must_use]
    pub const fn drizzle_import(&self) -> &'static str {
        match self {
            Self::SmallInt => "smallint",
            Self::Integer => "integer",
            Self::BigInt => "bigint",
            Self::Numeric => "numeric",
            Self::Real => "real",
            Self::DoublePrecision => "doublePrecision",
            Self::Boolean => "boolean",
            Self::Char => "char",
            Self::Varchar => "varchar",
            Self::Text => "text",
            Self::Json => "json",
            Self::Jsonb => "jsonb",
            Self::Time | Self::TimeTz => "time",
            Self::Timestamp | Self::TimestampTz => "timestamp",
            Self::Date => "date",
            Self::Uuid => "uuid",
            Self::Interval => "interval",
            Self::Inet => "inet",
            Self::Cidr => "cidr",
            Self::MacAddr => "macaddr",
            Self::MacAddr8 => "macaddr8",
            Self::Vector => "vector",
            Self::HalfVec => "halfvec",
            Self::SparseVec => "sparsevec",
            Self::Bit => "bit",
            Self::Point => "point",
            Self::Line => "line",
            Self::Geometry => "geometry",
            Self::Serial => "serial",
            Self::SmallSerial => "smallserial",
            Self::BigSerial => "bigserial",
            Self::Enum => "pgEnum",
            Self::Custom => "customType",
        }
    }

    /// Check if this is a serial type
    #[must_use]
    pub const fn is_serial(&self) -> bool {
        matches!(self, Self::Serial | Self::SmallSerial | Self::BigSerial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_category_from_string() {
        assert_eq!(
            TypeCategory::from_type_string("String"),
            TypeCategory::String
        );
        assert_eq!(TypeCategory::from_type_string("i16"), TypeCategory::I16);
        assert_eq!(TypeCategory::from_type_string("i32"), TypeCategory::I32);
        assert_eq!(TypeCategory::from_type_string("i64"), TypeCategory::I64);
        assert_eq!(TypeCategory::from_type_string("f32"), TypeCategory::F32);
        assert_eq!(TypeCategory::from_type_string("f64"), TypeCategory::F64);
        assert_eq!(TypeCategory::from_type_string("bool"), TypeCategory::Bool);
        assert_eq!(
            TypeCategory::from_type_string("Vec<u8>"),
            TypeCategory::Blob
        );
        assert_eq!(TypeCategory::from_type_string("Uuid"), TypeCategory::Uuid);
        assert_eq!(
            TypeCategory::from_type_string("Option<String>"),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_string("NaiveDateTime"),
            TypeCategory::NaiveDateTime
        );
    }

    #[test]
    fn test_pg_type_category_from_sql_type() {
        assert_eq!(
            PgTypeCategory::from_sql_type("integer"),
            PgTypeCategory::Integer
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("VARCHAR(255)"),
            PgTypeCategory::Varchar
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("serial"),
            PgTypeCategory::Serial
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("timestamp with time zone"),
            PgTypeCategory::TimestampTz
        );
        assert_eq!(PgTypeCategory::from_sql_type("uuid"), PgTypeCategory::Uuid);
        assert_eq!(
            PgTypeCategory::from_sql_type("jsonb"),
            PgTypeCategory::Jsonb
        );
    }

    #[test]
    fn test_pg_type_category_is_serial() {
        assert!(PgTypeCategory::Serial.is_serial());
        assert!(PgTypeCategory::SmallSerial.is_serial());
        assert!(PgTypeCategory::BigSerial.is_serial());
        assert!(!PgTypeCategory::Integer.is_serial());
    }
}
