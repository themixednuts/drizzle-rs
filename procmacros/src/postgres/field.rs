use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::{collections::HashSet, fmt::Display};
use syn::{Attribute, Error, Expr, ExprPath, Field, Ident, Lit, Meta, Result, Token, Type};

// =============================================================================
// Type Category - Centralized type classification for code generation
// =============================================================================

/// Categorizes Rust types for consistent handling across the macro system.
///
/// This enum provides a single source of truth for type detection, eliminating
/// fragile string matching scattered across multiple files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeCategory {
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
    pub(crate) fn from_type_string(type_str: &str) -> Self {
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
        // cidr::IpInet -> INET (host address with optional netmask)
        // cidr::IpCidr -> CIDR (network specification)
        // std::net::IpAddr also supported
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
    /// Returns None for Unknown types (should trigger compile error).
    pub(crate) fn to_postgres_type(&self) -> Option<PostgreSQLType> {
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
    pub(crate) fn is_valid_constraint(&self, constraint: &str) -> bool {
        match constraint {
            "serial" => matches!(self, TypeCategory::I32),
            "bigserial" => matches!(self, TypeCategory::I64),
            "primary" | "unique" | "not_null" | "check" | "references" | "default"
            | "default_fn" => true,
            _ => false,
        }
    }
}

/// Enum representing supported PostgreSQL column types.
///
/// These correspond to PostgreSQL data types.
/// See: <https://www.postgresql.org/docs/current/datatype.html>
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum PostgreSQLType {
    /// PostgreSQL INTEGER type - 32-bit signed integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
    Integer,

    /// PostgreSQL BIGINT type - 64-bit signed integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
    Bigint,

    /// PostgreSQL SMALLINT type - 16-bit signed integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
    Smallint,

    /// PostgreSQL SERIAL type - auto-incrementing 32-bit integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
    Serial,

    /// PostgreSQL BIGSERIAL type - auto-incrementing 64-bit integer
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
    Bigserial,

    /// PostgreSQL TEXT type - variable-length character string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-character.html>
    #[default]
    Text,

    /// PostgreSQL VARCHAR type - variable-length character string with limit
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-character.html>
    Varchar,

    /// PostgreSQL CHAR type - fixed-length character string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-character.html>
    Char,

    /// PostgreSQL REAL type - single precision floating-point number
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
    Real,

    /// PostgreSQL DOUBLE PRECISION type - double precision floating-point number
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
    DoublePrecision,

    /// PostgreSQL NUMERIC type - exact numeric with selectable precision
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-NUMERIC-DECIMAL>
    Numeric,

    /// PostgreSQL BOOLEAN type - true/false
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-boolean.html>
    Boolean,

    /// PostgreSQL BYTEA type - binary data
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-binary.html>
    Bytea,

    /// PostgreSQL UUID type - universally unique identifier
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-uuid.html>
    #[cfg(feature = "uuid")]
    Uuid,

    /// PostgreSQL JSON type - JSON data
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-json.html>
    #[cfg(feature = "serde")]
    Json,

    /// PostgreSQL JSONB type - binary JSON data
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-json.html>
    #[cfg(feature = "serde")]
    Jsonb,

    /// PostgreSQL TIMESTAMP type - date and time
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Timestamp,

    /// PostgreSQL TIMESTAMPTZ type - date and time with time zone
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Timestamptz,

    /// PostgreSQL DATE type - calendar date
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Date,

    /// PostgreSQL TIME type - time of day
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Time,

    /// PostgreSQL TIMETZ type - time of day with time zone
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    Timetz,

    /// PostgreSQL INTERVAL type - time interval
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
    #[cfg(feature = "chrono")]
    Interval,

    /// PostgreSQL INET type - IPv4 or IPv6 host address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    Inet,

    /// PostgreSQL CIDR type - IPv4 or IPv6 network address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    Cidr,

    /// PostgreSQL MACADDR type - MAC address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    MacAddr,

    /// PostgreSQL MACADDR8 type - EUI-64 MAC address
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-net-types.html>
    #[cfg(feature = "cidr")]
    MacAddr8,

    /// PostgreSQL POINT type - geometric point
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Point,

    /// PostgreSQL LINE type - geometric line (infinite)
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Line,

    /// PostgreSQL LSEG type - geometric line segment
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Lseg,

    /// PostgreSQL BOX type - geometric box
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Box,

    /// PostgreSQL PATH type - geometric path
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Path,

    /// PostgreSQL POLYGON type - geometric polygon
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Polygon,

    /// PostgreSQL CIRCLE type - geometric circle
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-geometric.html>
    #[cfg(feature = "geo-types")]
    Circle,

    /// PostgreSQL BIT type - fixed-length bit string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-bit.html>
    #[cfg(feature = "bit-vec")]
    Bit,

    /// PostgreSQL BIT VARYING type - variable-length bit string
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-bit.html>
    #[cfg(feature = "bit-vec")]
    Varbit,

    /// PostgreSQL custom ENUM type - user-defined enumerated type
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-enum.html>
    Enum(String), // The enum type name
}

impl Display for PostgreSQLType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql_type())
    }
}

