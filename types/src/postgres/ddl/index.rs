//! PostgreSQL Index DDL types
//!
//! See: <https://github.com/drizzle-team/drizzle-orm/blob/beta/drizzle-kit/src/dialects/postgres/ddl.ts>

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Const-friendly Definition Types
// =============================================================================

/// Const-friendly operator class definition
///
/// Represents the operator class for an index column with optional default flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OpclassDef {
    /// Operator class name
    pub name: &'static str,
    /// Whether this is the default operator class
    #[cfg_attr(feature = "serde", serde(default))]
    pub default: bool,
}

impl OpclassDef {
    /// Create a new operator class definition
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            default: false,
        }
    }

    /// Mark as default operator class
    #[must_use]
    pub const fn default_opclass(self) -> Self {
        Self {
            default: true,
            ..self
        }
    }

    /// Convert to runtime [`Opclass`] type
    #[must_use]
    pub const fn into_opclass(self) -> Opclass {
        Opclass {
            name: Cow::Borrowed(self.name),
            default: self.default,
        }
    }
}

/// Runtime operator class entity
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Opclass {
    /// Operator class name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    /// Whether this is the default operator class
    #[cfg_attr(feature = "serde", serde(default))]
    pub default: bool,
}

impl Opclass {
    /// Create a new operator class
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            default: false,
        }
    }

    /// Get the operator class name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl core::fmt::Display for OpclassDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl core::fmt::Display for Opclass {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name)
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
    /// Ascending order (true) or descending (false)
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub asc: bool,
    /// NULLS FIRST ordering
    #[cfg_attr(feature = "serde", serde(default))]
    pub nulls_first: bool,
    /// Operator class (optional)
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub opclass: Option<Opclass>,
}

impl IndexColumn {
    /// Create a new index column
    #[must_use]
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            is_expression: false,
            asc: true,
            nulls_first: false,
            opclass: None,
        }
    }

    /// Create an expression-based index column
    #[must_use]
    pub fn expression(expr: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: expr.into(),
            is_expression: true,
            asc: true,
            nulls_first: false,
            opclass: None,
        }
    }

    /// Set descending order
    #[must_use]
    pub fn desc(mut self) -> Self {
        self.asc = false;
        self
    }

    /// Set NULLS FIRST
    #[must_use]
    pub fn nulls_first(mut self) -> Self {
        self.nulls_first = true;
        self
    }

    /// Set operator class
    #[must_use]
    pub fn with_opclass(mut self, opclass: Opclass) -> Self {
        self.opclass = Some(opclass);
        self
    }
}

impl IndexColumn {
    /// Generate SQL for this index column
    #[must_use]
    pub fn to_sql(&self) -> String {
        let mut sql = if self.is_expression {
            format!("({})", self.value)
        } else {
            format!("\"{}\"", self.value)
        };

        if let Some(ref op) = self.opclass {
            sql.push_str(&format!(" {}", op));
        }
        if !self.asc {
            sql.push_str(" DESC");
        }
        if self.nulls_first {
            sql.push_str(" NULLS FIRST");
        }

        sql
    }
}

impl From<IndexColumnDef> for IndexColumn {
    fn from(def: IndexColumnDef) -> Self {
        Self {
            value: Cow::Borrowed(def.value),
            is_expression: def.is_expression,
            asc: def.asc,
            nulls_first: def.nulls_first,
            opclass: def.opclass.map(|o| o.into_opclass()),
        }
    }
}

impl From<OpclassDef> for Opclass {
    fn from(def: OpclassDef) -> Self {
        def.into_opclass()
    }
}

/// Const-friendly index column definition
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IndexColumnDef {
    /// Column name or expression
    pub value: &'static str,
    /// Whether this is an expression (vs column name)
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_expression: bool,
    /// Ascending order (true) or descending (false)
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub asc: bool,
    /// NULLS FIRST ordering
    #[cfg_attr(feature = "serde", serde(default))]
    pub nulls_first: bool,
    /// Operator class (optional)
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub opclass: Option<OpclassDef>,
}

#[cfg(feature = "serde")]
const fn default_true() -> bool {
    true
}

impl IndexColumnDef {
    /// Create a new index column definition
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self {
            value,
            is_expression: false,
            asc: true,
            nulls_first: false,
            opclass: None,
        }
    }

    /// Create an expression-based index column
    #[must_use]
    pub const fn expression(expr: &'static str) -> Self {
        Self {
            value: expr,
            is_expression: true,
            asc: true,
            nulls_first: false,
            opclass: None,
        }
    }

    /// Set descending order
    #[must_use]
    pub const fn desc(self) -> Self {
        Self { asc: false, ..self }
    }

    /// Set NULLS FIRST
    #[must_use]
    pub const fn nulls_first(self) -> Self {
        Self {
            nulls_first: true,
            ..self
        }
    }

    /// Set operator class
    #[must_use]
    pub const fn opclass(self, opclass: OpclassDef) -> Self {
        Self {
            opclass: Some(opclass),
            ..self
        }
    }

    /// Set operator class by name (convenience method)
    #[must_use]
    pub const fn opclass_name(self, name: &'static str) -> Self {
        Self {
            opclass: Some(OpclassDef::new(name)),
            ..self
        }
    }
}

