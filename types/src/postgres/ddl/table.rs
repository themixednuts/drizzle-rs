//! PostgreSQL Table DDL types
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableDef {
    /// Schema name
    pub schema: &'static str,
    /// Table name
    pub name: &'static str,
    /// Is Row-Level Security enabled?
    pub is_rls_enabled: bool,
}

impl TableDef {
    /// Create a new table definition
    #[must_use]
    pub const fn new(schema: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            name,
            is_rls_enabled: false,
        }
    }

    /// Set Row-Level Security enabled
    #[must_use]
    pub const fn rls_enabled(self) -> Self {
        Self {
            schema: self.schema,
            name: self.name,
            is_rls_enabled: true,
        }
    }

    /// Convert to runtime [`Table`] type
    #[must_use]
    pub const fn into_table(self) -> Table {
        Table {
            schema: Cow::Borrowed(self.schema),
            name: Cow::Borrowed(self.name),
            is_rls_enabled: if self.is_rls_enabled {
                Some(true)
            } else {
                None
            },
        }
    }
}

impl Default for TableDef {
    fn default() -> Self {
        Self::new("public", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime table entity for serde serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Table {
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub schema: Cow<'static, str>,

    /// Table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Is Row-Level Security enabled?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub is_rls_enabled: Option<bool>,
}

impl Table {
    /// Create a new table (runtime)
    #[must_use]
    pub fn new(schema: impl Into<Cow<'static, str>>, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            is_rls_enabled: None,
        }
    }

    /// Set Row-Level Security enabled
    #[must_use]
    pub fn rls_enabled(mut self) -> Self {
        self.is_rls_enabled = Some(true);
        self
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Get the table name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new("public", "")
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
        const TABLE: TableDef = TableDef::new("public", "users").rls_enabled();

        assert_eq!(TABLE.schema, "public");
        assert_eq!(TABLE.name, "users");
        assert!(TABLE.is_rls_enabled);
    }

    #[test]
    fn test_table_def_to_table() {
        const DEF: TableDef = TableDef::new("public", "users");
        let table = DEF.into_table();
        assert_eq!(table.schema(), "public");
        assert_eq!(table.name(), "users");
    }
}