impl PostgreSQLType {
    /// Convert from attribute name to enum variant
    /// For native enums, use `from_enum_attribute` instead
    pub(crate) fn from_attribute_name(name: &str) -> Option<Self> {
        match name {
            // Integer types and aliases
            "integer" | "int" | "int4" => Some(Self::Integer),
            "bigint" | "int8" => Some(Self::Bigint),
            "smallint" | "int2" => Some(Self::Smallint),
            "serial" | "serial4" => Some(Self::Serial),
            "bigserial" | "serial8" => Some(Self::Bigserial),

            // Text types and aliases
            "text" => Some(Self::Text),
            "varchar" | "character_varying" => Some(Self::Varchar),
            "char" | "character" => Some(Self::Char),

            // Float types and aliases
            "real" | "float4" => Some(Self::Real),
            "double_precision" | "float8" | "double" => Some(Self::DoublePrecision),
            "numeric" | "decimal" => Some(Self::Numeric),

            // Other basic types
            "boolean" | "bool" => Some(Self::Boolean),
            "bytea" => Some(Self::Bytea),

            // UUID
            #[cfg(feature = "uuid")]
            "uuid" => Some(Self::Uuid),

            // JSON types
            #[cfg(feature = "serde")]
            "json" => Some(Self::Json),
            #[cfg(feature = "serde")]
            "jsonb" => Some(Self::Jsonb),

            // Date/time types and aliases
            "timestamp" | "timestamp_without_time_zone" => Some(Self::Timestamp),
            "timestamptz" | "timestamp_with_time_zone" => Some(Self::Timestamptz),
            "date" => Some(Self::Date),
            "time" | "time_without_time_zone" => Some(Self::Time),
            "timetz" | "time_with_time_zone" => Some(Self::Timetz),
            #[cfg(feature = "chrono")]
            "interval" => Some(Self::Interval),

            // Network address types
            #[cfg(feature = "cidr")]
            "inet" => Some(Self::Inet),
            #[cfg(feature = "cidr")]
            "cidr" => Some(Self::Cidr),
            #[cfg(feature = "cidr")]
            "macaddr" => Some(Self::MacAddr),
            #[cfg(feature = "cidr")]
            "macaddr8" => Some(Self::MacAddr8),

            // Geometric types
            #[cfg(feature = "geo-types")]
            "point" => Some(Self::Point),
            #[cfg(feature = "geo-types")]
            "line" => Some(Self::Line),
            #[cfg(feature = "geo-types")]
            "lseg" => Some(Self::Lseg),
            #[cfg(feature = "geo-types")]
            "box" => Some(Self::Box),
            #[cfg(feature = "geo-types")]
            "path" => Some(Self::Path),
            #[cfg(feature = "geo-types")]
            "polygon" => Some(Self::Polygon),
            #[cfg(feature = "geo-types")]
            "circle" => Some(Self::Circle),

            // Bit string types
            #[cfg(feature = "bit-vec")]
            "bit" => Some(Self::Bit),
            #[cfg(feature = "bit-vec")]
            "varbit" | "bit_varying" => Some(Self::Varbit),

            "enum" => None, // enum() requires a parameter, handled separately
            _ => None,
        }
    }

