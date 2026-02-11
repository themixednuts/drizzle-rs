use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::{collections::HashSet, fmt::Display};
use syn::{Attribute, Error, Expr, ExprPath, Field, Ident, Lit, Result, Token, Type};

use crate::common::make_uppercase_path;
use crate::common::{
    is_option_type, option_inner_type, references_required_message, type_is_array_char,
    type_is_array_string, type_is_array_u8, type_is_arrayvec_u8, type_is_bit_vec, type_is_bool,
    type_is_datetime_tz, type_is_float, type_is_geo_linestring, type_is_geo_point,
    type_is_geo_rect, type_is_int, type_is_ip_addr, type_is_ip_cidr, type_is_json_value,
    type_is_mac_addr, type_is_naive_date, type_is_naive_datetime, type_is_naive_time,
    type_is_offset_datetime, type_is_primitive_date_time, type_is_string_like, type_is_time_date,
    type_is_time_time, type_is_uuid, type_is_vec_u8, unwrap_option,
};

// Note: drizzle_types::postgres::TypeCategory exists but has different feature gates.
// The local TypeCategory is kept for now to maintain feature flag consistency.

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
    /// Detect the category from a `syn::Type` without stringification.
    ///
    /// Order matters: more specific types (ArrayString) must be checked
    /// before more general types (String).
    pub(crate) fn from_type(ty: &Type) -> Self {
        let ty = unwrap_option(ty);

        if type_is_array_u8(ty) {
            return TypeCategory::ByteArray;
        }
        if type_is_array_char(ty) {
            return TypeCategory::CharArray;
        }
        if type_is_array_string(ty) {
            return TypeCategory::ArrayString;
        }
        if type_is_arrayvec_u8(ty) {
            return TypeCategory::ArrayVec;
        }
        if type_is_uuid(ty) {
            return TypeCategory::Uuid;
        }
        if type_is_json_value(ty) {
            return TypeCategory::Json;
        }

        if type_is_naive_date(ty) {
            return TypeCategory::NaiveDate;
        }
        if type_is_naive_time(ty) {
            return TypeCategory::NaiveTime;
        }
        if type_is_naive_datetime(ty) {
            return TypeCategory::NaiveDateTime;
        }
        if type_is_datetime_tz(ty) {
            return TypeCategory::DateTimeTz;
        }

        if type_is_time_date(ty) {
            return TypeCategory::TimeDate;
        }
        if type_is_time_time(ty) {
            return TypeCategory::TimeTime;
        }
        if type_is_primitive_date_time(ty) {
            return TypeCategory::TimePrimitiveDateTime;
        }
        if type_is_offset_datetime(ty) {
            return TypeCategory::TimeOffsetDateTime;
        }

        if type_is_geo_point(ty) {
            return TypeCategory::GeoPoint;
        }
        if type_is_geo_rect(ty) {
            return TypeCategory::GeoRect;
        }
        if type_is_geo_linestring(ty) {
            return TypeCategory::GeoLineString;
        }

        if type_is_ip_addr(ty) {
            return TypeCategory::IpAddr;
        }
        if type_is_ip_cidr(ty) {
            return TypeCategory::Cidr;
        }

        if type_is_mac_addr(ty) {
            return TypeCategory::MacAddr;
        }
        if type_is_bit_vec(ty) {
            return TypeCategory::BitVec;
        }

        if type_is_string_like(ty) {
            return TypeCategory::String;
        }
        if type_is_vec_u8(ty) {
            return TypeCategory::Blob;
        }

        if type_is_int(ty, "i16") {
            return TypeCategory::I16;
        }
        if type_is_int(ty, "i32") {
            return TypeCategory::I32;
        }
        if type_is_int(ty, "i64") {
            return TypeCategory::I64;
        }
        if type_is_float(ty, "f32") {
            return TypeCategory::F32;
        }
        if type_is_float(ty, "f64") {
            return TypeCategory::F64;
        }
        if type_is_bool(ty) {
            return TypeCategory::Bool;
        }

        TypeCategory::Unknown
    }

    /// Infer the PostgreSQL type from this category.
    /// Returns None for Unknown types (should trigger compile error).
    pub(crate) fn to_postgres_type(self) -> Option<PostgreSQLType> {
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
#[allow(dead_code)]
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

}

