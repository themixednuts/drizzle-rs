//! `SQLite` columns: [`ColumnDef`] (const) and [`Column`] (runtime).

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Generated Column Types
// =============================================================================

/// Whether a generated column is `STORED` or `VIRTUAL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum GeneratedType {
    /// Stored generated column
    #[default]
    Stored,
    /// Virtual generated column
    Virtual,
}

/// The `GENERATED ALWAYS AS (...)` part of a [`ColumnDef`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GeneratedDef {
    /// SQL expression for generation
    pub expression: &'static str,
    /// Generation type: stored or virtual
    pub gen_type: GeneratedType,
}

impl GeneratedDef {
    /// Creates a `STORED` generated column from a SQL expression.
    #[must_use]
    pub const fn stored(expression: &'static str) -> Self {
        Self {
            expression,
            gen_type: GeneratedType::Stored,
        }
    }

    /// Creates a `VIRTUAL` generated column from a SQL expression.
    #[must_use]
    pub const fn virtual_col(expression: &'static str) -> Self {
        Self {
            expression,
            gen_type: GeneratedType::Virtual,
        }
    }

    /// Converts to the runtime [`Generated`].
    #[must_use]
    pub const fn into_generated(self) -> Generated {
        Generated {
            expression: Cow::Borrowed(self.expression),
            gen_type: self.gen_type,
        }
    }
}

/// The `GENERATED ALWAYS AS (...)` part of a [`Column`]. With `serde`, the
/// fields are named `as` and `type`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Generated {
    /// SQL expression for generation
    #[cfg_attr(
        feature = "serde",
        serde(rename = "as", deserialize_with = "cow_from_string")
    )]
    pub expression: Cow<'static, str>,
    /// Generation type: stored or virtual
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub gen_type: GeneratedType,
}

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// How a [`ColumnDef`] is a primary key: plain `PRIMARY KEY` or
/// `PRIMARY KEY AUTOINCREMENT`. A `None` in the `primary_key` field means it
/// is not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimaryKeyKind {
    /// Plain `PRIMARY KEY`
    Plain,
    /// `PRIMARY KEY AUTOINCREMENT`
    Autoincrement,
}

/// A column definition that can be built in a `const`.
///
/// Builder methods set one property each; [`into_column`](Self::into_column)
/// converts to the runtime [`Column`].
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::ColumnDef;
///
/// const ID: ColumnDef = ColumnDef::new("users", "id", "INTEGER")
///     .primary_key()
///     .autoincrement();
///
/// const COLUMNS: &[ColumnDef] = &[
///     ColumnDef::new("users", "id", "INTEGER").primary_key().autoincrement(),
///     ColumnDef::new("users", "name", "TEXT").not_null(),
///     ColumnDef::new("users", "email", "TEXT"),
/// ];
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColumnDef {
    /// Parent table name
    pub table: &'static str,
    /// Column name
    pub name: &'static str,
    /// SQL type, such as `"INTEGER"` or `"TEXT"`
    pub sql_type: &'static str,
    /// `NOT NULL`
    pub not_null: bool,
    /// Primary key form; `None` if not a primary key
    pub primary_key: Option<PrimaryKeyKind>,
    /// `UNIQUE`
    pub unique: bool,
    /// `DEFAULT` value as SQL, such as `"0"` or `"'active'"`
    pub default: Option<&'static str>,
    /// Generated column configuration
    pub generated: Option<GeneratedDef>,
    /// Collation name (`BINARY`, `NOCASE`, `RTRIM`, or a custom registered collation).
    /// `None` means the default collation (`BINARY`) and no `COLLATE` clause is emitted.
    pub collate: Option<&'static str>,
}

