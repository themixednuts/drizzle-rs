//! Snapshot management - reading/writing schema snapshots
//!
//! Snapshots are JSON files that capture the complete state of the schema
//! at a particular migration. They follow the drizzle-kit format.

use crate::error::CliError;
use drizzle_migrations::sqlite::SQLiteSnapshot;
use std::path::Path;

/// Load the latest snapshot from the migrations meta directory
pub fn load_latest_snapshot(migrations_dir: &Path, dialect: &str) -> Result<Option<SQLiteSnapshot>, CliError> {
    let meta_dir = migrations_dir.join("meta");
    
    if !meta_dir.exists() {
        return Ok(None);
    }

    // Read the journal to find the latest snapshot
    let journal_path = meta_dir.join("_journal.json");
    if !journal_path.exists() {
        return Ok(None);
    }

    let journal_content = std::fs::read_to_string(&journal_path)?;
    let journal: drizzle_migrations::Journal = serde_json::from_str(&journal_content)?;

    if journal.entries.is_empty() {
        return Ok(None);
    }

    // Get the last entry
    let last_entry = journal.entries.last().unwrap();
    let snapshot_name = format!("{:04}_snapshot.json", last_entry.idx);
    let snapshot_path = meta_dir.join(&snapshot_name);

    if !snapshot_path.exists() {
        return Err(CliError::SnapshotNotFound(snapshot_path.display().to_string()));
    }

    let snapshot_content = std::fs::read_to_string(&snapshot_path)?;
    
    match dialect {
        "sqlite" | "turso" | "libsql" => {
            let snapshot: SQLiteSnapshot = serde_json::from_str(&snapshot_content)?;
            Ok(Some(snapshot))
        }
        _ => Err(CliError::InvalidDialect(dialect.to_string())),
    }
}

/// Save a snapshot to the meta directory
pub fn save_snapshot(
    migrations_dir: &Path,
    snapshot: &SQLiteSnapshot,
    idx: u32,
) -> Result<(), CliError> {
    let meta_dir = migrations_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;

    let snapshot_name = format!("{:04}_snapshot.json", idx);
    let snapshot_path = meta_dir.join(&snapshot_name);

    let content = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&snapshot_path, content)?;

    Ok(())
}

/// Create an empty snapshot for the given dialect
pub fn empty_snapshot(dialect: &str) -> Result<SQLiteSnapshot, CliError> {
    match dialect {
        "sqlite" | "turso" | "libsql" => Ok(SQLiteSnapshot::new()),
        _ => Err(CliError::InvalidDialect(dialect.to_string())),
    }
}