/// PostgreSQL column constraint flags
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PostgreSQLFlag {
    Primary,
    Unique,
    NotNull,
    /// Identity column (GENERATED ALWAYS/BY DEFAULT AS IDENTITY)
    Identity,
    /// Used with TEXT/INTEGER columns to store enum as text/discriminant
    Enum,
    /// Used with native PostgreSQL ENUM types - references the enum type name
    NativeEnum(String),
    Json,
    Check(String),
}

/// Default value specification for PostgreSQL columns
#[allow(dead_code)]
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

/// Identity column mode for GENERATED IDENTITY columns
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IdentityMode {
    /// GENERATED ALWAYS AS IDENTITY - user values rejected unless OVERRIDING SYSTEM VALUE
    Always,
    /// GENERATED BY DEFAULT AS IDENTITY - user values take precedence
    ByDefault,
}

/// Generated column specification (GENERATED AS expression)
#[derive(Debug, Clone)]
pub(crate) struct GeneratedColumn {
    /// The generation expression (SQL)
    pub expression: String,
    /// Whether the column is STORED (computed on write) or VIRTUAL (computed on read)
    pub stored: bool,
}

/// Information about a PostgreSQL table field
#[allow(dead_code)]
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
    pub is_jsonb: bool,
    pub is_serial: bool,
    pub is_generated_identity: bool,
    /// Identity mode for GENERATED IDENTITY columns (always/by_default)
    pub identity_mode: Option<IdentityMode>,
    /// Generated column specification (GENERATED AS expression STORED/VIRTUAL)
    pub generated_column: Option<GeneratedColumn>,
    pub default: Option<PostgreSQLDefault>,
    pub default_fn: Option<TokenStream>,
    pub check_constraint: Option<String>,
    pub foreign_key: Option<PostgreSQLReference>,
    pub has_default: bool,
    pub marker_exprs: Vec<syn::ExprPath>,
}

