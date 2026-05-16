//! Generic `Snapshot<E>` — shared CRUD + serde IO across dialects.
//!
//! Per-dialect snapshot types (`SQLiteSnapshot`, `PostgresSnapshot`) are
//! type aliases of this generic struct. The dialect-neutral pieces — the
//! field set (`version`, `dialect`, `id`, `prev_ids`, `ddl`, `renames`),
//! the JSON round-trip, the file load/save — live here once. Dialect-
//! specific methods (e.g. Postgres's `scoped_to_tables`,
//! `filter_serial_sequences`) attach via `impl Snapshot<PostgresEntity>`
//! blocks in the per-dialect modules, which orphan rules permit because
//! the entity type is local to this crate.
//!
//! The cross-dialect `migrations::Snapshot` enum (at
//! [`crate::schema::Snapshot`]) is unrelated — it wraps either of the
//! concrete aliases for callers that don't know the dialect at compile
//! time. The two share a name but live at different module paths.

use crate::version::ORIGIN_UUID;
use serde::{Deserialize, Serialize};

/// Per-dialect metadata for [`Snapshot<E>`].
///
/// Lets the generic `new()` constructor stamp the right `version` and
/// `dialect` strings without knowing which dialect it's working with.
pub trait SnapshotEntity {
    /// Dialect identifier serialized into the `dialect` field
    /// (e.g. `"sqlite"`, `"postgres"`).
    const DIALECT: &'static str;
    /// Snapshot format version serialized into the `version` field
    /// (e.g. `"7"` for SQLite, `"8"` for Postgres).
    const SNAPSHOT_VERSION: &'static str;
}

/// Generic schema snapshot keyed on an entity type.
///
/// The `entity_type`-tagged DDL array is the v7+ drizzle-kit format;
/// `Vec<E>` lets each dialect supply its own entity enum.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot<E> {
    /// Snapshot format version (e.g. `"7"`).
    pub version: String,
    /// Dialect identifier (e.g. `"sqlite"`, `"postgres"`).
    pub dialect: String,
    /// Unique ID for this snapshot.
    pub id: String,
    /// IDs of previous snapshots in the chain.
    pub prev_ids: Vec<String>,
    /// DDL entities (tables, columns, indexes, ...).
    pub ddl: Vec<E>,
    /// Tracked renames for migration generation.
    #[serde(default)]
    pub renames: Vec<String>,
}

impl<E: SnapshotEntity> Default for Snapshot<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: SnapshotEntity> Snapshot<E> {
    /// Create a new empty snapshot stamped with this entity's dialect /
    /// snapshot-version constants.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: E::SNAPSHOT_VERSION.to_string(),
            dialect: E::DIALECT.to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            prev_ids: vec![ORIGIN_UUID.to_string()],
            ddl: Vec::new(),
            renames: Vec::new(),
        }
    }

    /// Create a new snapshot with specific previous IDs.
    #[must_use]
    pub fn with_prev_ids(prev_ids: Vec<String>) -> Self {
        let mut snapshot = Self::new();
        snapshot.prev_ids = prev_ids;
        snapshot
    }
}

impl<E> Snapshot<E> {
    /// Add an entity to the DDL array.
    pub fn add_entity(&mut self, entity: E) {
        self.ddl.push(entity);
    }

    /// True if the snapshot has no DDL entities.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.ddl.is_empty()
    }
}

impl<E> Snapshot<E>
where
    E: Serialize + for<'de> Deserialize<'de>,
{
    /// Load a snapshot from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if `json` is not a valid snapshot
    /// document for this dialect.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize the snapshot to a pretty-printed JSON string.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if the snapshot cannot be serialized.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load a snapshot from a file.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if the file cannot be read, or
    /// [`std::io::ErrorKind::InvalidData`] wrapping the underlying
    /// [`serde_json::Error`] if the contents cannot be parsed.
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save the snapshot to a file (creating parent directories as needed).
    ///
    /// # Errors
    ///
    /// Returns [`std::io::ErrorKind::InvalidData`] wrapping the underlying
    /// [`serde_json::Error`] if serialization fails, or any other
    /// [`std::io::Error`] produced while creating the parent directory or
    /// writing the file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, json)
    }
}
