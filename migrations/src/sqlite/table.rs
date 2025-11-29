//! SQLite table schema types matching drizzle-kit format

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Column metadata - matches drizzle-kit's column schema
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Column name
    pub name: String,
    /// SQL type (lowercase: "integer", "text", "real", "blob")
    #[serde(rename = "type")]
    pub sql_type: String,
    /// Is this column a primary key?
    pub primary_key: bool,
    /// Is this column NOT NULL?
    pub not_null: bool,
    /// Is this column AUTOINCREMENT?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoincrement: Option<bool>,
    /// Default value (as JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Generated column configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<Generated>,
}

impl Column {
    /// Create a new column with minimal configuration
    pub fn new(name: impl Into<String>, sql_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql_type: sql_type.into(),
            primary_key: false,
            not_null: false,
            autoincrement: None,
            default: None,
            generated: None,
        }
    }

    /// Set this column as primary key
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    /// Set this column as NOT NULL
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Set this column as AUTOINCREMENT
    pub fn autoincrement(mut self) -> Self {
        self.autoincrement = Some(true);
        self
    }

    /// Set default value
    pub fn default_value(mut self, value: serde_json::Value) -> Self {
        self.default = Some(value);
        self
    }
}

/// Generated column configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Generated {
    /// Generation type: "stored" or "virtual"
    #[serde(rename = "type")]
    pub gen_type: GeneratedType,
    /// SQL expression for generation
    #[serde(rename = "as")]
    pub expression: String,
}

/// Generated column type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GeneratedType {
    Stored,
    Virtual,
}

/// Index metadata
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    /// Index name
    pub name: String,
    /// Columns included in the index
    pub columns: Vec<String>,
    /// Is this a unique index?
    pub is_unique: bool,
    /// WHERE clause for partial indexes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
}

impl Index {
    /// Create a new index
    pub fn new(name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            name: name.into(),
            columns,
            is_unique: false,
            r#where: None,
        }
    }

    /// Make this a unique index
    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }
}

/// Foreign key constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    /// Constraint name
    pub name: String,
    /// Source table name
    pub table_from: String,
    /// Source column names
    pub columns_from: Vec<String>,
    /// Target table name
    pub table_to: String,
    /// Target column names
    pub columns_to: Vec<String>,
    /// ON UPDATE action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,
    /// ON DELETE action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<String>,
}

impl ForeignKey {
    /// Create a new foreign key
    pub fn new(
        name: impl Into<String>,
        table_from: impl Into<String>,
        columns_from: Vec<String>,
        table_to: impl Into<String>,
        columns_to: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            table_from: table_from.into(),
            columns_from,
            table_to: table_to.into(),
            columns_to,
            on_update: None,
            on_delete: None,
        }
    }
}

/// Composite primary key
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CompositePK {
    /// Columns in the composite primary key
    pub columns: Vec<String>,
    /// Optional constraint name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Unique constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UniqueConstraint {
    /// Constraint name
    pub name: String,
    /// Columns in the unique constraint
    pub columns: Vec<String>,
}

/// Check constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CheckConstraint {
    /// Constraint name
    pub name: String,
    /// Check expression
    pub value: String,
}

/// Table metadata - matches drizzle-kit's table schema
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    /// Table name
    pub name: String,
    /// Columns (keyed by column name)
    pub columns: HashMap<String, Column>,
    /// Indexes (keyed by index name)
    pub indexes: HashMap<String, Index>,
    /// Foreign keys (keyed by constraint name)
    pub foreign_keys: HashMap<String, ForeignKey>,
    /// Composite primary keys
    pub composite_primary_keys: HashMap<String, CompositePK>,
    /// Unique constraints
    #[serde(default)]
    pub unique_constraints: HashMap<String, UniqueConstraint>,
    /// Check constraints
    #[serde(default)]
    pub check_constraints: HashMap<String, CheckConstraint>,
}

impl Table {
    /// Create a new table with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: HashMap::new(),
            indexes: HashMap::new(),
            foreign_keys: HashMap::new(),
            composite_primary_keys: HashMap::new(),
            unique_constraints: HashMap::new(),
            check_constraints: HashMap::new(),
        }
    }

    /// Add a column to the table
    pub fn add_column(&mut self, column: Column) {
        self.columns.insert(column.name.clone(), column);
    }

    /// Add an index to the table
    pub fn add_index(&mut self, index: Index) {
        self.indexes.insert(index.name.clone(), index);
    }

    /// Add a foreign key to the table
    pub fn add_foreign_key(&mut self, fk: ForeignKey) {
        self.foreign_keys.insert(fk.name.clone(), fk);
    }
}

/// View metadata
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// View name
    pub name: String,
    /// View columns
    pub columns: HashMap<String, Column>,
    /// View definition SQL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    /// Is this an existing view (for introspection)
    pub is_existing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_builder() {
        let col = Column::new("id", "integer")
            .primary_key()
            .not_null()
            .autoincrement();

        assert_eq!(col.name, "id");
        assert_eq!(col.sql_type, "integer");
        assert!(col.primary_key);
        assert!(col.not_null);
        assert_eq!(col.autoincrement, Some(true));
    }

    #[test]
    fn test_table_builder() {
        let mut table = Table::new("users");
        table.add_column(Column::new("id", "integer").primary_key().not_null());
        table.add_column(Column::new("name", "text").not_null());

        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);
        assert!(table.columns.contains_key("id"));
        assert!(table.columns.contains_key("name"));
    }

    #[test]
    fn test_column_serialization() {
        let col = Column::new("id", "integer")
            .primary_key()
            .not_null()
            .autoincrement();

        let json = serde_json::to_string(&col).unwrap();
        assert!(json.contains("\"primaryKey\":true"));
        assert!(json.contains("\"notNull\":true"));
        assert!(json.contains("\"autoincrement\":true"));

        let parsed: Column = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, col);
    }
}
