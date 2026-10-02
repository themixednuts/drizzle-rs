//! Classification of Rust field types and `SQLite` type declarations.

use super::SQLiteType;

// =============================================================================
// TypeCategory - Rust type classification for code generation
// =============================================================================

/// The kind of a Rust field type, as the `SQLite` macros see it.
///
/// [`from_type_string`](Self::from_type_string) finds the category of a type
/// written as a string, and [`to_sqlite_type`](Self::to_sqlite_type) gives
/// its default column type (the `SQLite` table macro uses it for fields
/// without an explicit type).
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::TypeCategory;
///
/// let category = TypeCategory::from_type_string("String");
/// assert_eq!(category, TypeCategory::String);
/// assert_eq!(category.to_sqlite_type(), Some(drizzle_types::sqlite::SQLiteType::Text));
///
/// let uuid_cat = TypeCategory::from_type_string("Uuid");
/// assert_eq!(uuid_cat, TypeCategory::Uuid);
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
    /// `uuid::Uuid` - UUID type (defaults to BLOB, can be overridden to TEXT)
    Uuid,
    /// Any type stored through `#[column(JSON)]` or `serde_json::Value`
    Json,
    /// Any type with `#[enum]` flag (defaults to TEXT, can be INTEGER)
    Enum,
    /// `i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`, `u32`, `usize` -
    /// Integer types (`u64` is not included)
    Integer,
    /// `f32`, `f64` - Floating point types
    Real,
    /// `bool` - Boolean type (stored as INTEGER 0/1)
    Bool,
    /// `chrono`, `time` and `jiff` date/time types - stored as TEXT
    DateTime,
    /// Not recognized; the column type must be given explicitly.
    Unknown,
}

impl TypeCategory {
    /// Classifies a Rust type written as a string, such as `"Option<i64>"`
    /// or `"chrono::NaiveDate"`. `Option<T>` classifies as `T`; spaces are
    /// ignored. Unrecognized types return [`TypeCategory::Unknown`].
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

        // Fixed-size byte arrays first
        if type_str.starts_with("[u8;")
            || (type_str.contains("[u8;") && !type_str.contains("SmallVec"))
        {
            return Self::ByteArray;
        }

        // ArrayVec/ArrayString and popular string wrappers before generic checks
        if type_str.contains("ArrayString") || type_str.contains("CompactString") {
            return Self::ArrayString;
        }
        if (type_str.contains("ArrayVec") && type_str.contains("u8"))
            || type_str.contains("bytes::Bytes")
            || type_str.contains("bytes::BytesMut")
            || type_str == "Bytes"
            || type_str == "BytesMut"
            || (type_str.contains("SmallVec") && type_str.contains("u8"))
        {
            return Self::ArrayVec;
        }

        // UUID
        if type_str.contains("Uuid") {
            return Self::Uuid;
        }

        // JSON (serde_json::Value)
        if type_str.contains("serde_json::Value") || type_str == "Value" {
            return Self::Json;
        }

        // Chrono types - all stored as TEXT in SQLite
        if type_str.contains("NaiveDate")
            || type_str.contains("NaiveTime")
            || type_str.contains("NaiveDateTime")
            || type_str.contains("DateTime<")
        {
            return Self::DateTime;
        }

        // time and jiff types
        if type_str.contains("time::Date")
            || type_str.contains("time::Time")
            || type_str.contains("PrimitiveDateTime")
            || type_str.contains("OffsetDateTime")
            || type_str.contains("civil::")
            || type_str.contains("jiff::Timestamp")
        {
            return Self::DateTime;
        }

        // String types
        if type_str.contains("String") {
            return Self::String;
        }

        // Vec<u8>
        if type_str.contains("Vec<u8>") {
            return Self::Blob;
        }

