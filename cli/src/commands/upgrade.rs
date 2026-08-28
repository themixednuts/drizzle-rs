//! Upgrade command - upgrades migration snapshots to the latest version
//!
//! This command matches drizzle-kit's `up` command and handles two layouts:
//!
//! * **Legacy stable-kit layout** (flat `NNNN_name.sql` files plus
//!   `meta/_journal.json` and `meta/NNNN_snapshot.json`): the whole
//!   directory is converted to the v1-beta layout (`{tag}/migration.sql` +
//!   `{tag}/snapshot.json`) with every snapshot structurally upgraded to the
//!   current entity-array format. SQL files are moved verbatim.
//! * **v1-beta layout** with old snapshot versions: each `snapshot.json` is
//!   upgraded in place.

use crate::config::{Config, Dialect as CliDialect};
use crate::error::CliError;
use crate::output;
use drizzle_migrations::upgrade::upgrade_to_latest;
use drizzle_migrations::version::{is_supported_version, snapshot_version};
use drizzle_types::Dialect;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(clap::Args, Debug, Clone, Default)]
pub struct UpgradeOptions {
    /// Override dialect from config
    #[arg(long)]
    pub dialect: Option<CliDialect>,

    /// Override output directory
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// Run the upgrade command.
///
/// # Errors
///
/// Returns [`CliError`] if the database cannot be resolved, the migration
/// directory is unreadable, the legacy journal cannot be parsed, a legacy
/// snapshot fails to convert to the current format, or writing the upgraded
/// files to disk fails.
pub fn run(config: &Config, db_name: Option<&str>, opts: &UpgradeOptions) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    // CLI flags override config
    let dialect = opts.dialect.unwrap_or(db.dialect).to_base();
    let out_dir = opts.out.as_deref().unwrap_or_else(|| db.migrations_dir());

    println!(
        "{}",
        output::heading(&format!(
            "Checking for snapshots to upgrade in {}",
            out_dir.display()
        ))
    );

    if !out_dir.exists() {
        println!(
            "{}",
            output::warning(&format!(
                "No migrations folder found at {}",
                out_dir.display()
            ))
        );
        return Ok(());
    }

    // Legacy stable-kit layout: flat SQL files with a meta/ journal. Convert
    // the whole directory to the v1-beta folder layout.
    if out_dir.join("meta").join("_journal.json").exists() {
        let converted = convert_legacy_layout(out_dir, dialect)?;
        println!(
            "{}",
            output::success(&format!(
                "Converted {} migration(s) to the folder layout (snapshots at version {})",
                converted,
                snapshot_version(dialect)
            ))
        );
        return Ok(());
    }

    let upgraded = upgrade_snapshots(out_dir, dialect)?;

    if upgraded == 0 {
        println!(
            "{}",
            output::success(&format!(
                "All snapshots are already at the latest version ({})",
                snapshot_version(dialect)
            ))
        );
    } else {
        println!(
            "{}",
            output::success(&format!(
                "Upgraded {} snapshot(s) to version {}",
                upgraded,
                snapshot_version(dialect)
            ))
        );
    }

    Ok(())
}

// =============================================================================
// Legacy layout conversion (flat NNNN_name.sql + meta/)
// =============================================================================

/// A fully converted legacy journal entry, held in memory until every entry
/// has converted cleanly.
struct ConvertedEntry {
    tag: String,
    sql_path: PathBuf,
    sql: String,
    snapshot_json: String,
}

