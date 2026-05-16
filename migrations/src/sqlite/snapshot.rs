//! `SQLite` snapshot type matching drizzle-kit format.
//!
//! `SQLiteSnapshot` is a type alias of the generic [`crate::snapshot::Snapshot`]
//! — the CRUD / serde IO surface lives once in that module. This file
//! supplies only:
//!
//! * the [`SnapshotEntity`] impl that pins the SQLite dialect / version
//!   constants used by `Snapshot::new()`;
//! * the legacy v6 types preserved for reading old snapshots.

use super::ddl::SqliteEntity;
use crate::snapshot::{Snapshot, SnapshotEntity};
use crate::version::SQLITE_SNAPSHOT_VERSION;
use serde::{Deserialize, Serialize};

impl SnapshotEntity for SqliteEntity {
    const DIALECT: &'static str = "sqlite";
    const SNAPSHOT_VERSION: &'static str = SQLITE_SNAPSHOT_VERSION;
}

/// `SQLite` schema snapshot — drizzle-kit beta v7 format.
///
/// Type alias of [`Snapshot<SqliteEntity>`]; see the generic type's docs
/// for the field set and IO surface.
pub type SQLiteSnapshot = Snapshot<SqliteEntity>;

// =============================================================================
// Legacy V6 Snapshot Format (for reading old snapshots)
// =============================================================================

use super::ddl::{Table, View};
use std::collections::HashMap;

/// Schema metadata for tracking renames (legacy v6 format)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Meta {
    /// Table renames: `old_name` -> `new_name`
    #[serde(default)]
    pub tables: HashMap<String, String>,
    /// Column renames: "`table.old_column`" -> "`table.new_column`"
    #[serde(default)]
    pub columns: HashMap<String, String>,
}

/// Internal kit metadata (legacy v6 format)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Internal {
    /// Index-specific internals
    #[serde(default)]
    pub indexes: HashMap<String, IndexInternal>,
}

/// Internal index metadata (legacy v6 format)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IndexInternal {
    /// Column-specific metadata
    #[serde(default)]
    pub columns: HashMap<String, ColumnInternal>,
}

/// Internal column metadata (legacy v6 format)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInternal {
    /// Is this column an expression?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_expression: Option<bool>,
}

/// Legacy `SQLite` schema snapshot - v6 format (for reading old snapshots)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SQLiteSnapshotV6 {
    /// Schema version ("6")
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
    /// Enums (empty for `SQLite`, kept for compatibility with drizzle-kit's
    /// cross-dialect v6 format).
    #[serde(default)]
    pub enums: HashMap<String, serde_json::Value>,
    /// Metadata for tracking renames
    #[serde(rename = "_meta")]
    pub meta: Meta,
    /// Internal kit metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<Internal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::ddl::{Column, Table};

    #[test]
    fn test_new_snapshot() {
        let snapshot = SQLiteSnapshot::new();
        assert_eq!(snapshot.version, "7");
        assert_eq!(snapshot.dialect, "sqlite");
        assert_eq!(snapshot.prev_ids[0], crate::ORIGIN_UUID);
        assert!(snapshot.ddl.is_empty());
    }

    #[test]
    fn test_add_entity() {
        let mut snapshot = SQLiteSnapshot::new();

        // Add a table entity
        let table = Table::new("users");
        snapshot.add_entity(SqliteEntity::Table(table));

        // Add column entities
        let id_col = Column::new("users", "id", "integer")
            .not_null()
            .autoincrement();
        let name_col = Column::new("users", "name", "text").not_null();

        snapshot.add_entity(SqliteEntity::Column(id_col));
        snapshot.add_entity(SqliteEntity::Column(name_col));

        assert_eq!(snapshot.ddl.len(), 3);
    }

    #[test]
    fn test_snapshot_serialization() {
        let mut snapshot = SQLiteSnapshot::new();

        let table = Table::new("users");
        snapshot.add_entity(SqliteEntity::Table(table));

        let col = Column::new("users", "id", "integer").not_null();
        snapshot.add_entity(SqliteEntity::Column(col));

        let json = snapshot.to_json().unwrap();

        // Verify round-trip via structured comparison
        let parsed = SQLiteSnapshot::from_json(&json).unwrap();
        assert_eq!(parsed.version, "7");
        assert_eq!(parsed.dialect, "sqlite");
        assert_eq!(parsed.ddl.len(), 2);

        // Verify JSON structure via serde_json::Value
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["version"], "7");
        assert_eq!(value["dialect"], "sqlite");
        assert_eq!(value["ddl"][0]["entityType"], "tables");
        assert_eq!(value["ddl"][1]["entityType"], "columns");
    }

    #[test]
    fn test_column_json_format_matches_drizzle_kit() {
        // Create a column with autoincrement to verify field naming
        let col = Column::new("users", "id", "integer")
            .not_null()
            .autoincrement();

        let value: serde_json::Value = serde_json::to_value(&col).unwrap();

        // Verify field names match drizzle-kit exactly:
        // - autoincrement (not autoIncrement)
        // - notNull (camelCase)
        // - type (renamed from sql_type)
        assert_eq!(value["autoincrement"], serde_json::json!(true));
        assert_eq!(value["notNull"], serde_json::json!(true));
        assert_eq!(value["type"], "integer");
        assert_eq!(value["table"], "users");
        assert_eq!(value["name"], "id");

        // Verify it doesn't contain snake_case versions
        assert!(
            value.get("sql_type").is_none(),
            "Should not contain 'sql_type'"
        );
        assert!(
            value.get("not_null").is_none(),
            "Should not contain 'not_null'"
        );
    }
}
