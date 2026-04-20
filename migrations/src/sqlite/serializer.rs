//! `SQLite` schema serialization
//!
//! This module provides functionality to serialize Drizzle schema definitions
//! into DDL entities and snapshots.

use super::collection::SQLiteDDL;
use super::ddl::SqliteEntity;
use super::snapshot::{SQLiteSnapshot, SQLiteSnapshotV6};
use std::path::Path;

/// Error type for serialization operations
#[derive(Debug, Clone)]
pub struct SerializerError {
    pub message: String,
    pub path: Option<String>,
}

impl std::fmt::Display for SerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "Serialization error in '{}': {}", path, self.message)
        } else {
            write!(f, "Serialization error: {}", self.message)
        }
    }
}

impl std::error::Error for SerializerError {}

/// Result type for serialization
pub type SerializerResult<T> = Result<T, SerializerError>;

/// Result of preparing `SQLite` snapshots for migration
#[derive(Debug, Clone)]
pub struct PreparedSnapshots {
    /// Previous DDL state
    pub ddl_prev: SQLiteDDL,
    /// Current DDL state
    pub ddl_cur: SQLiteDDL,
    /// Current snapshot to be written
    pub snapshot: SQLiteSnapshot,
    /// Previous snapshot (read from file)
    pub snapshot_prev: SQLiteSnapshot,
}

/// Load a snapshot from a JSON file.
///
/// # Errors
///
/// Returns a [`SerializerError`] if the file cannot be read or the contents
/// cannot be parsed as either a v6 or v7 [`SQLiteSnapshot`].
pub fn load_snapshot(path: &Path) -> SerializerResult<SQLiteSnapshot> {
    let contents = std::fs::read_to_string(path).map_err(|e| SerializerError {
        message: format!("Failed to read snapshot file: {e}"),
        path: Some(path.display().to_string()),
    })?;

    // Try parsing as v7 first
    if let Ok(snapshot) = serde_json::from_str::<SQLiteSnapshot>(&contents) {
        return Ok(snapshot);
    }

    // Try parsing as v6 and upgrading
    if let Ok(v6) = serde_json::from_str::<SQLiteSnapshotV6>(&contents) {
        return Ok(upgrade_v6_to_v7(v6));
    }

    Err(SerializerError {
        message: "Failed to parse snapshot as v6 or v7 format".to_string(),
        path: Some(path.display().to_string()),
    })
}

/// Save a snapshot to a JSON file.
///
/// # Errors
///
/// Returns a [`SerializerError`] if the parent directory cannot be created,
/// the snapshot cannot be serialized, or the file cannot be written.
pub fn save_snapshot(snapshot: &SQLiteSnapshot, path: &Path) -> SerializerResult<()> {
    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SerializerError {
            message: format!("Failed to create directory: {e}"),
            path: Some(parent.display().to_string()),
        })?;
    }

    let json = serde_json::to_string_pretty(snapshot).map_err(|e| SerializerError {
        message: format!("Failed to serialize snapshot: {e}"),
        path: Some(path.display().to_string()),
    })?;

    std::fs::write(path, json).map_err(|e| SerializerError {
        message: format!("Failed to write snapshot file: {e}"),
        path: Some(path.display().to_string()),
    })?;

    Ok(())
}

/// Upgrade a v6 snapshot to v7 format
fn upgrade_v6_to_v7(v6: SQLiteSnapshotV6) -> SQLiteSnapshot {
    let mut snapshot = SQLiteSnapshot::with_prev_ids(vec![v6.prev_id]);
    snapshot.id = v6.id;
    // Note: v6 entities are in a different format; this would need conversion
    // For now, we just create an empty snapshot with the correct IDs
    snapshot
}