/// Convert a legacy drizzle-kit migrations directory to the v1-beta layout.
///
/// For each journal entry, the flat `{tag}.sql` file becomes
/// `{tag}/migration.sql` (content moved verbatim) and the matching
/// `meta/{idx}_snapshot.json` becomes `{tag}/snapshot.json`, structurally
/// upgraded to the current entity-array format.
///
/// The conversion is all-or-nothing: every entry is read, upgraded, and
/// validated in memory first; the new folders are then written; the legacy
/// flat files and the `meta/` directory are removed only after every write
/// succeeded. Any failure before that point leaves the directory untouched
/// (or with extra folders next to the intact legacy files, which a rerun
/// simply overwrites).
fn convert_legacy_layout(out_dir: &Path, dialect: Dialect) -> Result<usize, CliError> {
    let meta_dir = out_dir.join("meta");
    let journal_path = meta_dir.join("_journal.json");

    println!(
        "{}",
        output::info(
            "Detected legacy drizzle-kit migration folders (meta/_journal.json) — converting to the folder layout"
        )
    );

    let journal_contents =
        fs::read_to_string(&journal_path).map_err(|e| CliError::IoError(e.to_string()))?;
    let journal: serde_json::Value = serde_json::from_str(&journal_contents).map_err(|e| {
        CliError::Other(format!("Invalid JSON in {}: {}", journal_path.display(), e))
    })?;

    let entries = journal
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            CliError::Other(format!("{} has no 'entries' array", journal_path.display()))
        })?;

    // Phase 1: read + upgrade + validate everything in memory. Nothing on
    // disk is touched until every entry converts cleanly.
    let mut converted: Vec<ConvertedEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        let tag = entry
            .get("tag")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CliError::Other(format!(
                    "Journal entry without a 'tag' in {}",
                    journal_path.display()
                ))
            })?;
        let idx = entry
            .get("idx")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                CliError::Other(format!(
                    "Journal entry '{tag}' without an 'idx' in {}",
                    journal_path.display()
                ))
            })?;

        let sql_path = out_dir.join(format!("{tag}.sql"));
        let sql = fs::read_to_string(&sql_path).map_err(|e| {
            CliError::Other(format!(
                "Cannot read migration SQL {}: {}",
                sql_path.display(),
                e
            ))
        })?;

        let snapshot_path = meta_dir.join(format!("{idx:04}_snapshot.json"));
        let snapshot_contents = fs::read_to_string(&snapshot_path).map_err(|e| {
            CliError::Other(format!(
                "Cannot read snapshot {}: {}",
                snapshot_path.display(),
                e
            ))
        })?;
        let snapshot: serde_json::Value =
            serde_json::from_str(&snapshot_contents).map_err(|e| {
                CliError::Other(format!(
                    "Invalid JSON in {}: {}",
                    snapshot_path.display(),
                    e
                ))
            })?;

        let upgraded = upgrade_to_latest(snapshot, dialect);
        let snapshot_json = validated_pretty_snapshot(&upgraded, dialect).map_err(|e| {
            CliError::Other(format!(
                "Snapshot {} did not convert cleanly to the current format: {}",
                snapshot_path.display(),
                e
            ))
        })?;

        converted.push(ConvertedEntry {
            tag: tag.to_string(),
            sql_path,
            sql,
            snapshot_json,
        });
    }

    // Phase 2: write the new folders; the legacy files stay untouched until
    // every write has succeeded.
    for entry in &converted {
        let folder = out_dir.join(&entry.tag);
        fs::create_dir_all(&folder).map_err(|e| CliError::IoError(e.to_string()))?;
        fs::write(folder.join("migration.sql"), &entry.sql)
            .map_err(|e| CliError::IoError(e.to_string()))?;
        fs::write(folder.join("snapshot.json"), &entry.snapshot_json)
            .map_err(|e| CliError::IoError(e.to_string()))?;
        println!(
            "{}",
            output::info(&format!(
                "  {} -> {}/migration.sql + snapshot.json",
                entry.tag, entry.tag
            ))
        );
    }

    // Phase 3: remove the legacy flat files and the meta/ directory.
    for entry in &converted {
        fs::remove_file(&entry.sql_path).map_err(|e| CliError::IoError(e.to_string()))?;
    }
    fs::remove_dir_all(&meta_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    Ok(converted.len())
}

