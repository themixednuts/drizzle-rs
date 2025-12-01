//! PostgreSQL snapshot types matching drizzle-kit format

use super::{Enum, Sequence, Table, View};
use crate::version::{ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema metadata for tracking renames
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Meta {
    #[serde(default)]
    pub schemas: HashMap<String, String>,
    #[serde(default)]
    pub tables: HashMap<String, String>,
    #[serde(default)]
    pub columns: HashMap<String, String>,
}

/// PostgreSQL schema snapshot (version 7)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSnapshot {
    pub version: String,
    pub dialect: String,
    pub id: String,
    pub prev_id: String,
    pub tables: HashMap<String, Table>,
    pub enums: HashMap<String, Enum>,
    pub schemas: HashMap<String, Schema>,
    pub sequences: HashMap<String, Sequence>,
    #[serde(default)]
    pub views: HashMap<String, View>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

/// PostgreSQL schema (namespace)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    pub name: String,
}

impl Default for PostgresSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresSnapshot {
    pub fn new() -> Self {
        let mut schemas = HashMap::new();
        schemas.insert(
            "public".to_string(),
            Schema {
                name: "public".to_string(),
            },
        );

        Self {
            version: POSTGRES_SNAPSHOT_VERSION.to_string(),
            dialect: "postgresql".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            prev_id: ORIGIN_UUID.to_string(),
            tables: HashMap::new(),
            enums: HashMap::new(),
            schemas,
            sequences: HashMap::new(),
            views: HashMap::new(),
            meta: Meta::default(),
        }
    }

    pub fn with_prev_id(prev_id: impl Into<String>) -> Self {
        let mut snapshot = Self::new();
        snapshot.prev_id = prev_id.into();
        snapshot
    }

    pub fn add_table(&mut self, table: Table) {
        let key = format!("{}.{}", table.schema, table.name);
        self.tables.insert(key, table);
    }

    pub fn add_enum(&mut self, e: Enum) {
        let key = format!("{}.{}", e.schema, e.name);
        self.enums.insert(key, e);
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

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty() && self.views.is_empty() && self.enums.is_empty()
    }
}
