//! PostgreSQL table schema types matching drizzle-kit format

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Column metadata for PostgreSQL
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Column name
    pub name: String,
    /// SQL type
    #[serde(rename = "type")]
    pub sql_type: String,
    /// Type schema (for custom types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_schema: Option<String>,
    /// Is this column a primary key?
    pub primary_key: bool,
    /// Is this column NOT NULL?
    pub not_null: bool,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Is this column unique?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unique: Option<bool>,
    /// Unique constraint name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_name: Option<String>,
    /// Nulls not distinct for unique constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nulls_not_distinct: Option<bool>,
    /// Generated column configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<Generated>,
    /// Identity column configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
}

impl Column {
    /// Create a new column
    pub fn new(name: impl Into<String>, sql_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql_type: sql_type.into(),
            type_schema: None,
            primary_key: false,
            not_null: false,
            default: None,
            is_unique: None,
            unique_name: None,
            nulls_not_distinct: None,
            generated: None,
            identity: None,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.is_unique = Some(true);
        self
    }
}

/// Generated column configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Generated {
    #[serde(rename = "type")]
    pub gen_type: String, // "stored"
    #[serde(rename = "as")]
    pub expression: String,
}

/// Identity column configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    /// Identity type: "always" or "byDefault"
    #[serde(rename = "type")]
    pub identity_type: String,
    /// Sequence name
    pub name: String,
    /// Schema
    pub schema: String,
    /// Increment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increment: Option<String>,
    /// Min value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    /// Max value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<String>,
    /// Start value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_with: Option<String>,
    /// Cache size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    /// Cycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<bool>,
}

/// Index column specification
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexColumn {
    /// Column expression
    pub expression: String,
    /// Is this an expression (vs column name)?
    pub is_expression: bool,
    /// Ascending order?
    pub asc: bool,
    /// Nulls ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nulls: Option<String>,
    /// Operator class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opclass: Option<String>,
}

/// Index metadata
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    /// Index name
    pub name: String,
    /// Index columns
    pub columns: Vec<IndexColumn>,
    /// Is unique?
    pub is_unique: bool,
    /// Index method (btree, hash, gin, gist, etc.)
    #[serde(default = "default_method")]
    pub method: String,
    /// WITH options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<HashMap<String, serde_json::Value>>,
    /// WHERE clause for partial index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    /// Created concurrently?
    #[serde(default)]
    pub concurrently: bool,
}

fn default_method() -> String {
    "btree".to_string()
}

/// Foreign key constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    pub name: String,
    pub table_from: String,
    pub columns_from: Vec<String>,
    pub table_to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_to: Option<String>,
    pub columns_to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<String>,
}

/// Check constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CheckConstraint {
    pub name: String,
    pub value: String,
}

/// Unique constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UniqueConstraint {
    pub name: String,
    pub columns: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nulls_not_distinct: Option<bool>,
}

/// Composite primary key
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CompositePK {
    pub name: String,
    pub columns: Vec<String>,
}

/// PostgreSQL enum type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Enum {
    pub name: String,
    pub schema: String,
    pub values: Vec<String>,
}

/// Sequence
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub name: String,
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<bool>,
}

/// Table metadata
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,
    #[serde(default)]
    pub schema: String,
    pub columns: HashMap<String, Column>,
    pub indexes: HashMap<String, Index>,
    pub foreign_keys: HashMap<String, ForeignKey>,
    pub composite_primary_keys: HashMap<String, CompositePK>,
    #[serde(default)]
    pub unique_constraints: HashMap<String, UniqueConstraint>,
    #[serde(default)]
    pub check_constraints: HashMap<String, CheckConstraint>,
}

impl Table {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: "public".to_string(),
            columns: HashMap::new(),
            indexes: HashMap::new(),
            foreign_keys: HashMap::new(),
            composite_primary_keys: HashMap::new(),
            unique_constraints: HashMap::new(),
            check_constraints: HashMap::new(),
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = schema.into();
        self
    }

    pub fn add_column(&mut self, column: Column) {
        self.columns.insert(column.name.clone(), column);
    }
}

/// View metadata
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub name: String,
    pub schema: String,
    pub columns: HashMap<String, Column>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    pub is_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materialized: Option<bool>,
}
