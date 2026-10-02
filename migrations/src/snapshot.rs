//! The generic snapshot document stored as `snapshot.json`.
//!
//! [`SQLiteSnapshot`](crate::sqlite::SQLiteSnapshot),
//! [`PostgresSnapshot`](crate::postgres::PostgresSnapshot), and
//! [`MySQLSnapshot`](crate::mysql::MySQLSnapshot) are aliases of
//! [`Snapshot<E>`] with each dialect's entity enum as `E`. The shared fields
//! and the JSON load/save live here; dialect-specific methods are added in
//! the per-dialect modules.
//!
//! Not to be confused with [`crate::Snapshot`], the enum that wraps one of
//! these when the dialect is only known at run time.

use crate::version::ORIGIN_UUID;
use serde::{Deserialize, Serialize};

/// The dialect name and format version that [`Snapshot::new`] stamps into a
/// snapshot of entity type `E`.
pub trait SnapshotEntity {
    /// Dialect identifier serialized into the `dialect` field
    /// (`"sqlite"`, `"postgresql"`, or `"mysql"`).
    const DIALECT: &'static str;
    /// Snapshot format version serialized into the `version` field
    /// (e.g. `"7"` for SQLite, `"8"` for Postgres, or `"6"` for MySQL).
    const SNAPSHOT_VERSION: &'static str;
}

/// A schema snapshot: format version, dialect, chain IDs, and a flat list of
/// DDL entities.
///
/// Serialized with camelCase keys (`prevIds`) and an `entityType`-tagged
/// `ddl` array, matching drizzle-kit's current snapshot format.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot<E> {
    /// Snapshot format version (e.g. `"7"`).
    pub version: String,
    /// Dialect identifier (`"sqlite"`, `"postgresql"`, or `"mysql"`).
    pub dialect: String,
    /// Unique ID for this snapshot.
    pub id: String,
    /// IDs of the snapshots this one follows; [`ORIGIN_UUID`] for the first.
    pub prev_ids: Vec<String>,
    /// DDL entities (tables, columns, indexes, ...).
    pub ddl: Vec<E>,
    /// drizzle-kit's rename log. Kept for format compatibility; the differ
    /// takes renames from [`DiffOptions`](crate::DiffOptions) instead.
    #[serde(default)]
    pub renames: Vec<String>,
}

impl<E: SnapshotEntity> Default for Snapshot<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: SnapshotEntity> Snapshot<E> {
    /// Creates an empty snapshot with a fresh random ID that follows
    /// [`ORIGIN_UUID`].
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

    /// Creates an empty snapshot that follows `prev_ids`.
    #[must_use]
    pub fn with_prev_ids(prev_ids: Vec<String>) -> Self {
        let mut snapshot = Self::new();
        snapshot.prev_ids = prev_ids;
        snapshot
    }
}

impl<E> Snapshot<E> {
    /// Appends an entity to `ddl`.
    pub fn add_entity(&mut self, entity: E) {
        self.ddl.push(entity);
    }

    /// Returns `true` if `ddl` is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.ddl.is_empty()
    }
}

impl<E> Snapshot<E>
where
    E: Serialize + for<'de> Deserialize<'de>,
{
    /// Parses a snapshot from JSON.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if `json` is not a valid snapshot
    /// document for this dialect.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serializes the snapshot to pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if the snapshot cannot be serialized.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Reads a snapshot from the JSON file at `path`.
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

    /// Writes the snapshot as pretty-printed JSON to `path`, creating parent
    /// folders.
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