impl FieldInfo {
    /// Parse field information from a struct field.
    ///
    /// The PostgreSQL type is INFERRED from the Rust type, not from attributes.
    /// Attributes are only used for constraints (primary, unique, etc.).
    pub(crate) fn from_field(field: &Field, is_composite_pk: bool) -> Result<Self> {
        let Some(name) = field.ident.clone() else {
            return Err(Error::new_spanned(
                field,
                "All struct fields must have names. Tuple structs are not supported.",
            ));
        };
        let vis = field.vis.clone();
        let ty = field.ty.clone();

        // Check if field is nullable (wrapped in Option<T>)
        let is_nullable = is_option_type(&ty);

        // Infer PostgreSQL type from Rust type
        let type_str = ty.to_token_stream().to_string();
        let type_category = TypeCategory::from_type(&ty);

        // Initialize constraint-related fields
        let mut flags = HashSet::new();
        let mut default = None;
        let mut default_fn = None;
        let mut check_constraint = None;
        let mut foreign_key = None;
        let mut is_serial = false;
        let mut is_bigserial = false;
        let mut is_generated_identity = false;
        let mut identity_mode = None;
        let mut generated_column = None;
        let mut is_pgenum = false;
        let mut marker_exprs = Vec::new();

        // Parse #[column(...)] attributes for constraints
        let mut is_explicit_json = false;
        let mut is_explicit_jsonb = false;
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
                is_generated_identity = column_info.is_generated_identity;
                identity_mode = column_info.identity_mode;
                generated_column = column_info.generated_column;
                is_pgenum = column_info.is_pgenum;
                is_explicit_json = column_info.is_json;
                is_explicit_jsonb = column_info.is_jsonb;
                marker_exprs = column_info.marker_exprs;
                break;
            }
        }

        // Determine the PostgreSQL column type
        #[cfg(feature = "serde")]
        let column_type = if is_serial {
            PostgreSQLType::Serial
        } else if is_bigserial {
            PostgreSQLType::Bigserial
        } else if is_pgenum {
            // Get the enum type name from the field's base type
            let base_type = option_inner_type(&ty).unwrap_or(&ty);
            let base_type_str = base_type.to_token_stream().to_string().replace(' ', "");
            PostgreSQLType::from_enum_attribute(&base_type_str)
        } else if is_explicit_json {
            // Explicit #[column(json)] - use JSON type for any Serialize/Deserialize type
            PostgreSQLType::Json
        } else if is_explicit_jsonb {
            // Explicit #[column(jsonb)] - use JSONB type for any Serialize/Deserialize type
            PostgreSQLType::Jsonb
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

        #[cfg(not(feature = "serde"))]
        let column_type = if is_serial {
            PostgreSQLType::Serial
        } else if is_bigserial {
            PostgreSQLType::Bigserial
        } else if is_pgenum {
            // Get the enum type name from the field's base type
            let base_type = option_inner_type(&ty).unwrap_or(&ty);
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
            let base_type = option_inner_type(&ty).unwrap_or(&ty);
            let base_type_str = base_type.to_token_stream().to_string().replace(' ', "");
            flags.insert(PostgreSQLFlag::NativeEnum(base_type_str));
        }

        let is_primary = flags.contains(&PostgreSQLFlag::Primary);
        let is_unique = flags.contains(&PostgreSQLFlag::Unique);
        let is_enum = flags.contains(&PostgreSQLFlag::Enum);
        // is_json is true for both inferred serde_json::Value and explicit #[column(json/jsonb)]
        let is_json =
            matches!(type_category, TypeCategory::Json) || is_explicit_json || is_explicit_jsonb;
        let is_serial_type = is_serial || is_bigserial;
        let has_default = default.is_some() || default_fn.is_some() || is_serial_type;

        // Compute base_type once and store it
        let base_type = option_inner_type(&ty).unwrap_or(&ty).clone();

        // Column name defaults to field ident converted to snake_case (can be overridden with NAME attribute)
        let column_name = name.to_string().to_snake_case();

        // Build SQL definition for this column
        let sql_definition = build_sql_definition(SqlDefinitionContext {
            column_name: &column_name,
            column_type: &column_type,
            is_primary_single: is_primary && !is_composite_pk,
            is_not_null: !is_nullable,
            is_unique,
            is_serial: is_serial || is_bigserial,
            default: &default,
            check_constraint: &check_constraint,
        });

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
            is_jsonb: is_explicit_jsonb,
            is_serial: is_serial_type,
            is_generated_identity,
            identity_mode,
            generated_column,
            default,
            default_fn,
            check_constraint,
            foreign_key,
            has_default,
            marker_exprs,
        })
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
        let mut is_generated_identity = false;
        let mut identity_mode: Option<IdentityMode> = None;
        let mut generated_column: Option<GeneratedColumn> = None;
        let mut is_pgenum = false;
        let mut is_json = false;
        let mut is_jsonb = false;
        let enum_type_name: Option<String> = None;
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
                        marker_exprs.push(make_uppercase_path(path_ident, "SERIAL"));
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
                        marker_exprs.push(make_uppercase_path(path_ident, "BIGSERIAL"));
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
                        marker_exprs.push(make_uppercase_path(path_ident, "SMALLSERIAL"));
                    }
                    "PRIMARY" | "PRIMARY_KEY" => {
                        flags.insert(PostgreSQLFlag::Primary);
                        marker_exprs.push(make_uppercase_path(path_ident, "PRIMARY"));
                    }
                    "UNIQUE" => {
                        flags.insert(PostgreSQLFlag::Unique);
                        marker_exprs.push(make_uppercase_path(path_ident, "UNIQUE"));
                    }
                    "IDENTITY" => {
                        // identity(always) or identity(by_default) syntax
                        is_generated_identity = true;
                        flags.insert(PostgreSQLFlag::Identity);

                        // Parse the parenthesized content: identity(always) or identity(by_default)
                        if meta.input.peek(syn::token::Paren) {
                            let content;
                            syn::parenthesized!(content in meta.input);
                            let mode_ident: Ident = content.parse()?;
                            let mode_str = mode_ident.to_string().to_ascii_uppercase();

                            match mode_str.as_str() {
                                "ALWAYS" => {
                                    identity_mode = Some(IdentityMode::Always);
                                }
                                "BY_DEFAULT" => {
                                    identity_mode = Some(IdentityMode::ByDefault);
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        &mode_ident,
                                        "Expected 'always' or 'by_default' as identity mode",
                                    ));
                                }
                            }

                            // TODO: Parse optional sequence options after a comma
                            // e.g., identity(always, start = 100, increment = 10)
                        } else {
                            // Default to ALWAYS if no argument
                            identity_mode = Some(IdentityMode::Always);
                        }

                        marker_exprs.push(make_uppercase_path(path_ident, "IDENTITY"));
                    }
                    "GENERATED" => {
                        // generated(stored, "expr") or generated(virtual, "expr") syntax
                        // For GENERATED AS (expr) STORED/VIRTUAL columns
                        if meta.input.peek(syn::token::Paren) {
                            let content;
                            syn::parenthesized!(content in meta.input);

                            // First argument: "stored" or "virtual"
                            let type_ident: Ident = content.parse()?;
                            let type_str = type_ident.to_string().to_ascii_lowercase();

                            let stored = match type_str.as_str() {
                                "stored" => true,
                                "virtual" => false,
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        &type_ident,
                                        "Expected 'stored' or 'virtual' as first argument to generated()",
                                    ));
                                }
                            };

                            // Expect comma then expression string
                            content.parse::<Token![,]>()?;
                            let expr_lit: Lit = content.parse()?;

                            let expression = if let Lit::Str(s) = expr_lit {
                                s.value()
                            } else {
                                return Err(syn::Error::new_spanned(
                                    expr_lit,
                                    "Expected string literal for generation expression",
                                ));
                            };

                            generated_column = Some(GeneratedColumn { expression, stored });
                        } else {
                            return Err(syn::Error::new_spanned(
                                &meta.path,
                                "generated() requires arguments: generated(stored|virtual, \"expression\")",
                            ));
                        }

                        marker_exprs.push(make_uppercase_path(path_ident, "GENERATED"));
                    }
                    "JSON" => {
                        // Mark as JSON type - allows any Serialize/Deserialize type
                        is_json = true;
                        marker_exprs.push(make_uppercase_path(path_ident, "JSON"));
                    }
                    "JSONB" => {
                        // Mark as JSONB type - allows any Serialize/Deserialize type
                        is_jsonb = true;
                        marker_exprs.push(make_uppercase_path(path_ident, "JSONB"));
                    }
                    "ENUM" => {
                        // Just mark as pgenum - the type is inferred from the field definition
                        is_pgenum = true;
                        marker_exprs.push(make_uppercase_path(path_ident, "ENUM"));
                    }
                    "DEFAULT" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let lit: Lit = meta.input.parse()?;
                            match lit {
                                Lit::Str(s) => {
                                    let escaped = s.value().replace('\'', "''");
                                    default =
                                        Some(PostgreSQLDefault::Literal(format!("'{}'", escaped)))
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
                            marker_exprs.push(make_uppercase_path(path_ident, "DEFAULT"));
                        }
                    }
                    "DEFAULT_FN" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let expr: Expr = meta.input.parse()?;
                            default_fn = Some(quote! { #expr });
                            marker_exprs.push(make_uppercase_path(path_ident, "DEFAULT_FN"));
                        }
                    }
                    "CHECK" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let lit: Lit = meta.input.parse()?;
                            if let Lit::Str(s) = lit {
                                check_constraint = Some(s.value());
                                flags.insert(PostgreSQLFlag::Check(s.value()));
                                marker_exprs.push(make_uppercase_path(path_ident, "CHECK"));
                            }
                        }
                    }
                    "REFERENCES" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let path: ExprPath = meta.input.parse()?;
                            foreign_key = Some(Self::parse_reference(&path)?);                            marker_exprs.push(make_uppercase_path(path_ident, "REFERENCES"));
                        }
                    }
                    "ON_DELETE" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let action_ident: Ident = meta.input.parse()?;
                            let action_upper = action_ident.to_string().to_ascii_uppercase();
                            let action = Self::validate_referential_action(&action_ident)?;
                            if let Some(ref mut fk) = foreign_key {
                                fk.on_delete = Some(action);
                            } else {
                                return Err(syn::Error::new_spanned(
                                    &action_ident,
                                    references_required_message(true, false),
                                ));
                            }
                            marker_exprs.push(make_uppercase_path(path_ident, "ON_DELETE"));
                            // Add marker for the action value (CASCADE, SET_NULL, etc.)
                            marker_exprs.push(make_uppercase_path(&action_ident, &action_upper));
                        }
                    }
                    "ON_UPDATE" => {
                        if meta.input.peek(Token![=]) {
                            meta.input.parse::<Token![=]>()?;
                            let action_ident: Ident = meta.input.parse()?;
                            let action_upper = action_ident.to_string().to_ascii_uppercase();
                            let action = Self::validate_referential_action(&action_ident)?;
                            if let Some(ref mut fk) = foreign_key {
                                fk.on_update = Some(action);
                            } else {
                                return Err(syn::Error::new_spanned(
                                    &action_ident,
                                    references_required_message(false, true),
                                ));
                            }
                            marker_exprs.push(make_uppercase_path(path_ident, "ON_UPDATE"));
                            // Add marker for the action value (CASCADE, SET_NULL, etc.)
                            marker_exprs.push(make_uppercase_path(&action_ident, &action_upper));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &meta.path,
                            format!("Unknown column constraint: '{}'. Supported: PRIMARY, UNIQUE, SERIAL, BIGSERIAL, SMALLSERIAL, IDENTITY, GENERATED, JSON, JSONB, ENUM, DEFAULT, DEFAULT_FN, CHECK, REFERENCES, ON_DELETE, ON_UPDATE", path_ident),
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
            is_generated_identity,
            identity_mode,
            generated_column,
            is_pgenum,
            is_json,
            is_jsonb,
            enum_type_name,
            marker_exprs,
        }))
    }

    /// Validate a referential action (ON DELETE/ON UPDATE)
    fn validate_referential_action(action: &Ident) -> Result<String> {
        let action_str = action.to_string().to_ascii_uppercase();
        match action_str.as_str() {
            "CASCADE" => Ok("CASCADE".to_string()),
            "SET_NULL" => Ok("SET NULL".to_string()),
            "SET_DEFAULT" => Ok("SET DEFAULT".to_string()),
            "RESTRICT" => Ok("RESTRICT".to_string()),
            "NO_ACTION" => Ok("NO ACTION".to_string()),
            _ => Err(Error::new_spanned(
                action,
                format!(
                    "Invalid referential action '{}'. Supported: CASCADE, SET_NULL, SET_DEFAULT, RESTRICT, NO_ACTION",
                    action_str
                ),
            )),
        }
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
            on_delete: None, // Set via on_delete = ... attribute
            on_update: None, // Set via on_update = ... attribute
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
        TypeCategory::from_type(&self.field_type)
    }
}