    /// Create a native PostgreSQL enum type from enum attribute
    /// Used for #[enum(MyEnum)] syntax
    pub(crate) fn from_enum_attribute(enum_name: &str) -> Self {
        Self::Enum(enum_name.to_string())
    }

    /// Get the SQL type string for this type
    pub(crate) fn to_sql_type(&self) -> &str {
        match self {
            Self::Integer => "INTEGER",
            Self::Bigint => "BIGINT",
            Self::Smallint => "SMALLINT",
            Self::Serial => "SERIAL",
            Self::Bigserial => "BIGSERIAL",
            Self::Text => "TEXT",
            Self::Varchar => "VARCHAR",
            Self::Char => "CHAR",
            Self::Real => "REAL",
            Self::DoublePrecision => "DOUBLE PRECISION",
            Self::Numeric => "NUMERIC",
            Self::Boolean => "BOOLEAN",
            Self::Bytea => "BYTEA",
            #[cfg(feature = "uuid")]
            Self::Uuid => "UUID",
            #[cfg(feature = "serde")]
            Self::Json => "JSON",
            #[cfg(feature = "serde")]
            Self::Jsonb => "JSONB",
            Self::Timestamp => "TIMESTAMP",
            Self::Timestamptz => "TIMESTAMPTZ",
            Self::Date => "DATE",
            Self::Time => "TIME",
            Self::Timetz => "TIMETZ",
            #[cfg(feature = "chrono")]
            Self::Interval => "INTERVAL",
            #[cfg(feature = "cidr")]
            Self::Inet => "INET",
            #[cfg(feature = "cidr")]
            Self::Cidr => "CIDR",
            #[cfg(feature = "cidr")]
            Self::MacAddr => "MACADDR",
            #[cfg(feature = "cidr")]
            Self::MacAddr8 => "MACADDR8",
            #[cfg(feature = "geo-types")]
            Self::Point => "POINT",
            #[cfg(feature = "geo-types")]
            Self::Line => "LINE",
            #[cfg(feature = "geo-types")]
            Self::Lseg => "LSEG",
            #[cfg(feature = "geo-types")]
            Self::Box => "BOX",
            #[cfg(feature = "geo-types")]
            Self::Path => "PATH",
            #[cfg(feature = "geo-types")]
            Self::Polygon => "POLYGON",
            #[cfg(feature = "geo-types")]
            Self::Circle => "CIRCLE",
            #[cfg(feature = "bit-vec")]
            Self::Bit => "BIT",
            #[cfg(feature = "bit-vec")]
            Self::Varbit => "VARBIT",
            Self::Enum(name) => name.as_str(), // Custom enum type name
        }
    }

    /// Check if a flag is valid for this column type
    pub(crate) fn is_valid_flag(&self, flag: &str) -> bool {
        match (self, flag) {
            (Self::Serial | Self::Bigserial, "generated_identity") => true,
            (Self::Text | Self::Bytea, "json") => true,
            #[cfg(feature = "serde")]
            (Self::Json | Self::Jsonb, "json") => true,
            (Self::Text | Self::Integer | Self::Smallint | Self::Bigint, "enum") => true,
            (Self::Enum(_), "enum") => true, // Native PostgreSQL enums support enum flag
            (_, "primary" | "primary_key" | "unique" | "not_null" | "check") => true,
            _ => false,
        }
    }

