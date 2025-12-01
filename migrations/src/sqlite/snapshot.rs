//! SQLite snapshot types matching drizzle-kit format

use super::{Table, View};
use crate::version::{ORIGIN_UUID, SQLITE_SNAPSHOT_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema metadata for tracking renames
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Meta {
    /// Table renames: old_name -> new_name
    #[serde(default)]
    pub tables: HashMap<String, String>,
    /// Column renames: "table.old_column" -> "table.new_column"
    #[serde(default)]
    pub columns: HashMap<String, String>,
}

/// Internal kit metadata
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Internal {
    /// Index-specific internals
    #[serde(default)]
    pub indexes: HashMap<String, IndexInternal>,
}

/// Internal index metadata
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IndexInternal {
    /// Column-specific metadata
    #[serde(default)]
    pub columns: HashMap<String, ColumnInternal>,
}

/// Internal column metadata
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInternal {
    /// Is this column an expression?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_expression: Option<bool>,
}

/// SQLite schema snapshot - matches drizzle-kit format (version 6)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SQLiteSnapshot {
    /// Schema version (currently "6")
    pub version: String,
    /// Dialect identifier
    pub dialect: String,
    /// Unique ID for this snapshot
    pub id: String,
    /// ID of the previous snapshot in the chain
    pub prev_id: String,
    /// Tables in the schema
    pub tables: HashMap<String, Table>,
    /// Views in the schema
    #[serde(default)]
    pub views: HashMap<String, View>,
    /// Enums (empty for SQLite, kept for compatibility)
    #[serde(default)]
    pub enums: HashMap<String, ()>,
    /// Metadata for tracking renames
    #[serde(rename = "_meta")]
    pub meta: Meta,
    /// Internal kit metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<Internal>,
}

impl Default for SQLiteSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl SQLiteSnapshot {
    /// Create a new empty SQLite snapshot
    pub fn new() -> Self {
        Self {
            version: SQLITE_SNAPSHOT_VERSION.to_string(),
            dialect: "sqlite".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            prev_id: ORIGIN_UUID.to_string(),
            tables: HashMap::new(),
            views: HashMap::new(),
            enums: HashMap::new(),
            meta: Meta::default(),
            internal: None,
        }
    }

    /// Create a new snapshot with a specific previous ID
    pub fn with_prev_id(prev_id: impl Into<String>) -> Self {
        let mut snapshot = Self::new();
        snapshot.prev_id = prev_id.into();
        snapshot
    }

    /// Add a table to the snapshot
    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }

    /// Add a view to the snapshot
    pub fn add_view(&mut self, view: View) {
        self.views.insert(view.name.clone(), view);
    }

    /// Load snapshot from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize snapshot to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load snapshot from file
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save snapshot to file
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, json)
    }

    /// Check if this snapshot is empty (no tables, views, etc.)
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty() && self.views.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::Column;

    #[test]
    fn test_new_snapshot() {
        let snapshot = SQLiteSnapshot::new();
        assert_eq!(snapshot.version, "6");
        assert_eq!(snapshot.dialect, "sqlite");
        assert_eq!(snapshot.prev_id, crate::ORIGIN_UUID);
        assert!(snapshot.tables.is_empty());
    }

    #[test]
    fn test_add_table() {
        let mut snapshot = SQLiteSnapshot::new();
        let mut table = Table::new("users");
        table.add_column(Column::new("id", "integer").primary_key().not_null());
        table.add_column(Column::new("name", "text").not_null());

        snapshot.add_table(table);

        assert_eq!(snapshot.tables.len(), 1);
        assert!(snapshot.tables.contains_key("users"));
    }

    #[test]
    fn test_snapshot_serialization() {
        let mut snapshot = SQLiteSnapshot::new();
        let mut table = Table::new("users");
        table.add_column(Column::new("id", "integer").primary_key().not_null());
        snapshot.add_table(table);

        let json = snapshot.to_json().unwrap();

        // Verify it contains expected fields (pretty print adds spaces)
        assert!(json.contains("\"version\": \"6\""));
        assert!(json.contains("\"dialect\": \"sqlite\""));
        assert!(json.contains("\"users\""));

        // Verify round-trip
        let parsed = SQLiteSnapshot::from_json(&json).unwrap();
        assert_eq!(parsed.version, snapshot.version);
        assert_eq!(parsed.tables.len(), 1);
    }
}