// =============================================================================
// Table Metadata Generation - Uses drizzle-schema types
// =============================================================================

impl FieldInfo {
    /// Convert default value to a string for DDL metadata (when possible).
    fn default_to_string(&self) -> Option<String> {
        match &self.default {
            Some(PostgreSQLDefault::Literal(lit)) => Some(lit.clone()),
            Some(PostgreSQLDefault::Function(func)) => Some(func.clone()),
            Some(PostgreSQLDefault::Expression(_)) | None => None,
        }
    }

    /// Convert this field to a drizzle-schema Column type.
    pub(crate) fn to_column_meta(
        &self,
        schema: &str,
        table_name: &str,
    ) -> drizzle_types::postgres::ddl::Column {
        let mut col = drizzle_types::postgres::ddl::Column::new(
            schema.to_string(),
            table_name.to_string(),
            self.column_name.clone(),
            self.column_type.to_sql_type().to_string(),
        );

        if !self.is_nullable {
            col = col.not_null();
        }
        if let Some(default) = self.default_to_string() {
            col = col.default_value(default);
        }

        if let Some(generated) = &self.generated_column
            && generated.stored
        {
            col.generated = Some(drizzle_types::postgres::ddl::Generated {
                expression: std::borrow::Cow::Owned(generated.expression.clone()),
                gen_type: drizzle_types::postgres::ddl::GeneratedType::Stored,
            });
        }

        col
    }

