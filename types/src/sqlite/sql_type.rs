//! SQLite column type definitions
//!
//! Defines the core SQLite storage classes and type affinities.

/// Enum representing supported SQLite column types.
///
/// These correspond to the [SQLite storage classes](https://sqlite.org/datatype3.html#storage_classes_and_datatypes).
/// Each type maps to specific Rust types and has different capabilities for constraints and features.
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::SQLiteType;
///
/// let int_type = SQLiteType::Integer;
/// assert_eq!(int_type.to_sql_type(), "INTEGER");
/// assert!(int_type.is_valid_flag("autoincrement"));
///
/// let text_type = SQLiteType::Text;
/// assert!(text_type.is_valid_flag("json"));
/// assert!(!text_type.is_valid_flag("autoincrement"));
/// ```
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
pub enum SQLiteType {
    /// SQLite INTEGER type - stores signed integers up to 8 bytes.
    ///
    /// See: <https://sqlite.org/datatype3.html#integer_datatype>
    ///
    /// Supports: primary keys, autoincrement, enums (discriminant storage)
    Integer,

    /// SQLite TEXT type - stores text in UTF-8, UTF-16BE, or UTF-16LE encoding.
    ///
    /// See: <https://sqlite.org/datatype3.html#text_datatype>
    ///
    /// Supports: enums (variant name storage), JSON serialization
    Text,

    /// SQLite BLOB type - stores binary data exactly as input.
    ///
    /// See: <https://sqlite.org/datatype3.html#blob_datatype>
    ///
    /// Supports: JSON serialization, UUID storage
    Blob,

    /// SQLite REAL type - stores floating point values as 8-byte IEEE floating point numbers.
    ///
    /// See: <https://sqlite.org/datatype3.html#real_datatype>
    Real,

    /// SQLite NUMERIC type - stores values as INTEGER, REAL, or TEXT depending on the value.
    ///
    /// See: <https://sqlite.org/datatype3.html#numeric_datatype>
    Numeric,

    /// SQLite ANY type - no type affinity, can store any type of data.
    ///
    /// See: <https://sqlite.org/datatype3.html#type_affinity>
    #[default]
    Any,
}

impl SQLiteType {
    /// Convert from attribute name to enum variant
    ///
    /// Handles common attribute names used in the macro system.
    #[must_use]
    pub fn from_attribute_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("integer") {
            Some(Self::Integer)
        } else if name.eq_ignore_ascii_case("text") {
            Some(Self::Text)
        } else if name.eq_ignore_ascii_case("blob") {
            Some(Self::Blob)
        } else if name.eq_ignore_ascii_case("real") {
            Some(Self::Real)
        } else if name.eq_ignore_ascii_case("number") || name.eq_ignore_ascii_case("numeric") {
            Some(Self::Numeric)
        } else if name.eq_ignore_ascii_case("boolean") {
            Some(Self::Integer) // Store booleans as integers (0/1)
        } else if name.eq_ignore_ascii_case("any") {
            Some(Self::Any)
        } else {
            None
        }
    }

    /// Get the SQL type string for this type
    #[must_use]
    pub const fn to_sql_type(&self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Text => "TEXT",
            Self::Blob => "BLOB",
            Self::Real => "REAL",
            Self::Numeric => "NUMERIC",
            Self::Any => "ANY",
        }
    }

    /// Check if a flag is valid for this column type
    ///
    /// # Valid Flags per Type
    ///
    /// - `INTEGER`: `primary`, `primary_key`, `unique`, `autoincrement`, `enum`
    /// - `TEXT`: `primary`, `primary_key`, `unique`, `json`, `enum`
    /// - `BLOB`: `primary`, `primary_key`, `unique`, `json`
    /// - `REAL`: `primary`, `primary_key`, `unique`
    /// - `NUMERIC`: `primary`, `primary_key`, `unique`
    /// - `ANY`: `primary`, `primary_key`, `unique`
    #[must_use]
    pub fn is_valid_flag(&self, flag: &str) -> bool {
        matches!(flag, "primary" | "primary_key" | "unique")
            || matches!(
                (self, flag),
                (Self::Integer, "autoincrement")
                    | (Self::Text | Self::Blob, "json")
                    | (Self::Text | Self::Integer, "enum")
            )
    }
}

impl core::fmt::Display for SQLiteType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.to_sql_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_attribute_name() {
        assert_eq!(
            SQLiteType::from_attribute_name("integer"),
            Some(SQLiteType::Integer)
        );
        assert_eq!(
            SQLiteType::from_attribute_name("INTEGER"),
            Some(SQLiteType::Integer)
        );
        assert_eq!(
            SQLiteType::from_attribute_name("text"),
            Some(SQLiteType::Text)
        );
        assert_eq!(
            SQLiteType::from_attribute_name("blob"),
            Some(SQLiteType::Blob)
        );
        assert_eq!(
            SQLiteType::from_attribute_name("boolean"),
            Some(SQLiteType::Integer)
        );
        assert_eq!(SQLiteType::from_attribute_name("unknown"), None);
    }

    #[test]
    fn test_to_sql_type() {
        assert_eq!(SQLiteType::Integer.to_sql_type(), "INTEGER");
        assert_eq!(SQLiteType::Text.to_sql_type(), "TEXT");
        assert_eq!(SQLiteType::Blob.to_sql_type(), "BLOB");
        assert_eq!(SQLiteType::Real.to_sql_type(), "REAL");
        assert_eq!(SQLiteType::Numeric.to_sql_type(), "NUMERIC");
        assert_eq!(SQLiteType::Any.to_sql_type(), "ANY");
    }

    #[test]
    fn test_is_valid_flag() {
        // Autoincrement only valid for INTEGER
        assert!(SQLiteType::Integer.is_valid_flag("autoincrement"));
        assert!(!SQLiteType::Text.is_valid_flag("autoincrement"));
        assert!(!SQLiteType::Blob.is_valid_flag("autoincrement"));

        // JSON valid for TEXT and BLOB
        assert!(SQLiteType::Text.is_valid_flag("json"));
        assert!(SQLiteType::Blob.is_valid_flag("json"));
        assert!(!SQLiteType::Integer.is_valid_flag("json"));

        // Enum valid for TEXT and INTEGER
        assert!(SQLiteType::Text.is_valid_flag("enum"));
        assert!(SQLiteType::Integer.is_valid_flag("enum"));
        assert!(!SQLiteType::Blob.is_valid_flag("enum"));

        // Primary/unique valid for all
        assert!(SQLiteType::Integer.is_valid_flag("primary"));
        assert!(SQLiteType::Text.is_valid_flag("unique"));
        assert!(SQLiteType::Blob.is_valid_flag("primary_key"));
    }
}
