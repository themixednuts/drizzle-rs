//! SQLite Column DDL types
//!
//! This module provides two complementary types:
//! - [`ColumnDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Column`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Generated Column Types
// =============================================================================

/// Generated column type
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

/// Generated column configuration (const-friendly)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GeneratedDef {
    /// SQL expression for generation
    pub expression: &'static str,
    /// Generation type: stored or virtual
    pub gen_type: GeneratedType,
}

impl GeneratedDef {
    /// Create a new stored generated column
    #[must_use]
    pub const fn stored(expression: &'static str) -> Self {
        Self {
            expression,
            gen_type: GeneratedType::Stored,
        }
    }

    /// Create a new virtual generated column
    #[must_use]
    pub const fn virtual_col(expression: &'static str) -> Self {
        Self {
            expression,
            gen_type: GeneratedType::Virtual,
        }
    }

    /// Convert to runtime type
    #[must_use]
    pub const fn into_generated(self) -> Generated {
        Generated {
            expression: Cow::Borrowed(self.expression),
            gen_type: self.gen_type,
        }
    }
}

/// Generated column configuration (runtime)
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

/// Const-friendly column definition for compile-time schema definitions.
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
    /// SQL type (e.g., "INTEGER", "TEXT", "REAL", "BLOB")
    pub sql_type: &'static str,
    /// Is this column NOT NULL?
    pub not_null: bool,
    /// Is this column AUTOINCREMENT?
    pub autoincrement: bool,
    /// Is this column a PRIMARY KEY?
    pub primary_key: bool,
    /// Is this column UNIQUE?
    pub unique: bool,
    /// Default value as string (if any)
    pub default: Option<&'static str>,
    /// Generated column configuration
    pub generated: Option<GeneratedDef>,
}

impl ColumnDef {
    /// Create a new column definition
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str, sql_type: &'static str) -> Self {
        Self {
            table,
            name,
            sql_type,
            not_null: false,
            autoincrement: false,
            primary_key: false,
            unique: false,
            default: None,
            generated: None,
        }
    }

    /// Set NOT NULL constraint
    #[must_use]
    pub const fn not_null(self) -> Self {
        Self {
            not_null: true,
            ..self
        }
    }

    /// Set AUTOINCREMENT
    #[must_use]
    pub const fn autoincrement(self) -> Self {
        Self {
            autoincrement: true,
            ..self
        }
    }

    /// Set PRIMARY KEY (also sets NOT NULL)
    #[must_use]
    pub const fn primary_key(self) -> Self {
        Self {
            primary_key: true,
            not_null: true,
            ..self
        }
    }

    /// Alias for primary_key()
    #[must_use]
    pub const fn primary(self) -> Self {
        self.primary_key()
    }

    /// Set UNIQUE constraint
    #[must_use]
    pub const fn unique(self) -> Self {
        Self {
            unique: true,
            ..self
        }
    }

    /// Set default value
    #[must_use]
    pub const fn default_value(self, value: &'static str) -> Self {
        Self {
            default: Some(value),
            ..self
        }
    }

    /// Set as generated stored column
    #[must_use]
    pub const fn generated_stored(self, expression: &'static str) -> Self {
        Self {
            generated: Some(GeneratedDef::stored(expression)),
            ..self
        }
    }

    /// Set as generated virtual column
    #[must_use]
    pub const fn generated_virtual(self, expression: &'static str) -> Self {
        Self {
            generated: Some(GeneratedDef::virtual_col(expression)),
            ..self
        }
    }

    /// Convert to runtime [`Column`] type
    #[must_use]
    pub const fn into_column(self) -> Column {
        Column {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            sql_type: Cow::Borrowed(self.sql_type),
            not_null: self.not_null,
            autoincrement: if self.autoincrement { Some(true) } else { None },
            primary_key: if self.primary_key { Some(true) } else { None },
            unique: if self.unique { Some(true) } else { None },
            default: match self.default {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            generated: match self.generated {
                Some(g) => Some(g.into_generated()),
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

/// Runtime column entity for serde serialization.
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

    /// SQL type (e.g., "INTEGER", "TEXT", "REAL", "BLOB")
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

    /// Default value as string
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "cow_option_from_string")
    )]
    pub default: Option<Cow<'static, str>>,

    /// Generated column configuration
    #[cfg_attr(feature = "serde", serde(default))]
    pub generated: Option<Generated>,

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
    /// Create a new column (runtime)
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
            ordinal_position: None,
        }
    }

    /// Set NOT NULL
    #[must_use]
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Set AUTOINCREMENT
    #[must_use]
    pub fn autoincrement(mut self) -> Self {
        self.autoincrement = Some(true);
        self
    }

    /// Set default value
    #[must_use]
    pub fn default_value(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Get the column name
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

    /// Get the SQL type
    #[inline]
    #[must_use]
    pub fn sql_type(&self) -> &str {
        &self.sql_type
    }

    /// Check if this is a primary key column
    #[inline]
    #[must_use]
    pub fn is_primary_key(&self) -> bool {
        self.primary_key.unwrap_or(false)
    }

    /// Check if this is an autoincrement column
    #[inline]
    #[must_use]
    pub fn is_autoincrement(&self) -> bool {
        self.autoincrement.unwrap_or(false)
    }

    /// Check if this column has a unique constraint
    #[inline]
    #[must_use]
    pub fn is_unique(&self) -> bool {
        self.unique.unwrap_or(false)
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
        assert!(COL_DEF.not_null);
        assert!(COL_DEF.primary_key);
        assert!(COL_DEF.autoincrement);

        const COL: Column = COL_DEF.into_column();

        assert_eq!(COL.name, Cow::Borrowed("id"));
        assert_eq!(COL.table, Cow::Borrowed("users"));
        assert_eq!(COL.sql_type, Cow::Borrowed("INTEGER"));
        assert!(COL.not_null);
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
