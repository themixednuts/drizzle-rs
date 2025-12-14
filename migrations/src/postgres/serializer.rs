//! PostgreSQL schema serialization
//!
//! This module provides functionality to serialize Drizzle schema definitions
//! into DDL entities and snapshots.

use super::collection::PostgresDDL;
use super::ddl::PostgresEntity;
use super::snapshot::PostgresSnapshot;
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

/// Result of preparing PostgreSQL snapshots for migration
#[derive(Debug, Clone)]
pub struct PreparedSnapshots {
    /// Previous DDL state
    pub ddl_prev: PostgresDDL,
    /// Current DDL state
    pub ddl_cur: PostgresDDL,
    /// Current snapshot to be written
    pub snapshot: PostgresSnapshot,
    /// Previous snapshot (read from file)
    pub snapshot_prev: PostgresSnapshot,
}

/// Load a snapshot from a JSON file
pub fn load_snapshot(path: &Path) -> SerializerResult<PostgresSnapshot> {
    let contents = std::fs::read_to_string(path).map_err(|e| SerializerError {
        message: format!("Failed to read snapshot file: {}", e),
        path: Some(path.display().to_string()),
    })?;

    serde_json::from_str::<PostgresSnapshot>(&contents).map_err(|e| SerializerError {
        message: format!("Failed to parse snapshot: {}", e),
        path: Some(path.display().to_string()),
    })
}

/// Save a snapshot to a JSON file
pub fn save_snapshot(snapshot: &PostgresSnapshot, path: &Path) -> SerializerResult<()> {
    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SerializerError {
            message: format!("Failed to create directory: {}", e),
            path: Some(parent.display().to_string()),
        })?;
    }

    let json = serde_json::to_string_pretty(snapshot).map_err(|e| SerializerError {
        message: format!("Failed to serialize snapshot: {}", e),
        path: Some(path.display().to_string()),
    })?;

    std::fs::write(path, json).map_err(|e| SerializerError {
        message: format!("Failed to write snapshot file: {}", e),
        path: Some(path.display().to_string()),
    })?;

    Ok(())
}

/// Load the latest snapshot from a drizzle folder
pub fn load_latest_snapshot(drizzle_folder: &Path) -> SerializerResult<Option<PostgresSnapshot>> {
    let meta_folder = drizzle_folder.join("meta");
    let journal_path = meta_folder.join("_journal.json");

    if !journal_path.exists() {
        return Ok(None);
    }

    // Read journal to find latest snapshot
    let journal_contents = std::fs::read_to_string(&journal_path).map_err(|e| SerializerError {
        message: format!("Failed to read journal: {}", e),
        path: Some(journal_path.display().to_string()),
    })?;

    let journal: serde_json::Value =
        serde_json::from_str(&journal_contents).map_err(|e| SerializerError {
            message: format!("Failed to parse journal: {}", e),
            path: Some(journal_path.display().to_string()),
        })?;

    let entries = journal["entries"]
        .as_array()
        .ok_or_else(|| SerializerError {
            message: "Invalid journal format: missing entries array".to_string(),
            path: Some(journal_path.display().to_string()),
        })?;

    if entries.is_empty() {
        return Ok(None);
    }

    // Get the last entry's tag
    let last_entry = entries.last().unwrap();
    let tag = last_entry["tag"].as_str().ok_or_else(|| SerializerError {
        message: "Invalid journal entry: missing tag".to_string(),
        path: Some(journal_path.display().to_string()),
    })?;

    let snapshot_path = meta_folder.join(format!("{}_snapshot.json", tag));
    load_snapshot(&snapshot_path).map(Some)
}

/// Find all snapshot files in a drizzle folder
pub fn find_snapshot_files(drizzle_folder: &Path) -> SerializerResult<Vec<std::path::PathBuf>> {
    let meta_folder = drizzle_folder.join("meta");

    if !meta_folder.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();

    let entries = std::fs::read_dir(&meta_folder).map_err(|e| SerializerError {
        message: format!("Failed to read meta folder: {}", e),
        path: Some(meta_folder.display().to_string()),
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("_snapshot.json") && name != "_journal.json" {
                snapshots.push(path);
            }
        }
    }

    // Sort by filename (which includes timestamp)
    snapshots.sort();

    Ok(snapshots)
}

/// Prepare snapshots for migration generation
pub fn prepare_snapshots(
    drizzle_folder: &Path,
    current_ddl: PostgresDDL,
) -> SerializerResult<PreparedSnapshots> {
    // Load previous snapshot if exists
    let snapshot_prev = load_latest_snapshot(drizzle_folder)?.unwrap_or_else(PostgresSnapshot::new);

    // Build DDL from previous snapshot
    let ddl_prev = PostgresDDL::from_entities(snapshot_prev.ddl.clone());

    // Create new snapshot from current DDL
    let mut snapshot = PostgresSnapshot::with_prev_ids(vec![snapshot_prev.id.clone()]);
    snapshot.ddl = current_ddl.to_entities();

    Ok(PreparedSnapshots {
        ddl_prev,
        ddl_cur: current_ddl,
        snapshot,
        snapshot_prev,
    })
}

/// Create an empty/dry snapshot (for initial migrations)
pub fn empty_snapshot() -> PostgresSnapshot {
    PostgresSnapshot::new()
}

/// Create a DDL from a list of entities
pub fn ddl_from_entities(entities: Vec<PostgresEntity>) -> PostgresDDL {
    PostgresDDL::from_entities(entities)
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
        assert_eq!(snapshot.version, "8");
    }

    #[test]
    fn test_save_and_load_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_path = temp_dir.path().join("test_snapshot.json");

        let snapshot = PostgresSnapshot::new();
        save_snapshot(&snapshot, &snapshot_path).unwrap();

        let loaded = load_snapshot(&snapshot_path).unwrap();
        assert_eq!(loaded.version, snapshot.version);
        assert_eq!(loaded.id, snapshot.id);
    }

    #[test]
    fn test_find_snapshot_files() {
        let temp_dir = TempDir::new().unwrap();
        let meta_dir = temp_dir.path().join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        // Create some snapshot files
        let mut f1 = std::fs::File::create(meta_dir.join("0001_snapshot.json")).unwrap();
        f1.write_all(b"{}").unwrap();

        let mut f2 = std::fs::File::create(meta_dir.join("0002_snapshot.json")).unwrap();
        f2.write_all(b"{}").unwrap();

        let snapshots = find_snapshot_files(temp_dir.path()).unwrap();
        assert_eq!(snapshots.len(), 2);
    }
}