    /// Convert this field to a drizzle-schema ForeignKey if it has a reference.
    pub(crate) fn to_foreign_key_meta(
        &self,
        schema: &str,
        table_name: &str,
    ) -> Option<drizzle_types::postgres::ddl::ForeignKey> {
        let fk_ref = self.foreign_key.as_ref()?;
        let table_to = fk_ref.table.to_string();
        let column_to = fk_ref.column.to_string();
        let fk_name = format!(
            "{}_{}_{}_{}_fk",
            table_name, self.column_name, table_to, column_to
        );

        let mut fk = drizzle_types::postgres::ddl::ForeignKey::from_strings(
            schema.to_string(),
            table_name.to_string(),
            fk_name,
            vec![self.column_name.clone()],
            schema.to_string(),
            table_to,
            vec![column_to],
        );

        if let Some(on_delete) = &fk_ref.on_delete
            && let Some(action) =
                drizzle_types::postgres::ddl::ReferentialAction::from_sql(on_delete)
        {
            fk = fk.on_delete(action.as_sql());
        }
        if let Some(on_update) = &fk_ref.on_update
            && let Some(action) =
                drizzle_types::postgres::ddl::ReferentialAction::from_sql(on_update)
        {
            fk = fk.on_update(action.as_sql());
        }

        Some(fk)
    }
}

