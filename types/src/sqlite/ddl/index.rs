//! SQLite Index DDL types
//!
//! This module provides two complementary types:
//! - [`IndexDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Index`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Index Origin
// =============================================================================

/// Index origin - how the index was created
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum IndexOrigin {
    /// Manually created via CREATE INDEX
    #[default]
    Manual,
    /// Auto-created for UNIQUE constraint
    Auto,
}

// =============================================================================
// Const-friendly Definition Types
// =============================================================================

/// Const-friendly index column specification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IndexColumnDef {
    /// Column name or expression
    pub value: &'static str,
    /// Whether this is an expression (vs column name)
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_expression: bool,
}

impl IndexColumnDef {
    /// Create a new index column
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self {
            value,
            is_expression: false,
        }
    }

    /// Create a new index column from an expression
    #[must_use]
    pub const fn expression(value: &'static str) -> Self {
        Self {
            value,
            is_expression: true,
        }
    }

    /// Convert to runtime [`IndexColumn`] type
    #[must_use]
    pub const fn into_column(self) -> IndexColumn {
        IndexColumn {
            value: Cow::Borrowed(self.value),
            is_expression: self.is_expression,
        }
    }
}

/// Runtime index column entity for serde serialization
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IndexColumn {
    /// Column name or expression
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub value: Cow<'static, str>,
    /// Whether this is an expression (vs column name)
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_expression: bool,
}

impl IndexColumn {
    /// Create a new index column
    #[must_use]
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            is_expression: false,
        }
    }

    /// Create a new index column from an expression
    #[must_use]
    pub fn expression(value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            is_expression: true,
        }
    }
}

impl IndexColumn {
    /// Generate SQL for this index column
    #[must_use]
    pub fn to_sql(&self) -> String {
        if self.is_expression {
            format!("({})", self.value)
        } else {
            format!("`{}`", self.value)
        }
    }
}

impl From<IndexColumnDef> for IndexColumn {
    fn from(def: IndexColumnDef) -> Self {
        def.into_column()
    }
}

/// Const-friendly index definition
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::{IndexDef, IndexColumnDef};
///
/// const COLS: &[IndexColumnDef] = &[
///     IndexColumnDef::new("email"),
///     IndexColumnDef::new("created_at"),
/// ];
///
/// const IDX: IndexDef = IndexDef::new("users", "idx_users_email")
///     .unique()
///     .columns(COLS);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexDef {
    /// Parent table name
    pub table: &'static str,
    /// Index name
    pub name: &'static str,
    /// Index columns
    pub columns: &'static [IndexColumnDef],
    /// Is this a UNIQUE index?
    pub is_unique: bool,
    /// Optional WHERE clause for partial indexes
    pub where_clause: Option<&'static str>,
    /// How the index was created
    pub origin: IndexOrigin,
}

impl IndexDef {
    /// Create a new index definition
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            columns: &[],
            is_unique: false,
            where_clause: None,
            origin: IndexOrigin::Manual,
        }
    }

    /// Set unique constraint
    #[must_use]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
        }
    }

    /// Set columns
    #[must_use]
    pub const fn columns(self, columns: &'static [IndexColumnDef]) -> Self {
        Self { columns, ..self }
    }

    /// Set WHERE clause for partial index
    #[must_use]
    pub const fn where_clause(self, clause: &'static str) -> Self {
        Self {
            where_clause: Some(clause),
            ..self
        }
    }

    /// Set origin to auto (for UNIQUE constraint indexes)
    #[must_use]
    pub const fn auto_origin(self) -> Self {
        Self {
            origin: IndexOrigin::Auto,
            ..self
        }
    }

    /// Convert to runtime [`Index`] type
    #[must_use]
    pub fn into_index(self) -> Index {
        Index {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            columns: self.columns.iter().map(|c| IndexColumn::from(*c)).collect(),
            is_unique: self.is_unique,
            where_clause: self.where_clause.map(Cow::Borrowed),
            origin: self.origin,
        }
    }
}

impl Default for IndexDef {
    fn default() -> Self {
        Self::new("", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime index entity for serde serialization
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Index {
    /// Parent table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,

    /// Index name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Columns included in the index
    pub columns: Vec<IndexColumn>,

    /// Is this a unique index?
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_unique: bool,

    /// WHERE clause for partial indexes
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "where",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub where_clause: Option<Cow<'static, str>>,

    /// How the index was created
    #[cfg_attr(feature = "serde", serde(default))]
    pub origin: IndexOrigin,
}

impl Index {
    /// Create a new index
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: Vec<IndexColumn>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns,
            is_unique: false,
            where_clause: None,
            origin: IndexOrigin::Manual,
        }
    }

    /// Make this a unique index
    #[must_use]
    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    /// Get the index name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the table name
    #[inline]
    #[must_use]
    pub fn table(&self) -> &str {
        &self.table
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new("", "", vec![])
    }
}

impl From<IndexDef> for Index {
    fn from(def: IndexDef) -> Self {
        def.into_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_index_def() {
        const COLS: &[IndexColumnDef] = &[
            IndexColumnDef::new("email"),
            IndexColumnDef::new("created_at"),
        ];

        const IDX: IndexDef = IndexDef::new("users", "idx_users_email")
            .unique()
            .columns(COLS);

        assert_eq!(IDX.name, "idx_users_email");
        assert_eq!(IDX.table, "users");
        assert!(IDX.is_unique);
        assert_eq!(IDX.columns.len(), 2);
    }

    #[test]
    fn test_expression_column() {
        const COLS: &[IndexColumnDef] = &[IndexColumnDef::expression("lower(email)")];

        const IDX: IndexDef = IndexDef::new("users", "idx_email_lower").columns(COLS);

        assert!(IDX.columns[0].is_expression);
    }

    #[test]
    fn test_index_def_to_index() {
        const COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
        const DEF: IndexDef = IndexDef::new("users", "idx_email").unique().columns(COLS);

        let idx = DEF.into_index();
        assert_eq!(idx.name(), "idx_email");
        assert!(idx.is_unique);
        assert_eq!(idx.columns.len(), 1);
    }

    #[test]
    fn test_into_index() {
        const COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
        const DEF: IndexDef = IndexDef::new("users", "idx_email").unique().columns(COLS);
        let idx = DEF.into_index();

        assert_eq!(idx.name, Cow::Borrowed("idx_email"));
        assert!(idx.is_unique);
    }
}