    /// Validate a flag for this column type, returning an error with PostgreSQL docs link if invalid.
    pub(crate) fn validate_flag(&self, flag: &str, span: proc_macro2::Span) -> Result<()> {
        if !self.is_valid_flag(flag) {
            let message = match (self, flag) {
                (non_serial, "generated_identity")
                    if !matches!(non_serial, Self::Serial | Self::Bigserial) =>
                {
                    "generated_identity can only be used with SERIAL or BIGSERIAL columns. \
                        See: https://www.postgresql.org/docs/current/ddl-identity-columns.html"
                        .to_string()
                }
                (non_text_or_binary, "json") => {
                    #[cfg(feature = "serde")]
                    let supports_json = matches!(
                        non_text_or_binary,
                        Self::Text | Self::Bytea | Self::Json | Self::Jsonb
                    );
                    #[cfg(not(feature = "serde"))]
                    let supports_json = matches!(non_text_or_binary, Self::Text | Self::Bytea);

                    if !supports_json {
                        "json can only be used with TEXT, BYTEA, JSON, or JSONB columns. \
                            See: https://www.postgresql.org/docs/current/datatype-json.html"
                            .to_string()
                    } else {
                        return Ok(());
                    }
                }
                (non_enum_compatible, "enum")
                    if !matches!(
                        non_enum_compatible,
                        Self::Text | Self::Integer | Self::Smallint | Self::Bigint | Self::Enum(_)
                    ) =>
                {
                    "enum can only be used with TEXT, INTEGER, SMALLINT, BIGINT, or native ENUM columns. \
                        For custom enum types, see: https://www.postgresql.org/docs/current/datatype-enum.html"
                        .to_string()
                }
                _ => format!("'{flag}' is not valid for {} columns", self.to_sql_type()),
            };

            return Err(Error::new(span, message));
        }
        Ok(())
    }

    /// Check if this type is an auto-incrementing type
    pub(crate) fn is_serial(&self) -> bool {
        matches!(self, Self::Serial | Self::Bigserial)
    }

    /// Check if this type supports primary keys
    pub(crate) fn supports_primary_key(&self) -> bool {
        #[cfg(feature = "serde")]
        {
            !matches!(self, Self::Json | Self::Jsonb)
        }
        #[cfg(not(feature = "serde"))]
        {
            true
        }
    }
}

/// PostgreSQL column constraint flags
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PostgreSQLFlag {
    Primary,
    Unique,
    NotNull,
    GeneratedIdentity,
    /// Used with TEXT/INTEGER columns to store enum as text/discriminant
    Enum,
    /// Used with native PostgreSQL ENUM types - references the enum type name
    NativeEnum(String),
    Json,
    Check(String),
}

impl PostgreSQLFlag {
    /// Parse a flag from its string name and optional value
    pub(crate) fn from_name_and_value(name: &str, value: Option<&Expr>) -> Result<Self> {
        match name {
            "primary" | "primary_key" => Ok(Self::Primary),
            "unique" => Ok(Self::Unique),
            "not_null" => Ok(Self::NotNull),
            "generated_identity" => Ok(Self::GeneratedIdentity),
            "enum" => Ok(Self::Enum),
            "json" => Ok(Self::Json),
            "check" => {
                if let Some(expr) = value {
                    if let Expr::Lit(syn::ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }) = expr
                    {
                        Ok(Self::Check(lit_str.value()))
                    } else {
                        Err(Error::new_spanned(
                            expr,
                            "check constraint must be a string literal",
                        ))
                    }
                } else {
                    Err(Error::new_spanned(
                        name,
                        "check constraint requires a value",
                    ))
                }
            }
            _ => Err(Error::new_spanned(
                name,
                format!("Unknown PostgreSQL flag: {}", name),
            )),
        }
    }

    /// Convert flag to SQL string
    pub(crate) fn to_sql(&self) -> String {
        match self {
            Self::Primary => "PRIMARY KEY".to_string(),
            Self::Unique => "UNIQUE".to_string(),
            Self::NotNull => "NOT NULL".to_string(),
            Self::GeneratedIdentity => "GENERATED ALWAYS AS IDENTITY".to_string(),
            Self::Enum => String::new(), // Handled separately in type conversion
            Self::NativeEnum(_) => String::new(), // Type already specifies the enum name
            Self::Json => String::new(), // Handled separately in type conversion
            Self::Check(constraint) => format!("CHECK ({})", constraint),
        }
    }
}