/// Load the latest snapshot from a drizzle folder.
///
/// # Errors
///
/// Returns a [`SerializerError`] if the folder cannot be scanned or the
/// latest snapshot file cannot be read/parsed.
pub fn load_latest_snapshot(drizzle_folder: &Path) -> SerializerResult<Option<SQLiteSnapshot>> {
    let snapshots = find_snapshot_files(drizzle_folder)?;
    snapshots.last().map(|path| load_snapshot(path)).transpose()
}

/// Find all snapshot files in a drizzle folder.
///
/// # Errors
///
/// Returns a [`SerializerError`] if the folder exists but cannot be read.
pub fn find_snapshot_files(drizzle_folder: &Path) -> SerializerResult<Vec<std::path::PathBuf>> {
    if !drizzle_folder.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();

    let entries = std::fs::read_dir(drizzle_folder).map_err(|e| SerializerError {
        message: format!("Failed to read migrations folder: {e}"),
        path: Some(drizzle_folder.display().to_string()),
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }

        let snapshot_path = path.join("snapshot.json");
        if snapshot_path.exists() {
            snapshots.push(snapshot_path);
        }
    }

    // Sort by parent folder name (which includes timestamp)
    snapshots.sort();

    Ok(snapshots)
}

/// Prepare snapshots for migration generation.
///
/// # Errors
///
/// Returns a [`SerializerError`] if loading the previous snapshot from
/// `drizzle_folder` fails.
pub fn prepare_snapshots(
    drizzle_folder: &Path,
    current_ddl: SQLiteDDL,
) -> SerializerResult<PreparedSnapshots> {
    // Load previous snapshot if exists
    let snapshot_prev = load_latest_snapshot(drizzle_folder)?.unwrap_or_else(SQLiteSnapshot::new);

    // Build DDL from previous snapshot
    let ddl_prev = SQLiteDDL::from_entities(snapshot_prev.ddl.clone());

    // Create new snapshot from current DDL
    let mut snapshot = SQLiteSnapshot::with_prev_ids(vec![snapshot_prev.id.clone()]);
    snapshot.ddl = current_ddl.to_entities();

    Ok(PreparedSnapshots {
        ddl_prev,
        ddl_cur: current_ddl,
        snapshot,
        snapshot_prev,
    })
}

/// Create an empty/dry snapshot (for initial migrations)
#[must_use]
pub fn empty_snapshot() -> SQLiteSnapshot {
    SQLiteSnapshot::new()
}

/// Create a DDL from a list of entities
#[must_use]
pub fn ddl_from_entities(entities: Vec<SqliteEntity>) -> SQLiteDDL {
    SQLiteDDL::from_entities(entities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_empty_snapshot() {
        let snapshot = empty_snapshot();
        assert!(snapshot.ddl.is_empty());
        assert_eq!(snapshot.version, "7");
    }

    #[test]
    fn test_save_and_load_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_path = temp_dir.path().join("test_snapshot.json");

        let snapshot = SQLiteSnapshot::new();
        save_snapshot(&snapshot, &snapshot_path).unwrap();

        let loaded = load_snapshot(&snapshot_path).unwrap();
        assert_eq!(loaded.version, snapshot.version);
        assert_eq!(loaded.id, snapshot.id);
    }

    #[test]
    fn test_find_snapshot_files() {
        let temp_dir = TempDir::new().unwrap();
        let mig1 = temp_dir.path().join("0001_first");
        let mig2 = temp_dir.path().join("0002_second");
        std::fs::create_dir_all(&mig1).unwrap();
        std::fs::create_dir_all(&mig2).unwrap();

        // Create some snapshot files
        let mut f1 = std::fs::File::create(mig1.join("snapshot.json")).unwrap();
        f1.write_all(b"{}").unwrap();

        let mut f2 = std::fs::File::create(mig2.join("snapshot.json")).unwrap();
        f2.write_all(b"{}").unwrap();

        let snapshots = find_snapshot_files(temp_dir.path()).unwrap();
        assert_eq!(snapshots.len(), 2);
    }
}