        // Primitives - check exact matches for simple types
        match type_str.as_str() {
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "isize" | "usize" => {
                Self::Integer
            }
            "f32" | "f64" => Self::Real,
            "bool" => Self::Bool,
            _ => Self::Unknown,
        }
    }

    /// Returns the default `SQLite` column type for this category, or `None`
    /// for `Unknown`.
    ///
    /// Integers and `bool` are `INTEGER`; strings, date/times, JSON and enums
    /// are `TEXT`; bytes and UUIDs are `BLOB`; floats are `REAL`.
    #[must_use]
    pub const fn to_sqlite_type(&self) -> Option<SQLiteType> {
        match self {
            // Integer types → INTEGER
            Self::Integer | Self::Bool => Some(SQLiteType::Integer),
            // Floating point → REAL
            Self::Real => Some(SQLiteType::Real),
            // String and JSON types → TEXT. Enum defaults to TEXT (variant
            // names) and may be overridden to INTEGER.
            Self::String | Self::ArrayString | Self::DateTime | Self::Json | Self::Enum => {
                Some(SQLiteType::Text)
            }
            // Binary types → BLOB. UUID defaults to BLOB (more efficient),
            // but can be overridden to TEXT.
            Self::Blob | Self::ArrayVec | Self::ByteArray | Self::Uuid => Some(SQLiteType::Blob),
            // Unknown types require explicit annotation
            Self::Unknown => None,
        }
    }

    /// Returns `true` for `ArrayString` and `ArrayVec`, whose values are
    /// converted through `FromSQLiteValue`.
    #[must_use]
    pub const fn uses_from_sqlite_value(&self) -> bool {
        matches!(self, Self::ArrayString | Self::ArrayVec)
    }

    /// Returns `true` for `String`, `Blob` and `Uuid`, whose values can be
    /// taken as an `impl Into<...>` parameter.
    #[must_use]
    pub const fn uses_into_param(&self) -> bool {
        matches!(self, Self::String | Self::Blob | Self::Uuid)
    }
}

// =============================================================================
// SQLTypeCategory - SQL type affinity for parsing
// =============================================================================

/// The affinity group of a `SQLite` type declaration, such as `VARCHAR(255)`
/// or `INTEGER`.
///
/// Only the type names listed on each variant are recognized, exactly or
/// followed directly by `(`; anything else is `Numeric`. This is simpler
/// than `SQLite`'s own substring rules, so `BIGINT UNSIGNED` is `Numeric`
/// here.
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::SQLTypeCategory;
///
/// assert_eq!(SQLTypeCategory::from_sql_type("INTEGER"), SQLTypeCategory::Integer);
/// assert_eq!(SQLTypeCategory::from_sql_type("VARCHAR(255)"), SQLTypeCategory::Text);
/// assert_eq!(SQLTypeCategory::from_sql_type("REAL"), SQLTypeCategory::Real);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum SQLTypeCategory {
    /// INT, INTEGER, TINYINT, SMALLINT, MEDIUMINT, BIGINT, UNSIGNED BIG INT,
    /// INT2, INT8
    Integer,
    /// REAL, DOUBLE, DOUBLE PRECISION, FLOAT
    Real,
    /// NUMERIC, DECIMAL, BOOLEAN, DATE, DATETIME, and any unrecognized type
    Numeric,
    /// TEXT, CHARACTER, VARCHAR, VARYING CHARACTER, NCHAR, NATIVE CHARACTER,
    /// NVARCHAR, CLOB
    Text,
    /// BLOB
    Blob,
}

/// Integer type affinities
const INT_AFFINITIES: &[&str] = &[
    "int",
    "integer",
    "tinyint",
    "smallint",
    "mediumint",
    "bigint",
    "unsigned big int",
    "int2",
    "int8",
];

/// Real number type affinities
const REAL_AFFINITIES: &[&str] = &["real", "double", "double precision", "float"];

/// Numeric type affinities
const NUMERIC_AFFINITIES: &[&str] = &["numeric", "decimal", "boolean", "date", "datetime"];

/// Text type affinities
const TEXT_AFFINITIES: &[&str] = &[
    "text",
    "character",
    "varchar",
    "varying character",
    "nchar",
    "native character",
    "nvarchar",
    "clob",
];

