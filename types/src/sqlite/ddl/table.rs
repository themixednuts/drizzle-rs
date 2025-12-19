//! SQLite Table DDL types
//!
//! This module provides two complementary types:
//! - [`TableDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Table`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::cow_from_string;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly table definition for compile-time schema definitions.
///
/// This type uses only `Copy` types (`&'static str`, `bool`) so it can be
/// used in const contexts. Use [`TableDef::into_table`] to convert to
/// the runtime [`Table`] type when needed.
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::TableDef;
///
/// // Fully const - can be used in const contexts
/// const USERS: TableDef = TableDef::new("users").strict();
///
/// assert_eq!(USERS.name, "users");
/// assert!(USERS.strict);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableDef {
    /// Table name
    pub name: &'static str,
    /// Is this a STRICT table?
    pub strict: bool,
    /// Is this a WITHOUT ROWID table?
    pub without_rowid: bool,
}

impl TableDef {
    /// Create a new table definition
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            strict: false,
            without_rowid: false,
        }
    }

    /// Set STRICT mode
    #[must_use]
    pub const fn strict(self) -> Self {
        Self {
            name: self.name,
            strict: true,
            without_rowid: self.without_rowid,
        }
    }

    /// Set WITHOUT ROWID mode
    #[must_use]
    pub const fn without_rowid(self) -> Self {
        Self {
            name: self.name,
            strict: self.strict,
            without_rowid: true,
        }
    }

    /// Convert to runtime [`Table`] type
    #[must_use]
    pub const fn into_table(self) -> Table {
        Table {
            name: Cow::Borrowed(self.name),
            strict: self.strict,
            without_rowid: self.without_rowid,
        }
    }
}

impl Default for TableDef {
    fn default() -> Self {
        Self::new("")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime table entity for serde serialization.
///
/// This type uses `Cow<'static, str>` to support both borrowed and owned strings,
/// making it suitable for JSON serialization/deserialization.
///
/// For compile-time definitions, use [`TableDef`] instead.
///
/// # Examples
///
/// ## From TableDef
///
/// ```
/// use drizzle_types::sqlite::ddl::{TableDef, Table};
///
/// const DEF: TableDef = TableDef::new("users").strict();
/// let table: Table = DEF.into_table();
/// assert_eq!(table.name(), "users");
/// ```
///
/// ## Runtime construction
///
/// ```
/// use drizzle_types::sqlite::ddl::Table;
///
/// let table = Table::new("dynamic_table");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Table {
    /// Table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Is this a STRICT table?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub strict: bool,

    /// Is this a WITHOUT ROWID table?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub without_rowid: bool,
}

impl Table {
    /// Create a new table (runtime)
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            strict: false,
            without_rowid: false,
        }
    }

    /// Set STRICT mode
    #[must_use]
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Set WITHOUT ROWID mode
    #[must_use]
    pub fn without_rowid(mut self) -> Self {
        self.without_rowid = true;
        self
    }

    /// Get the table name as a string slice
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new("")
    }
}

impl From<TableDef> for Table {
    fn from(def: TableDef) -> Self {
        def.into_table()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_table_def() {
        const TABLE: TableDef = TableDef::new("users").strict().without_rowid();
        assert_eq!(TABLE.name, "users");
        assert!(TABLE.strict);
        assert!(TABLE.without_rowid);
    }

    #[test]
    fn test_table_def_to_table() {
        const DEF: TableDef = TableDef::new("users").strict();
        let table = DEF.into_table();
        assert_eq!(table.name(), "users");
        assert!(table.strict);
    }

    #[test]
    fn test_runtime_table() {
        let table = Table::new("posts").strict();
        assert_eq!(table.name(), "posts");
        assert!(table.strict);
        assert!(!table.without_rowid);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let table = Table::new("users").strict();
        let json = serde_json::to_string(&table).unwrap();
        let parsed: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name(), "users");
        assert!(parsed.strict);
    }
}