/// Const-friendly index definition
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexDef {
    /// Schema name
    pub schema: &'static str,
    /// Parent table name
    pub table: &'static str,
    /// Index name
    pub name: &'static str,
    /// Whether the index name was explicitly specified
    pub name_explicit: bool,
    /// Columns included in the index
    pub columns: &'static [IndexColumnDef],
    /// Is this a unique index?
    pub is_unique: bool,
    /// WHERE clause for partial indexes
    pub where_clause: Option<&'static str>,
    /// Index method (btree, hash, etc.)
    pub method: Option<&'static str>,
    /// Storage parameters (WITH clause)
    pub with: Option<&'static str>,
    /// Create concurrently?
    pub concurrently: bool,
}

impl IndexDef {
    /// Create a new index definition
    #[must_use]
    pub const fn new(
        schema: &'static str,
        table: &'static str,
        name: &'static str,
        columns: &'static [IndexColumnDef],
    ) -> Self {
        Self {
            schema,
            table,
            name,
            name_explicit: false,
            columns,
            is_unique: false,
            where_clause: None,
            method: None,
            with: None,
            concurrently: false,
        }
    }

    /// Make this a unique index
    #[must_use]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
        }
    }

    /// Mark the name as explicitly specified
    #[must_use]
    pub const fn explicit_name(self) -> Self {
        Self {
            name_explicit: true,
            ..self
        }
    }

    /// Set WHERE clause for partial index
    #[must_use]
    pub const fn where_clause(self, clause: &'static str) -> Self {
        Self {
            where_clause: Some(clause),
            ..self
        }
    }

    /// Set index method
    #[must_use]
    pub const fn method(self, method: &'static str) -> Self {
        Self {
            method: Some(method),
            ..self
        }
    }

    /// Set storage parameters (WITH clause)
    #[must_use]
    pub const fn with(self, params: &'static str) -> Self {
        Self {
            with: Some(params),
            ..self
        }
    }

    /// Set concurrently flag
    #[must_use]
    pub const fn concurrently(self) -> Self {
        Self {
            concurrently: true,
            ..self
        }
    }

    /// Convert to runtime [`Index`] type
    #[must_use]
    pub fn into_index(self) -> Index {
        Index {
            schema: Cow::Borrowed(self.schema),
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            name_explicit: self.name_explicit,
            columns: self.columns.iter().map(|c| IndexColumn::from(*c)).collect(),
            is_unique: self.is_unique,
            where_clause: self.where_clause.map(Cow::Borrowed),
            method: self.method.map(Cow::Borrowed),
            with: self.with.map(Cow::Borrowed),
            concurrently: self.concurrently,
        }
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
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub schema: Cow<'static, str>,

    /// Parent table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,

    /// Index name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Whether the index name was explicitly specified
    #[cfg_attr(feature = "serde", serde(default))]
    pub name_explicit: bool,

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

    /// Index method (btree, hash, etc.)
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub method: Option<Cow<'static, str>>,

    /// Storage parameters (WITH clause)
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub with: Option<Cow<'static, str>>,

    /// Create concurrently?
    #[cfg_attr(feature = "serde", serde(default))]
    pub concurrently: bool,
}

impl Index {
    /// Create a new index
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'static, str>>,
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: Vec<IndexColumn>,
    ) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            name: name.into(),
            name_explicit: false,
            columns,
            is_unique: false,
            where_clause: None,
            method: None,
            with: None,
            concurrently: false,
        }
    }

    /// Make this a unique index
    #[must_use]
    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    /// Mark the name as explicitly specified
    #[must_use]
    pub fn explicit_name(mut self) -> Self {
        self.name_explicit = true;
        self
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
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
        Self::new("public", "", "", vec![])
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
            IndexColumnDef::new("created_at").desc(),
        ];

        const IDX: IndexDef = IndexDef::new("public", "users", "idx_users_email", COLS).unique();

        assert_eq!(IDX.name, "idx_users_email");
        assert_eq!(IDX.table, "users");
        assert!(IDX.is_unique);
        assert_eq!(IDX.columns.len(), 2);
    }

    #[test]
    fn test_index_def_to_index() {
        const COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
        const DEF: IndexDef = IndexDef::new("public", "users", "idx_email", COLS).unique();

        let idx = DEF.into_index();
        assert_eq!(idx.name(), "idx_email");
        assert!(idx.is_unique);
        assert_eq!(idx.columns.len(), 1);
    }

    #[test]
    fn test_into_index() {
        const COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
        const DEF: IndexDef = IndexDef::new("public", "users", "idx_email", COLS).unique();
        let idx = DEF.into_index();

        assert_eq!(idx.name, Cow::Borrowed("idx_email"));
        assert!(idx.is_unique);
    }
}