impl SQLTypeCategory {
    /// Classifies a SQL type declaration, ignoring case.
    #[must_use]
    pub fn from_sql_type(sql_type: &str) -> Self {
        // Helper to check if s starts with prefix followed by '('  (case-insensitive)
        fn starts_with_paren(s: &str, prefix: &str) -> bool {
            if s.len() <= prefix.len() {
                return false;
            }
            s[..prefix.len()].eq_ignore_ascii_case(prefix) && s.as_bytes()[prefix.len()] == b'('
        }

        // Check integer affinities
        for a in INT_AFFINITIES {
            if sql_type.eq_ignore_ascii_case(a) || starts_with_paren(sql_type, a) {
                return Self::Integer;
            }
        }

        // Check real affinities
        for a in REAL_AFFINITIES {
            if sql_type.eq_ignore_ascii_case(a) || starts_with_paren(sql_type, a) {
                return Self::Real;
            }
        }

        // Check numeric affinities
        for a in NUMERIC_AFFINITIES {
            if sql_type.eq_ignore_ascii_case(a) || starts_with_paren(sql_type, a) {
                return Self::Numeric;
            }
        }

        // Check text affinities
        for a in TEXT_AFFINITIES {
            if sql_type.eq_ignore_ascii_case(a) || starts_with_paren(sql_type, a) {
                return Self::Text;
            }
        }

        // Check blob
        if sql_type.eq_ignore_ascii_case("blob") || starts_with_paren(sql_type, "blob") {
            return Self::Blob;
        }

        // Default to numeric for unknown types
        Self::Numeric
    }

    /// Returns the name of the matching drizzle-orm (TypeScript) column
    /// builder, such as `"integer"`.
    #[must_use]
    pub const fn drizzle_import(&self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Real => "real",
            Self::Numeric => "numeric",
            Self::Text => "text",
            Self::Blob => "blob",
        }
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
        assert_eq!(TypeCategory::from_type_string("i32"), TypeCategory::Integer);
        assert_eq!(TypeCategory::from_type_string("i64"), TypeCategory::Integer);
        assert_eq!(TypeCategory::from_type_string("f64"), TypeCategory::Real);
        assert_eq!(TypeCategory::from_type_string("bool"), TypeCategory::Bool);
        assert_eq!(
            TypeCategory::from_type_string("Vec<u8>"),
            TypeCategory::Blob
        );
        assert_eq!(TypeCategory::from_type_string("Uuid"), TypeCategory::Uuid);
        assert_eq!(
            TypeCategory::from_type_string("compact_str::CompactString"),
            TypeCategory::ArrayString
        );
        assert_eq!(
            TypeCategory::from_type_string("bytes::Bytes"),
            TypeCategory::ArrayVec
        );
        assert_eq!(
            TypeCategory::from_type_string("Bytes"),
            TypeCategory::ArrayVec
        );
        assert_eq!(
            TypeCategory::from_type_string("BytesMut"),
            TypeCategory::ArrayVec
        );
        assert_eq!(
            TypeCategory::from_type_string("smallvec::SmallVec<[u8; 16]>"),
            TypeCategory::ArrayVec
        );
        assert_eq!(
            TypeCategory::from_type_string("[u8; 16]"),
            TypeCategory::ByteArray
        );
        assert_eq!(
            TypeCategory::from_type_string("Option<String>"),
            TypeCategory::String
        );
        assert_eq!(
            TypeCategory::from_type_string("NaiveDateTime"),
            TypeCategory::DateTime
        );
    }

    #[test]
    fn test_type_category_to_sqlite_type() {
        assert_eq!(
            TypeCategory::Integer.to_sqlite_type(),
            Some(SQLiteType::Integer)
        );
        assert_eq!(TypeCategory::Real.to_sqlite_type(), Some(SQLiteType::Real));
        assert_eq!(
            TypeCategory::String.to_sqlite_type(),
            Some(SQLiteType::Text)
        );
        assert_eq!(TypeCategory::Blob.to_sqlite_type(), Some(SQLiteType::Blob));
        assert_eq!(TypeCategory::Unknown.to_sqlite_type(), None);
    }

    #[test]
    fn test_sql_type_category() {
        assert_eq!(
            SQLTypeCategory::from_sql_type("INTEGER"),
            SQLTypeCategory::Integer
        );
        assert_eq!(
            SQLTypeCategory::from_sql_type("varchar(255)"),
            SQLTypeCategory::Text
        );
        assert_eq!(
            SQLTypeCategory::from_sql_type("REAL"),
            SQLTypeCategory::Real
        );
        assert_eq!(
            SQLTypeCategory::from_sql_type("BLOB"),
            SQLTypeCategory::Blob
        );
        assert_eq!(
            SQLTypeCategory::from_sql_type("DECIMAL(10,2)"),
            SQLTypeCategory::Numeric
        );
    }
}