impl ColumnDef {
    /// Creates a nullable column with no constraints.
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str, sql_type: &'static str) -> Self {
        Self {
            table,
            name,
            sql_type,
            not_null: false,
            primary_key: None,
            unique: false,
            default: None,
            generated: None,
            collate: None,
        }
    }

    /// Adds `NOT NULL`.
    #[must_use]
    pub const fn not_null(self) -> Self {
        Self {
            not_null: true,
            ..self
        }
    }

    /// Makes the column `PRIMARY KEY AUTOINCREMENT` (and `NOT NULL`).
    #[must_use]
    pub const fn autoincrement(self) -> Self {
        Self {
            primary_key: Some(PrimaryKeyKind::Autoincrement),
            not_null: true,
            ..self
        }
    }

    /// Makes the column `PRIMARY KEY` (and `NOT NULL`), keeping
    /// `AUTOINCREMENT` if already set.
    #[must_use]
    pub const fn primary_key(self) -> Self {
        let primary_key = match self.primary_key {
            Some(kind) => Some(kind),
            None => Some(PrimaryKeyKind::Plain),
        };
        Self {
            primary_key,
            not_null: true,
            ..self
        }
    }

    /// Same as [`primary_key`](Self::primary_key()).
    #[must_use]
    pub const fn primary(self) -> Self {
        self.primary_key()
    }

    /// Adds `UNIQUE`.
    #[must_use]
    pub const fn unique(self) -> Self {
        Self {
            unique: true,
            ..self
        }
    }

    /// Sets the `DEFAULT` value, written as SQL.
    #[must_use]
    pub const fn default_value(self, value: &'static str) -> Self {
        Self {
            default: Some(value),
            ..self
        }
    }

    /// Makes the column `GENERATED ALWAYS AS (expression) STORED`.
    #[must_use]
    pub const fn generated_stored(self, expression: &'static str) -> Self {
        Self {
            generated: Some(GeneratedDef::stored(expression)),
            ..self
        }
    }

    /// Makes the column `GENERATED ALWAYS AS (expression) VIRTUAL`.
    #[must_use]
    pub const fn generated_virtual(self, expression: &'static str) -> Self {
        Self {
            generated: Some(GeneratedDef::virtual_col(expression)),
            ..self
        }
    }

    /// Sets the column's `COLLATE` sequence.
    ///
    /// `name` should be a built-in collation (`BINARY`, `NOCASE`, `RTRIM`) or
    /// one registered on the connection with `sqlite3_create_collation`.
    #[must_use]
    pub const fn collate(self, name: &'static str) -> Self {
        Self {
            collate: Some(name),
            ..self
        }
    }

    /// Converts to the runtime [`Column`].
    #[must_use]
    pub const fn into_column(self) -> Column {
        Column {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            sql_type: Cow::Borrowed(self.sql_type),
            not_null: self.not_null,
            autoincrement: match self.primary_key {
                Some(PrimaryKeyKind::Autoincrement) => Some(true),
                _ => None,
            },
            primary_key: if self.primary_key.is_some() {
                Some(true)
            } else {
                None
            },
            unique: if self.unique { Some(true) } else { None },
            default: match self.default {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            generated: match self.generated {
                Some(g) => Some(g.into_generated()),
                None => None,
            },
            collate: match self.collate {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            ordinal_position: None,
        }
    }
}

impl Default for ColumnDef {
    fn default() -> Self {
        Self::new("", "", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// A table column, as stored in migration snapshots.
///
/// The boolean constraints are `Option<bool>` to match the snapshot format;
/// `None` and `Some(false)` both mean unset.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Column {
    /// Parent table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,

    /// Column name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// SQL type, such as `"INTEGER"` (serialized as `type`)
    #[cfg_attr(
        feature = "serde",
        serde(rename = "type", deserialize_with = "cow_from_string")
    )]
    pub sql_type: Cow<'static, str>,

    /// Is this column NOT NULL?
    #[cfg_attr(feature = "serde", serde(default))]
    pub not_null: bool,

    /// Is this column AUTOINCREMENT?
    #[cfg_attr(feature = "serde", serde(default))]
    pub autoincrement: Option<bool>,

    /// Is this column a PRIMARY KEY?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub primary_key: Option<bool>,

    /// Is this column UNIQUE?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub unique: Option<bool>,

    /// `DEFAULT` value as SQL
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "cow_option_from_string")
    )]
    pub default: Option<Cow<'static, str>>,

    /// Generated column configuration
    #[cfg_attr(feature = "serde", serde(default))]
    pub generated: Option<Generated>,

    /// Collation sequence (`BINARY`, `NOCASE`, `RTRIM`, or custom). `None` means
    /// the default `BINARY` collation and no `COLLATE` clause is emitted.
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "cow_option_from_string")
    )]
    pub collate: Option<Cow<'static, str>>,

    /// Ordinal position within the table (cid, 0-based).
    ///
    /// This is primarily populated by introspection and used for stable codegen ordering.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ordinal_position: Option<i32>,
}

