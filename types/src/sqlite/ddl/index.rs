//! `SQLite` indexes: [`IndexDef`] (const) and [`Index`] (runtime), with
//! their key parts [`IndexColumnDef`] and [`IndexColumn`].

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Index Origin
// =============================================================================

/// How an index was created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum IndexOrigin {
    /// By `CREATE INDEX` (the default).
    #[default]
    Manual,
    /// By `SQLite` itself, for a `UNIQUE` constraint.
    Auto,
}

// =============================================================================
// Const-friendly Definition Types
// =============================================================================

/// One key part of an [`IndexDef`]: a column name or a SQL expression.
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
    /// Creates a key part for a column.
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self {
            value,
            is_expression: false,
        }
    }

    /// Creates a key part for a SQL expression.
    #[must_use]
    pub const fn expression(value: &'static str) -> Self {
        Self {
            value,
            is_expression: true,
        }
    }

    /// Converts to the runtime [`IndexColumn`].
    #[must_use]
    pub const fn into_column(self) -> IndexColumn {
        IndexColumn {
            value: Cow::Borrowed(self.value),
            is_expression: self.is_expression,
        }
    }
}

/// One key part of an [`Index`]: a column name or a SQL expression.
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
    /// Creates a key part for a column.
    #[must_use]
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            is_expression: false,
        }
    }

    /// Creates a key part for a SQL expression.
    #[must_use]
    pub fn expression(value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            value: value.into(),
            is_expression: true,
        }
    }
}

impl IndexColumn {
    /// Renders the key part: the column name in backticks, or the
    /// expression in parentheses.
    #[must_use]
    pub fn to_sql(&self) -> String {
        if self.is_expression {
            format!("({})", self.value)
        } else {
            format!("`{}`", self.value.replace('`', "``"))
        }
    }
}

impl From<IndexColumnDef> for IndexColumn {
    fn from(def: IndexColumnDef) -> Self {
        def.into_column()
    }
}

/// An index that can be built in a `const`.
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
    /// `WHERE` condition of a partial index, as SQL
    pub where_clause: Option<&'static str>,
    /// How the index was created
    pub origin: IndexOrigin,
}

impl IndexDef {
    /// Creates a non-unique index with no columns; set them with
    /// [`columns`](Self::columns()).
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

    /// Makes the index `UNIQUE`.
    #[must_use]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
        }
    }

    /// Sets the key parts.
    #[must_use]
    pub const fn columns(self, columns: &'static [IndexColumnDef]) -> Self {
        Self { columns, ..self }
    }

    /// Sets the `WHERE` condition, making it a partial index.
    #[must_use]
    pub const fn where_clause(self, clause: &'static str) -> Self {
        Self {
            where_clause: Some(clause),
            ..self
        }
    }

    /// Marks the index as created by `SQLite` for a `UNIQUE` constraint.
    #[must_use]
    pub const fn auto_origin(self) -> Self {
        Self {
            origin: IndexOrigin::Auto,
            ..self
        }
    }

    /// Converts to the runtime [`Index`].
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

/// An index, as stored in migration snapshots.
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

    /// `WHERE` condition of a partial index, as SQL (serialized as `where`)
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
    /// Creates a non-unique index.
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

    /// Makes the index `UNIQUE`.
    #[must_use]
    pub const fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    /// Returns the index name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the table name.
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
    fn column_names_with_backticks_are_escaped() {
        let column = IndexColumn::new("a`b");
        assert_eq!(column.to_sql(), "`a``b`");
    }

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
        const {
            assert!(IDX.is_unique);
        }
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
