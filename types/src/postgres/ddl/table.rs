//! `PostgreSQL` Table DDL types
//!
//! This module provides two complementary types:
//! - [`TableDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Table`] - A runtime type for serde serialization/deserialization

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

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
    /// Create as an UNLOGGED table?
    pub is_unlogged: bool,
    /// Create as a TEMPORARY table?
    pub is_temporary: bool,
    /// Parent table for INHERITS clause.
    pub inherits: Option<&'static str>,
    /// Tablespace for the table.
    pub tablespace: Option<&'static str>,
    /// Is Row-Level Security enabled?
    pub is_rls_enabled: bool,
    /// Table comment emitted through COMMENT ON TABLE.
    pub comment: Option<&'static str>,
}

impl TableDef {
    /// Create a new table definition
    #[must_use]
    pub const fn new(schema: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            name,
            is_unlogged: false,
            is_temporary: false,
            inherits: None,
            tablespace: None,
            is_rls_enabled: false,
            comment: None,
        }
    }

    /// Set UNLOGGED table storage.
    #[must_use]
    pub const fn unlogged(self) -> Self {
        Self {
            is_unlogged: true,
            is_temporary: false,
            ..self
        }
    }

    /// Set TEMPORARY table storage.
    #[must_use]
    pub const fn temporary(self) -> Self {
        Self {
            is_temporary: true,
            is_unlogged: false,
            ..self
        }
    }

    /// Set INHERITS parent table.
    #[must_use]
    pub const fn inherits(self, parent: &'static str) -> Self {
        Self {
            inherits: Some(parent),
            ..self
        }
    }

    /// Set table tablespace.
    #[must_use]
    pub const fn tablespace(self, tablespace: &'static str) -> Self {
        Self {
            tablespace: Some(tablespace),
            ..self
        }
    }

    /// Set Row-Level Security enabled
    #[must_use]
    pub const fn rls_enabled(self) -> Self {
        Self {
            is_rls_enabled: true,
            ..self
        }
    }

    /// Set the table comment.
    #[must_use]
    pub const fn comment(self, comment: &'static str) -> Self {
        Self {
            comment: Some(comment),
            ..self
        }
    }

    /// Convert to runtime [`Table`] type
    #[must_use]
    pub const fn into_table(self) -> Table {
        Table {
            schema: Cow::Borrowed(self.schema),
            name: Cow::Borrowed(self.name),
            is_unlogged: if self.is_unlogged { Some(true) } else { None },
            is_temporary: if self.is_temporary { Some(true) } else { None },
            inherits: match self.inherits {
                Some(parent) => Some(Cow::Borrowed(parent)),
                None => None,
            },
            tablespace: match self.tablespace {
                Some(tablespace) => Some(Cow::Borrowed(tablespace)),
                None => None,
            },
            is_rls_enabled: if self.is_rls_enabled {
                Some(true)
            } else {
                None
            },
            comment: match self.comment {
                Some(comment) => Some(Cow::Borrowed(comment)),
                None => None,
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

    /// Create as an UNLOGGED table?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub is_unlogged: Option<bool>,

    /// Create as a TEMPORARY table?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub is_temporary: Option<bool>,

    /// Parent table for INHERITS clause.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub inherits: Option<Cow<'static, str>>,

    /// Tablespace for the table.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub tablespace: Option<Cow<'static, str>>,

    /// Is Row-Level Security enabled?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub is_rls_enabled: Option<bool>,

    /// Table comment emitted through COMMENT ON TABLE.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub comment: Option<Cow<'static, str>>,
}

impl Table {
    /// Create a new table (runtime)
    #[must_use]
    pub fn new(schema: impl Into<Cow<'static, str>>, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        }
    }

    /// Set UNLOGGED table storage.
    #[must_use]
    pub const fn unlogged(mut self) -> Self {
        self.is_unlogged = Some(true);
        self.is_temporary = None;
        self
    }

    /// Set TEMPORARY table storage.
    #[must_use]
    pub const fn temporary(mut self) -> Self {
        self.is_temporary = Some(true);
        self.is_unlogged = None;
        self
    }

    /// Set INHERITS parent table.
    #[must_use]
    pub fn inherits(mut self, parent: impl Into<Cow<'static, str>>) -> Self {
        self.inherits = Some(parent.into());
        self
    }

    /// Set table tablespace.
    #[must_use]
    pub fn tablespace(mut self, tablespace: impl Into<Cow<'static, str>>) -> Self {
        self.tablespace = Some(tablespace.into());
        self
    }

    /// Set Row-Level Security enabled
    #[must_use]
    pub const fn rls_enabled(mut self) -> Self {
        self.is_rls_enabled = Some(true);
        self
    }

    /// Set the table comment.
    #[must_use]
    pub fn comment(mut self, comment: impl Into<Cow<'static, str>>) -> Self {
        self.comment = Some(comment.into());
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
        const {
            assert!(TABLE.is_rls_enabled);
        }
    }

    #[test]
    fn test_table_def_storage_attrs() {
        const TABLE: TableDef = TableDef::new("public", "events")
            .unlogged()
            .inherits("base_events")
            .tablespace("fast_ssd");

        const { assert!(TABLE.is_unlogged) };
        assert_eq!(TABLE.inherits, Some("base_events"));
        assert_eq!(TABLE.tablespace, Some("fast_ssd"));

        let table = TABLE.into_table();
        assert_eq!(table.is_unlogged, Some(true));
        assert_eq!(table.inherits.as_deref(), Some("base_events"));
        assert_eq!(table.tablespace.as_deref(), Some("fast_ssd"));
    }

    #[test]
    fn test_table_def_to_table() {
        const DEF: TableDef = TableDef::new("public", "users");
        let table = DEF.into_table();
        assert_eq!(table.schema(), "public");
        assert_eq!(table.name(), "users");
    }
}