/// Default value specification for PostgreSQL columns
#[derive(Debug, Clone)]
pub(crate) enum PostgreSQLDefault {
    /// Literal value (e.g., 'default_value')
    Literal(String),
    /// Function call (e.g., NOW())
    Function(String),
    /// Expression using Rust code (evaluated at compile time)
    Expression(TokenStream),
}

/// References specification for PostgreSQL foreign keys
#[derive(Debug, Clone)]
pub(crate) struct PostgreSQLReference {
    pub table: Ident,
    pub column: Ident,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Information about a PostgreSQL table field
#[derive(Clone)]
pub(crate) struct FieldInfo {
    pub ident: Ident,
    pub vis: syn::Visibility,
    /// The original field type (e.g., Option<String> or i32)
    pub field_type: Type,
    /// The base type with Option<> unwrapped (e.g., String from Option<String>)
    pub base_type: Type,
    /// The column name in the database (defaults to field ident, can be overridden)
    pub column_name: String,
    /// SQL column definition string (e.g., "name TEXT NOT NULL")
    pub sql_definition: String,
    pub column_type: PostgreSQLType,
    pub flags: HashSet<PostgreSQLFlag>,
    pub is_primary: bool,
    pub is_unique: bool,
    pub is_nullable: bool,
    pub is_enum: bool,
    pub is_pgenum: bool,
    pub is_json: bool,
    pub is_serial: bool,
    pub default: Option<PostgreSQLDefault>,
    pub default_fn: Option<TokenStream>,
    pub check_constraint: Option<String>,
    pub foreign_key: Option<PostgreSQLReference>,
    pub has_default: bool,
    pub marker_exprs: Vec<syn::ExprPath>,
}

impl FieldInfo {
    /// Create an ExprPath with an UPPERCASE ident but preserving the original span.
    ///
    /// This allows users to write `#[column(primary)]` (lowercase) but the generated
    /// code references `PRIMARY` (uppercase, resolves to prelude). The preserved span
    /// enables IDE hover documentation by linking back to the user's source.
    fn make_uppercase_path(original_ident: &syn::Ident, uppercase_name: &str) -> syn::ExprPath {
        let new_ident = syn::Ident::new(uppercase_name, original_ident.span());
        syn::ExprPath {
            attrs: vec![],
            qself: None,
            path: new_ident.into(),
        }
    }

