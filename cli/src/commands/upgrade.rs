//! Upgrade command - upgrades migration snapshots to the latest version
//!
//! This command scans the migrations folder and upgrades any old snapshot
//! versions to the latest format, matching drizzle-kit's `up` command.

use crate::config::DrizzleConfig;
use crate::error::CliError;
use colored::Colorize;
use drizzle_migrations::upgrade::upgrade_to_latest;
use drizzle_migrations::version::{is_supported_version, snapshot_version};
use drizzle_types::Dialect;
use std::fs;
use std::path::Path;

/// Run the upgrade command
pub fn run(config: &DrizzleConfig, db_name: Option<&str>) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    let dialect = db.dialect.to_base();
    let out_dir = db.migrations_dir();

    println!(
        "{} Checking for snapshots to upgrade in {}",
        "🔍".bright_cyan(),
        out_dir.display()
    );

    if !out_dir.exists() {
        println!(
            "{} No migrations folder found at {}",
            "ℹ️".bright_blue(),
            out_dir.display()
        );
        return Ok(());
    }

    let upgraded = upgrade_snapshots(out_dir, dialect)?;

    if upgraded == 0 {
        println!(
            "{} All snapshots are already at the latest version ({})",
            "✅".bright_green(),
            snapshot_version(dialect)
        );
    } else {
        println!(
            "{} Upgraded {} snapshot(s) to version {}",
            "✅".bright_green(),
            upgraded,
            snapshot_version(dialect)
        );
    }

    Ok(())
}

/// Upgrade all snapshots in a migrations folder
fn upgrade_snapshots(out_dir: &Path, dialect: Dialect) -> Result<usize, CliError> {
    let mut upgraded_count = 0;

    // Check for V3 folder-based migrations (each folder has snapshot.json)
    let v3_snapshots = find_v3_snapshots(out_dir)?;

    for snapshot_path in v3_snapshots {
        if upgrade_snapshot_file(&snapshot_path, dialect)? {
            upgraded_count += 1;
        }
    }

    // Also check for legacy meta/ folder snapshots
    let meta_folder = out_dir.join("meta");
    if meta_folder.exists() {
        let legacy_snapshots = find_legacy_snapshots(&meta_folder)?;
        for snapshot_path in legacy_snapshots {
            if upgrade_snapshot_file(&snapshot_path, dialect)? {
                upgraded_count += 1;
            }
        }
    }

    Ok(upgraded_count)
}

/// Find V3 format snapshots (folder/snapshot.json)
fn find_v3_snapshots(out_dir: &Path) -> Result<Vec<std::path::PathBuf>, CliError> {
    let mut snapshots = Vec::new();

    if !out_dir.exists() {
        return Ok(snapshots);
    }

    for entry in fs::read_dir(out_dir).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_dir() {
            let snapshot_path = path.join("snapshot.json");
            if snapshot_path.exists() {
                snapshots.push(snapshot_path);
            }
        }
    }

    Ok(snapshots)
}

/// Find legacy format snapshots (meta/*_snapshot.json)
fn find_legacy_snapshots(meta_folder: &Path) -> Result<Vec<std::path::PathBuf>, CliError> {
    let mut snapshots = Vec::new();

    if !meta_folder.exists() {
        return Ok(snapshots);
    }

    for entry in fs::read_dir(meta_folder).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with("_snapshot.json") {
                    snapshots.push(path);
                }
            }
        }
    }

    Ok(snapshots)
}

/// Upgrade a single snapshot file if needed
/// Returns true if the file was upgraded, false if already at latest version
fn upgrade_snapshot_file(path: &Path, dialect: Dialect) -> Result<bool, CliError> {
    let contents = fs::read_to_string(path).map_err(|e| CliError::IoError(e.to_string()))?;

    let json: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| CliError::Other(format!("Invalid JSON in {}: {}", path.display(), e)))?;

    // Get current version
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let latest_version = snapshot_version(dialect);

    if version == latest_version {
        // Already at latest version
        return Ok(false);
    }

    // Check if version is supported for upgrade
    let version_num: u32 = version.parse().unwrap_or(0);
    if !is_supported_version(dialect, version) && version_num > 0 {
        println!(
            "{} Skipping {}: version {} is not supported for upgrade",
            "⚠️".bright_yellow(),
            path.display(),
            version
        );
        return Ok(false);
    }

    println!(
        "{} Upgrading {} from version {} to {}",
        "📦".bright_cyan(),
        path.display(),
        version,
        latest_version
    );

    // Upgrade the snapshot
    let upgraded = upgrade_to_latest(json, dialect);

    // Write back
    let upgraded_json = serde_json::to_string_pretty(&upgraded)
        .map_err(|e| CliError::Other(format!("Failed to serialize upgraded snapshot: {}", e)))?;

    fs::write(path, upgraded_json).map_err(|e| CliError::IoError(e.to_string()))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_v3_snapshots() {
        let temp_dir = TempDir::new().unwrap();

        // Create a V3 migration folder
        let migration_folder = temp_dir.path().join("20231220_initial");
        fs::create_dir_all(&migration_folder).unwrap();
        fs::write(migration_folder.join("snapshot.json"), "{}").unwrap();
        fs::write(migration_folder.join("migration.sql"), "").unwrap();

        let snapshots = find_v3_snapshots(temp_dir.path()).unwrap();
        assert_eq!(snapshots.len(), 1);
    }

    #[test]
    fn test_find_legacy_snapshots() {
        let temp_dir = TempDir::new().unwrap();

        // Create a legacy meta folder
        let meta_folder = temp_dir.path().join("meta");
        fs::create_dir_all(&meta_folder).unwrap();
        fs::write(meta_folder.join("0000_initial_snapshot.json"), "{}").unwrap();
        fs::write(meta_folder.join("0001_add_users_snapshot.json"), "{}").unwrap();
        fs::write(meta_folder.join("_journal.json"), "{}").unwrap(); // Should not be included

        let snapshots = find_legacy_snapshots(&meta_folder).unwrap();
        assert_eq!(snapshots.len(), 2);
    }
}
