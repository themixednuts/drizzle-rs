//! `SQLite` column types ([`SQLiteType`]) and affinities ([`SQLiteAffinity`]).

/// A `SQLite` column type as written in DDL.
///
/// These are the types a STRICT table accepts, plus `NUMERIC`; see
/// [SQLite datatypes](https://sqlite.org/datatype3.html). The type decides
/// which column attributes are allowed ([`is_valid_flag`](Self::is_valid_flag)).
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
    /// `SQLite` INTEGER type - stores signed integers up to 8 bytes.
    ///
    /// See: <https://sqlite.org/datatype3.html#integer_datatype>
    ///
    /// Supports: primary keys, autoincrement, enums (discriminant storage)
    Integer,

    /// `SQLite` TEXT type - stores text in UTF-8, UTF-16BE, or UTF-16LE encoding.
    ///
    /// See: <https://sqlite.org/datatype3.html#text_datatype>
    ///
    /// Supports: enums (variant name storage), JSON (`#[column(json)]`)
    Text,

    /// `SQLite` BLOB type - stores binary data exactly as input.
    ///
    /// See: <https://sqlite.org/datatype3.html#blob_datatype>
    ///
    /// Used for byte arrays and UUIDs.
    Blob,

    /// `SQLite` REAL type - stores floating point values as 8-byte IEEE floating point numbers.
    ///
    /// See: <https://sqlite.org/datatype3.html#real_datatype>
    Real,

    /// `SQLite` NUMERIC type - stores values as INTEGER, REAL, or TEXT depending on the value.
    ///
    /// See: <https://sqlite.org/datatype3.html#numeric_datatype>
    Numeric,

    /// `SQLite` ANY type - no type affinity, can store any type of data.
    /// This holds in STRICT tables; elsewhere `SQLite` gives a column declared
    /// `ANY` `NUMERIC` affinity. The default.
    ///
    /// See: <https://sqlite.org/stricttables.html>
    #[default]
    Any,
}

/// `SQLite` type affinity: how a column converts values it stores.
///
/// See <https://sqlite.org/datatype3.html#type_affinity>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
pub enum SQLiteAffinity {
    /// `INTEGER` affinity.
    Integer,
    /// `TEXT` affinity.
    Text,
    /// `BLOB` affinity: values are stored as given.
    Blob,
    /// `REAL` affinity.
    Real,
    /// `NUMERIC` affinity.
    Numeric,
    /// No affinity (`ANY` in STRICT tables).
    Any,
}

impl SQLiteType {
    /// Parses a type name from a column attribute, ignoring case. Returns
    /// `None` for unknown names.
    ///
    /// Also accepts `"number"` (as `NUMERIC`) and `"boolean"` (as `INTEGER`).
    #[must_use]
    pub const fn from_attribute_name(name: &str) -> Option<Self> {
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

    /// Returns the type as written in DDL, such as `"INTEGER"`.
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

    /// Returns this type's `SQLite` affinity.
    #[must_use]
    pub const fn affinity(&self) -> SQLiteAffinity {
        match self {
            Self::Integer => SQLiteAffinity::Integer,
            Self::Text => SQLiteAffinity::Text,
            Self::Blob => SQLiteAffinity::Blob,
            Self::Real => SQLiteAffinity::Real,
            Self::Numeric => SQLiteAffinity::Numeric,
            Self::Any => SQLiteAffinity::Any,
        }
    }

    /// Returns `true` if a STRICT table accepts this type: every type except `NUMERIC`.
    #[must_use]
    pub const fn is_strict_allowed(&self) -> bool {
        matches!(
            self,
            Self::Integer | Self::Real | Self::Text | Self::Blob | Self::Any
        )
    }

    /// Returns `true` if the column attribute flag can be used with this type.
    ///
    /// Valid flags per type:
    ///
    /// - `INTEGER`: `primary`, `primary_key`, `unique`, `autoincrement`, `enum`
    /// - `TEXT`: `primary`, `primary_key`, `unique`, `json`, `enum`
    /// - `BLOB`: `primary`, `primary_key`, `unique`
    /// - `REAL`: `primary`, `primary_key`, `unique`
    /// - `NUMERIC`: `primary`, `primary_key`, `unique`
    /// - `ANY`: `primary`, `primary_key`, `unique`
    #[must_use]
    pub fn is_valid_flag(&self, flag: &str) -> bool {
        matches!(flag, "primary" | "primary_key" | "unique")
            || matches!(
                (self, flag),
                (Self::Integer, "autoincrement")
                    | (Self::Text, "json")
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
    fn test_affinity_mapping() {
        assert_eq!(SQLiteType::Integer.affinity(), SQLiteAffinity::Integer);
        assert_eq!(SQLiteType::Text.affinity(), SQLiteAffinity::Text);
        assert_eq!(SQLiteType::Blob.affinity(), SQLiteAffinity::Blob);
        assert_eq!(SQLiteType::Real.affinity(), SQLiteAffinity::Real);
        assert_eq!(SQLiteType::Numeric.affinity(), SQLiteAffinity::Numeric);
        assert_eq!(SQLiteType::Any.affinity(), SQLiteAffinity::Any);
    }

    #[test]
    fn test_strict_allowed_types() {
        assert!(SQLiteType::Integer.is_strict_allowed());
        assert!(SQLiteType::Text.is_strict_allowed());
        assert!(SQLiteType::Blob.is_strict_allowed());
        assert!(SQLiteType::Real.is_strict_allowed());
        assert!(SQLiteType::Any.is_strict_allowed());
        assert!(!SQLiteType::Numeric.is_strict_allowed());
    }

    #[test]
    fn test_is_valid_flag() {
        // Autoincrement only valid for INTEGER
        assert!(SQLiteType::Integer.is_valid_flag("autoincrement"));
        assert!(!SQLiteType::Text.is_valid_flag("autoincrement"));
        assert!(!SQLiteType::Blob.is_valid_flag("autoincrement"));

        // JSON uses TEXT storage.
        assert!(SQLiteType::Text.is_valid_flag("json"));
        assert!(!SQLiteType::Blob.is_valid_flag("json"));
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