    /// Parse field information from a struct field.
    ///
    /// The PostgreSQL type is INFERRED from the Rust type, not from attributes.
    /// Attributes are only used for constraints (primary, unique, etc.).
    pub(crate) fn from_field(field: &Field, is_composite_pk: bool) -> Result<Self> {
        let name = field.ident.as_ref().unwrap().clone();
        let vis = field.vis.clone();
        let ty = field.ty.clone();

        // Check if field is nullable (wrapped in Option<T>)
        let is_nullable = Self::is_option_type(&ty);

        // Infer PostgreSQL type from Rust type
        let type_str = ty.to_token_stream().to_string();
        let type_category = TypeCategory::from_type_string(&type_str);

        // Initialize constraint-related fields
        let mut flags = HashSet::new();
        let mut default = None;
        let mut default_fn = None;
        let mut check_constraint = None;
        let mut foreign_key = None;
        let mut is_serial = false;
        let mut is_bigserial = false;
        let mut is_pgenum = false;
        let mut enum_type_name: Option<String> = None;
        let mut marker_exprs = Vec::new();

        // Parse #[column(...)] attributes for constraints
        for attr in &field.attrs {
            if let Some(column_info) =
                Self::parse_column_attribute(attr, &type_category, name.span())?
            {
                flags = column_info.flags;
                default = column_info.default;
                default_fn = column_info.default_fn;
                check_constraint = column_info.check_constraint;
                foreign_key = column_info.foreign_key;
                is_serial = column_info.is_serial;
                is_bigserial = column_info.is_bigserial;
                is_pgenum = column_info.is_pgenum;
                enum_type_name = column_info.enum_type_name;
                marker_exprs = column_info.marker_exprs;
                break;
            }
        }

        // Determine the PostgreSQL column type
        let column_type = if is_serial {
            PostgreSQLType::Serial
        } else if is_bigserial {
            PostgreSQLType::Bigserial
        } else if is_pgenum {
            // Get the enum type name from the field's base type
            let base_type = Self::extract_option_inner(&ty);
            let base_type_str = base_type.to_token_stream().to_string().replace(' ', "");
            PostgreSQLType::from_enum_attribute(&base_type_str)
        } else {
            // Infer from Rust type
            type_category.to_postgres_type().ok_or_else(|| {
                Error::new(
                    name.span(),
                    format!(
                        "Cannot infer PostgreSQL type for Rust type '{}'. \
                        Use a supported type or add #[column(enum)] for enum types.",
                        type_str
                    ),
                )
            })?
        };

        // Apply flags from type category
        if is_pgenum {
            let base_type = Self::extract_option_inner(&ty);
            let base_type_str = base_type.to_token_stream().to_string().replace(' ', "");
            flags.insert(PostgreSQLFlag::NativeEnum(base_type_str));
        }

        let is_primary = flags.contains(&PostgreSQLFlag::Primary);
        let is_unique = flags.contains(&PostgreSQLFlag::Unique);
        let is_enum = flags.contains(&PostgreSQLFlag::Enum);
        let is_json = matches!(type_category, TypeCategory::Json);
        let is_serial_type = is_serial || is_bigserial;
        let has_default = default.is_some() || default_fn.is_some() || is_serial_type;

        // Compute base_type once and store it
        let base_type = Self::extract_option_inner(&ty).clone();

        // Column name defaults to field ident (can be overridden in future with name attribute)
        let column_name = name.to_string();

        // Build SQL definition for this column
        let sql_definition = build_sql_definition(
            &column_name,
            &column_type,
            is_primary && !is_composite_pk,
            !is_nullable,
            is_unique,
            is_serial || is_bigserial,
            &default,
            &check_constraint,
        );

        Ok(FieldInfo {
            ident: name,
            vis,
            field_type: ty,
            base_type,
            column_name,
            sql_definition,
            column_type,
            flags,
            is_primary,
            is_unique,
            is_nullable,
            is_enum,
            is_pgenum,
            is_json,
            is_serial: is_serial_type,
            default,
            default_fn,
            check_constraint,
            foreign_key,
            has_default,
            marker_exprs,
        })
    }

    /// Check if a type is Option<T>
    fn is_option_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty
            && let Some(segment) = type_path.path.segments.last()
        {
            return segment.ident == "Option";
        }
        false
    }