impl Column {
    /// Creates a nullable column with no constraints.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        sql_type: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            sql_type: sql_type.into(),
            not_null: false,
            autoincrement: None,
            primary_key: None,
            unique: None,
            default: None,
            generated: None,
            collate: None,
            ordinal_position: None,
        }
    }

    /// Adds `NOT NULL`.
    #[must_use]
    pub const fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Marks the column `AUTOINCREMENT`. Unlike [`ColumnDef::autoincrement`],
    /// this does not set `primary_key` or `not_null`.
    #[must_use]
    pub const fn autoincrement(mut self) -> Self {
        self.autoincrement = Some(true);
        self
    }

    /// Sets the `DEFAULT` value, written as SQL.
    #[must_use]
    pub fn default_value(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Returns the column name.
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

    /// Returns the SQL type.
    #[inline]
    #[must_use]
    pub fn sql_type(&self) -> &str {
        &self.sql_type
    }

    /// Returns `true` if `primary_key` is `Some(true)`.
    #[inline]
    #[must_use]
    pub const fn is_primary_key(&self) -> bool {
        matches!(self.primary_key, Some(true))
    }

    /// Returns `true` if `autoincrement` is `Some(true)`.
    #[inline]
    #[must_use]
    pub const fn is_autoincrement(&self) -> bool {
        matches!(self.autoincrement, Some(true))
    }

    /// Returns `true` if `unique` is `Some(true)`.
    #[inline]
    #[must_use]
    pub const fn is_unique(&self) -> bool {
        matches!(self.unique, Some(true))
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new("", "", "")
    }
}

impl From<ColumnDef> for Column {
    fn from(def: ColumnDef) -> Self {
        let mut col = def.into_column();
        // Handle generated conversion at runtime
        if let Some(generated_def) = def.generated {
            col.generated = Some(generated_def.into_generated());
        }
        col
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_column_def() {
        const COL_DEF: ColumnDef = ColumnDef::new("users", "id", "INTEGER")
            .primary_key()
            .autoincrement();

        assert_eq!(COL_DEF.name, "id");
        assert_eq!(COL_DEF.table, "users");
        assert_eq!(COL_DEF.sql_type, "INTEGER");
        const {
            assert!(COL_DEF.not_null);
        }
        const {
            assert!(COL_DEF.primary_key.is_some());
        }
        const {
            assert!(matches!(
                COL_DEF.primary_key,
                Some(PrimaryKeyKind::Autoincrement)
            ));
        }

        let col: Column = COL_DEF.into_column();

        assert_eq!(col.name, Cow::Borrowed("id"));
        assert_eq!(col.table, Cow::Borrowed("users"));
        assert_eq!(col.sql_type, Cow::Borrowed("INTEGER"));
        assert!(col.not_null);
        // assert!(COL.primary_key);
        // assert!(COL.autoincrement);
    }

    #[test]
    fn test_const_columns_array() {
        const COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("users", "id", "INTEGER")
                .primary_key()
                .autoincrement(),
            ColumnDef::new("users", "name", "TEXT").not_null(),
            ColumnDef::new("users", "email", "TEXT"),
        ];

        assert_eq!(COLUMNS.len(), 3);
        assert_eq!(COLUMNS[0].name, "id");
        assert_eq!(COLUMNS[1].name, "name");
        assert_eq!(COLUMNS[2].name, "email");
        assert!(COLUMNS[1].not_null);
        assert!(!COLUMNS[2].not_null);
    }

    #[test]
    fn test_generated_column() {
        const GEN_COL: ColumnDef = ColumnDef::new("users", "full_name", "TEXT")
            .generated_stored("first_name || ' ' || last_name");

        assert!(GEN_COL.generated.is_some());
        assert_eq!(GEN_COL.generated.unwrap().gen_type, GeneratedType::Stored);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let col = Column::new("users", "id", "INTEGER");
        let json = serde_json::to_string(&col).unwrap();
        let parsed: Column = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name(), "id");
    }
}