/// Generate the complete table metadata JSON for use in drizzle-kit compatible migrations.
pub(crate) fn generate_table_meta_json(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
) -> String {
    use drizzle_types::postgres::ddl::{PostgresEntity, PrimaryKey, Table};

    let schema = "public";
    let mut entities: Vec<PostgresEntity> = Vec::new();

    entities.push(PostgresEntity::Table(Table::new(
        schema.to_string(),
        table_name.to_string(),
    )));

    for field in field_infos {
        entities.push(PostgresEntity::Column(
            field.to_column_meta(schema, table_name),
        ));
    }

    for field in field_infos {
        if let Some(fk) = field.to_foreign_key_meta(schema, table_name) {
            entities.push(PostgresEntity::ForeignKey(fk));
        }
    }

    if is_composite_pk {
        let pk_columns: Vec<String> = field_infos
            .iter()
            .filter(|f| f.is_primary)
            .map(|f| f.column_name.clone())
            .collect();

        if pk_columns.len() > 1 {
            let pk_name = format!("{}_pk", table_name);
            let pk = PrimaryKey::from_strings(
                schema.to_string(),
                table_name.to_string(),
                pk_name,
                pk_columns,
            );
            entities.push(PostgresEntity::PrimaryKey(pk));
        }
    }

    serde_json::to_string(&entities).unwrap_or_else(|_| "[]".to_string())
}

/// Context for building a SQL column definition
struct SqlDefinitionContext<'a> {
    column_name: &'a str,
    column_type: &'a PostgreSQLType,
    is_primary_single: bool,
    is_not_null: bool,
    is_unique: bool,
    is_serial: bool,
    default: &'a Option<PostgreSQLDefault>,
    check_constraint: &'a Option<String>,
}

/// Build SQL column definition string for PostgreSQL
fn build_sql_definition(ctx: SqlDefinitionContext<'_>) -> String {
    let mut sql = format!("\"{}\" {}", ctx.column_name, ctx.column_type.to_sql_type());

    // Handle primary key
    if ctx.is_primary_single {
        sql.push_str(" PRIMARY KEY");
    }

    // Add NOT NULL constraint (serial types are implicitly NOT NULL)
    if ctx.is_not_null && !ctx.is_serial {
        sql.push_str(" NOT NULL");
    }

    // Add UNIQUE constraint
    if ctx.is_unique && !ctx.is_primary_single {
        sql.push_str(" UNIQUE");
    }

    // Add DEFAULT value if present
    if let Some(default_value) = ctx.default {
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
    if let Some(check) = ctx.check_constraint {
        sql.push_str(&format!(" CHECK ({})", check));
    }

    sql
}

/// Intermediate structure for parsing column constraint information
#[allow(dead_code)]
struct ColumnInfo {
    flags: HashSet<PostgreSQLFlag>,
    default: Option<PostgreSQLDefault>,
    default_fn: Option<TokenStream>,
    check_constraint: Option<String>,
    foreign_key: Option<PostgreSQLReference>,
    is_serial: bool,
    is_bigserial: bool,
    is_generated_identity: bool,
    identity_mode: Option<IdentityMode>,
    generated_column: Option<GeneratedColumn>,
    is_pgenum: bool,
    is_json: bool,
    is_jsonb: bool,
    enum_type_name: Option<String>,
    marker_exprs: Vec<syn::ExprPath>,
}