/// Validate that an upgraded snapshot deserializes as the current typed
/// snapshot format, then pretty-print the *upgraded document itself* (it may
/// carry explicit `null`s that keep optional fields loadable — a typed
/// re-serialization would drop them).
fn validated_pretty_snapshot(
    upgraded: &serde_json::Value,
    dialect: Dialect,
) -> Result<String, CliError> {
    match dialect {
        Dialect::SQLite => {
            serde_json::from_value::<drizzle_migrations::sqlite::SQLiteSnapshot>(upgraded.clone())
                .map_err(|e| CliError::Other(e.to_string()))?;
        }
        Dialect::PostgreSQL => {
            serde_json::from_value::<drizzle_migrations::postgres::PostgresSnapshot>(
                upgraded.clone(),
            )
            .map_err(|e| CliError::Other(e.to_string()))?;
        }
        Dialect::MySQL => {
            serde_json::from_value::<drizzle_migrations::mysql::MySQLSnapshot>(upgraded.clone())
                .map_err(|e| CliError::Other(e.to_string()))?;
        }
    }

    serde_json::to_string_pretty(upgraded).map_err(|e| CliError::Other(e.to_string()))
}

// =============================================================================
// In-place upgrades (v1-beta layout with old snapshot versions)
// =============================================================================

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

    // Also check for orphaned meta/ folder snapshots (no _journal.json, so
    // the full layout conversion doesn't apply)
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

        if path.is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.ends_with("_snapshot.json")
        {
            snapshots.push(path);
        }
    }

    Ok(snapshots)
}

/// True if the snapshot document already has the current entity-array
/// structure (a `ddl` array) *and* the latest version stamp. Version alone is
/// not trusted: historic upgrades stamped the latest version onto documents
/// that still had the legacy object shape.
fn is_current_format(json: &serde_json::Value, dialect: Dialect) -> bool {
    let version = json
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    version == snapshot_version(dialect) && json.get("ddl").is_some_and(serde_json::Value::is_array)
}

/// Upgrade a single snapshot file if needed
/// Returns true if the file was upgraded, false if already at latest version
fn upgrade_snapshot_file(path: &Path, dialect: Dialect) -> Result<bool, CliError> {
    let contents = fs::read_to_string(path).map_err(|e| CliError::IoError(e.to_string()))?;

    let json: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| CliError::Other(format!("Invalid JSON in {}: {}", path.display(), e)))?;

    if is_current_format(&json, dialect) {
        return Ok(false);
    }

    // Get current version
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let latest_version = snapshot_version(dialect);

    // Check if version is supported for upgrade
    let version_num: u32 = version.parse().unwrap_or(0);
    if !is_supported_version(dialect, version) && version_num > 0 {
        println!(
            "{}",
            output::warning(&format!(
                "Skipping {}: version {} is not supported for upgrade",
                path.display(),
                version
            ))
        );
        return Ok(false);
    }

    println!(
        "{}",
        output::info(&format!(
            "Upgrading {} from version {} to {}",
            path.display(),
            version,
            latest_version
        ))
    );

    // Upgrade the snapshot and refuse to write anything that doesn't parse
    // as the current typed format.
    let upgraded = upgrade_to_latest(json, dialect);
    let upgraded_json = validated_pretty_snapshot(&upgraded, dialect).map_err(|e| {
        CliError::Other(format!(
            "Snapshot {} did not convert cleanly to the current format: {}",
            path.display(),
            e
        ))
    })?;

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

    #[test]
    fn test_is_current_format_requires_ddl_array() {
        // A mis-stamped legacy doc (latest version string, object shape)
        // must not be treated as current.
        let mis_stamped = serde_json::json!({
            "version": snapshot_version(Dialect::SQLite),
            "dialect": "sqlite",
            "tables": {}
        });
        assert!(!is_current_format(&mis_stamped, Dialect::SQLite));

        let current = serde_json::json!({
            "version": snapshot_version(Dialect::SQLite),
            "dialect": "sqlite",
            "ddl": []
        });
        assert!(is_current_format(&current, Dialect::SQLite));
    }

    #[test]
    fn mysql_latest_snapshot_is_validated_like_other_dialects() {
        let current = serde_json::json!({
            "version": snapshot_version(Dialect::MySQL),
            "dialect": "mysql",
            "id": "00000000-0000-0000-0000-000000000001",
            "prevIds": ["00000000-0000-0000-0000-000000000000"],
            "ddl": []
        });
        let rendered =
            validated_pretty_snapshot(&current, Dialect::MySQL).expect("current MySQL snapshot");
        assert!(rendered.contains("\"dialect\": \"mysql\""));
    }
}