    /// Extract the inner type from Option<T>, returning T
    /// If the type is not Option<T>, returns the original type
    fn extract_option_inner(ty: &Type) -> &Type {
        if let Type::Path(type_path) = ty
            && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return inner;
        }
        ty
    }

    // base_type is now a field, not a method - see struct definition

    /// Parse #[column(...)] attribute for constraints.
    ///
    /// The PostgreSQL type is NOT determined by attributes anymore - it's inferred
    /// from the Rust type. This method only parses constraints like primary, unique, etc.
    fn parse_column_attribute(
        attr: &Attribute,
        type_category: &TypeCategory,
        span: proc_macro2::Span,
    ) -> Result<Option<ColumnInfo>> {
        // Only process #[column(...)] attributes
        let attr_name = if let Some(ident) = attr.path().get_ident() {
            ident.to_string()
        } else {
            return Ok(None);
        };

        if attr_name != "column" {
            return Ok(None);
        }

        let mut flags = HashSet::new();
        let mut default = None;
        let mut default_fn = None;
        let mut check_constraint = None;
        let mut foreign_key = None;
        let mut is_serial = false;
        let mut is_bigserial = false;
        let mut is_pgenum = false;
        let mut enum_type_name: Option<String> = None;
        let mut marker_exprs = Vec::new();

        // Parse attribute arguments: #[column(primary, unique, default = "foo")]
        if attr.meta.require_list().is_ok() {
            attr.parse_nested_meta(|meta| {
                let path_ident = meta
                    .path
                    .get_ident()
                    .ok_or_else(|| syn::Error::new_spanned(&meta.path, "Expected identifier"))?;
                // Convert to uppercase for case-insensitive matching
                let path = path_ident.to_string().to_ascii_uppercase();

                match path.as_str() {
                    "SERIAL" => {
                        // Validate: serial only valid on i32
                        if !type_category.is_valid_constraint("serial") {
                            return Err(syn::Error::new(
                                span,
                                "serial constraint requires field type i32",
                            ));
                        }
                        is_serial = true;
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "SERIAL"));
                    }
                    "BIGSERIAL" => {
                        // Validate: bigserial only valid on i64
                        if !type_category.is_valid_constraint("bigserial") {
                            return Err(syn::Error::new(
                                span,
                                "bigserial constraint requires field type i64",
                            ));
                        }
                        is_bigserial = true;
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "BIGSERIAL"));
                    }
                    "SMALLSERIAL" => {
                        // Validate: smallserial only valid on i16
                        if !type_category.is_valid_constraint("smallserial") {
                            return Err(syn::Error::new(
                                span,
                                "smallserial constraint requires field type i16",
                            ));
                        }
                        is_serial = true; // Treat as serial for now
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "SMALLSERIAL"));
                    }
                    "PRIMARY" | "PRIMARY_KEY" => {
                        flags.insert(PostgreSQLFlag::Primary);
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "PRIMARY"));
                    }
                    "UNIQUE" => {
                        flags.insert(PostgreSQLFlag::Unique);
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "UNIQUE"));
                    }
                    "GENERATED_IDENTITY" => {
                        flags.insert(PostgreSQLFlag::GeneratedIdentity);
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "GENERATED_IDENTITY"));
                    }
                    "JSON" => {
                        // Mark as JSON type
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "JSON"));
                    }
                    "JSONB" => {
                        // Mark as JSONB type
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "JSONB"));
                    }
                    "ENUM" => {
                        // Just mark as pgenum - the type is inferred from the field definition
                        is_pgenum = true;
                        marker_exprs.push(Self::make_uppercase_path(path_ident, "ENUM"));
                    }
                    "DEFAULT" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let lit: Lit = meta.input.parse()?;
                            match lit {
                                Lit::Str(s) => {
                                    default = Some(PostgreSQLDefault::Literal(s.value()))
                                }
                                Lit::Int(i) => {
                                    default = Some(PostgreSQLDefault::Literal(i.to_string()))
                                }
                                Lit::Float(f) => {
                                    default = Some(PostgreSQLDefault::Literal(f.to_string()))
                                }
                                Lit::Bool(b) => {
                                    default = Some(PostgreSQLDefault::Literal(b.value.to_string()))
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        lit,
                                        "Unsupported default literal type",
                                    ));
                                }
                            }
                            marker_exprs.push(Self::make_uppercase_path(path_ident, "DEFAULT"));
                        }
                    }
                    "DEFAULT_FN" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let expr: Expr = meta.input.parse()?;
                            default_fn = Some(quote! { #expr });
                            marker_exprs.push(Self::make_uppercase_path(path_ident, "DEFAULT_FN"));
                        }
                    }
                    "CHECK" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let lit: Lit = meta.input.parse()?;
                            if let Lit::Str(s) = lit {
                                check_constraint = Some(s.value());
                                flags.insert(PostgreSQLFlag::Check(s.value()));
                                marker_exprs.push(Self::make_uppercase_path(path_ident, "CHECK"));
                            }
                        }
                    }
                    "REFERENCES" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let path: ExprPath = meta.input.parse()?;
                            foreign_key = Some(Self::parse_reference(&path)?);
                            marker_exprs.push(Self::make_uppercase_path(path_ident, "REFERENCES"));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &meta.path,
                            format!("Unknown column constraint: '{}'. Supported: PRIMARY, UNIQUE, SERIAL, BIGSERIAL, SMALLSERIAL, GENERATED_IDENTITY, JSON, JSONB, ENUM, DEFAULT, DEFAULT_FN, CHECK, REFERENCES", path_ident),
                        ));
                    }
                }
                Ok(())
            })?;
        }

        Ok(Some(ColumnInfo {
            flags,
            default,
            default_fn,
            check_constraint,
            foreign_key,
            is_serial,
            is_bigserial,
            is_pgenum,
            enum_type_name,
            marker_exprs,
        }))
    }

    /// Parse foreign key reference from path expression
    fn parse_reference(path: &ExprPath) -> Result<PostgreSQLReference> {
        // Convert ExprPath to string format like "Users::id"

        if !path.path.segments.len().eq(&2) {
            return Err(Error::new_spanned(
                path,
                "References must be in the format Table::column",
            ));
        }

        let table = path
            .path
            .segments
            .first()
            .ok_or(Error::new_spanned(
                path,
                "References must be in the format Table::column",
            ))?
            .ident
            .clone();
        let column = path
            .path
            .segments
            .last()
            .ok_or(Error::new_spanned(
                path,
                "References must be in the format Table::column",
            ))?
            .ident
            .clone();

        Ok(PostgreSQLReference {
            table,
            column,
            on_delete: None, // TODO: Add support for ON DELETE/UPDATE actions
            on_update: None,
        })
    }
}

