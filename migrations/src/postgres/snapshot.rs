//! PostgreSQL snapshot types matching drizzle-kit format

use crate::postgres::ddl::PostgresEntity;
use crate::version::{ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PostgreSQL schema snapshot (version 8 - drizzle-kit beta)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSnapshot {
    pub version: String,
    pub dialect: String,
    pub id: String,
    pub prev_ids: Vec<String>,
    pub ddl: Vec<PostgresEntity>,
}

impl Default for PostgresSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresSnapshot {
    pub fn new() -> Self {
        Self {
            version: POSTGRES_SNAPSHOT_VERSION.to_string(),
            dialect: "postgresql".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            prev_ids: vec![ORIGIN_UUID.to_string()],
            ddl: Vec::new(),
        }
    }

    pub fn with_prev_ids(prev_ids: Vec<String>) -> Self {
        let mut snapshot = Self::new();
        snapshot.prev_ids = prev_ids;
        snapshot
    }

    pub fn add_entity(&mut self, entity: PostgresEntity) {
        self.ddl.push(entity);
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, json)
    }
}

// =============================================================================
// Legacy V7 Snapshot (drizzle-kit stable)
// =============================================================================

/// Schema metadata for tracking renames (Legacy)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Meta {
    #[serde(default)]
    pub schemas: HashMap<String, String>,
    #[serde(default)]
    pub tables: HashMap<String, String>,
    #[serde(default)]
    pub columns: HashMap<String, String>,
}

/// Legacy V7 Snapshot for reading compatibility
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSnapshotV7 {
    pub version: String,
    pub dialect: String,
    pub id: String,
    pub prev_id: String,
    // Using Value for legacy details to avoid redefining all legacy structs
    pub tables: HashMap<String, serde_json::Value>,
    pub enums: HashMap<String, serde_json::Value>,
    pub schemas: HashMap<String, serde_json::Value>,
    pub sequences: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub views: HashMap<String, serde_json::Value>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::ddl::{Schema, Table};

    #[test]
    fn test_new_snapshot() {
        let snapshot = PostgresSnapshot::new();
        assert_eq!(snapshot.version, "8");
        assert_eq!(snapshot.dialect, "postgresql");
        assert_eq!(snapshot.prev_ids, vec![ORIGIN_UUID]);
        assert!(snapshot.ddl.is_empty());
    }

    #[test]
    fn test_add_entity() {
        let mut snapshot = PostgresSnapshot::new();

        let schema = Schema {
            name: "public".to_string(),
        };
        snapshot.add_entity(PostgresEntity::Schema(schema));

        let table = Table {
            schema: "public".to_string(),
            name: "users".to_string(),
            is_rls_enabled: None,
        };
        snapshot.add_entity(PostgresEntity::Table(table));

        assert_eq!(snapshot.ddl.len(), 2);
    }
}
