//! SQLite type category definitions
//!
//! Provides type classification for both Rust type mapping and SQL parsing.

use super::SQLiteType;

// =============================================================================
// TypeCategory - Rust type classification for code generation
// =============================================================================

/// Categorizes Rust types for consistent handling across the macro system.
///
/// This enum provides a single source of truth for type detection, eliminating
/// fragile string matching scattered across multiple files. This is used for
/// both type inference (Rust type → SQLite type) and code generation.
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
    /// Any type with `#[json]` flag or `serde_json::Value`
    Json,
    /// Any type with `#[enum]` flag (defaults to TEXT, can be INTEGER)
    Enum,
    /// `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32` - Integer types
    Integer,
    /// `f32`, `f64` - Floating point types
    Real,
    /// `bool` - Boolean type (stored as INTEGER 0/1)
    Bool,
    /// Chrono date/time types - stored as TEXT
    DateTime,
    /// Unknown type - requires explicit type annotation
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

        // Fixed-size byte arrays first
        if type_str.starts_with("[u8;") || type_str.contains("[u8;") {
            return TypeCategory::ByteArray;
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

        // Chrono types - all stored as TEXT in SQLite
        if type_str.contains("NaiveDate")
            || type_str.contains("NaiveTime")
            || type_str.contains("NaiveDateTime")
            || type_str.contains("DateTime<")
        {
            return TypeCategory::DateTime;
        }

        // Time crate types
        if type_str.contains("time::Date")
            || type_str.contains("time::Time")
            || type_str.contains("PrimitiveDateTime")
            || type_str.contains("OffsetDateTime")
        {
            return TypeCategory::DateTime;
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
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "isize" | "usize" => {
                TypeCategory::Integer
            }
            "f32" | "f64" => TypeCategory::Real,
            "bool" => TypeCategory::Bool,
            _ => TypeCategory::Unknown,
        }
    }

    /// Infer the SQLite type from this category.
    ///
    /// Returns `Some(SQLiteType)` for types that can be automatically inferred,
    /// or `None` for types that require explicit annotation (Unknown).
    #[must_use]
    pub const fn to_sqlite_type(&self) -> Option<SQLiteType> {
        match self {
            // Integer types → INTEGER
            TypeCategory::Integer | TypeCategory::Bool => Some(SQLiteType::Integer),
            // Floating point → REAL
            TypeCategory::Real => Some(SQLiteType::Real),
            // String types → TEXT
            TypeCategory::String | TypeCategory::ArrayString | TypeCategory::DateTime => {
                Some(SQLiteType::Text)
            }
            // Binary types → BLOB
            TypeCategory::Blob | TypeCategory::ArrayVec | TypeCategory::ByteArray => {
                Some(SQLiteType::Blob)
            }
            // UUID defaults to BLOB (more efficient), but can be overridden to TEXT
            TypeCategory::Uuid => Some(SQLiteType::Blob),
            // JSON defaults to TEXT (human-readable), but can be overridden to BLOB
            TypeCategory::Json => Some(SQLiteType::Text),
            // Enum defaults to TEXT (variant names), but can be overridden to INTEGER
            TypeCategory::Enum => Some(SQLiteType::Text),
            // Unknown types require explicit annotation
            TypeCategory::Unknown => None,
        }
    }

    /// Check if this category requires the `FromSQLiteValue` trait for conversion
    #[must_use]
    pub const fn uses_from_sqlite_value(&self) -> bool {
        matches!(self, TypeCategory::ArrayString | TypeCategory::ArrayVec)
    }

    /// Check if this category should use a generic `impl Into<...>` parameter
    #[must_use]
    pub const fn uses_into_param(&self) -> bool {
        matches!(
            self,
            TypeCategory::String | TypeCategory::Blob | TypeCategory::Uuid
        )
    }
}

// =============================================================================
// SQLTypeCategory - SQL type affinity for parsing
// =============================================================================

/// SQL type category for parsing SQL type strings.
///
/// This categorizes SQL type declarations (e.g., "VARCHAR(255)", "INTEGER")
/// into their SQLite affinity groups for migration/introspection purposes.
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
    /// Integer affinity types: INT, INTEGER, TINYINT, SMALLINT, MEDIUMINT, BIGINT, etc.
    Integer,
    /// Real affinity types: REAL, DOUBLE, DOUBLE PRECISION, FLOAT
    Real,
    /// Numeric affinity types: NUMERIC, DECIMAL, BOOLEAN, DATE, DATETIME
    Numeric,
    /// Text affinity types: TEXT, CHARACTER, VARCHAR, NCHAR, NVARCHAR, CLOB
    Text,
    /// Blob type
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
    /// Determine the type category for a SQL type string
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

    /// Get the drizzle import name for this type
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