impl FieldInfo {
    /// Get the category of this field's type for code generation decisions.
    ///
    /// This provides a single source of truth for type handling, eliminating
    /// scattered string matching throughout the codebase.
    pub(crate) fn type_category(&self) -> TypeCategory {
        // Special flags take precedence
        if self.is_json {
            return TypeCategory::Json;
        }
        if self.is_enum || self.is_pgenum {
            return TypeCategory::Enum;
        }

        // Detect from the base type string
        let type_str = self.field_type.to_token_stream().to_string();
        TypeCategory::from_type_string(&type_str)
    }
}

/// Build SQL column definition string for PostgreSQL
fn build_sql_definition(
    column_name: &str,
    column_type: &PostgreSQLType,
    is_primary_single: bool,
    is_not_null: bool,
    is_unique: bool,
    is_serial: bool,
    default: &Option<PostgreSQLDefault>,
    check_constraint: &Option<String>,
) -> String {
    let mut sql = format!("{} {}", column_name, column_type.to_sql_type());

    // Handle primary key
    if is_primary_single {
        sql.push_str(" PRIMARY KEY");
    }

    // Add NOT NULL constraint (serial types are implicitly NOT NULL)
    if is_not_null && !is_serial {
        sql.push_str(" NOT NULL");
    }

    // Add UNIQUE constraint
    if is_unique && !is_primary_single {
        sql.push_str(" UNIQUE");
    }

    // Add DEFAULT value if present
    if let Some(default_value) = default {
        match default_value {
            PostgreSQLDefault::Literal(lit) => {
                sql.push_str(&format!(" DEFAULT {}", lit));
            }
            PostgreSQLDefault::Function(func) => {
                sql.push_str(&format!(" DEFAULT {}", func));
            }
            PostgreSQLDefault::Expression(_) => {
                // Expression defaults are handled at runtime
            }
        }
    }

    // Add CHECK constraint
    if let Some(check) = check_constraint {
        sql.push_str(&format!(" CHECK ({})", check));
    }

    sql
}

/// Intermediate structure for parsing column constraint information
struct ColumnInfo {
    flags: HashSet<PostgreSQLFlag>,
    default: Option<PostgreSQLDefault>,
    default_fn: Option<TokenStream>,
    check_constraint: Option<String>,
    foreign_key: Option<PostgreSQLReference>,
    is_serial: bool,
    is_bigserial: bool,
    is_pgenum: bool,
    enum_type_name: Option<String>,
    marker_exprs: Vec<syn::ExprPath>,
}
