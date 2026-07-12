//! Low-level migration file writer for V3 folder layouts.
//!
//! Prefer [`crate::build::run`] for normal `build.rs` workflows. This module is the
//! lower-level writer used for custom generation flows.
//!
//! V3 format (matches drizzle-kit):
//! - Each migration is in its own folder: `out/{tag}/`
//! - SQL file: `out/{tag}/migration.sql`
//! - Snapshot: `out/{tag}/snapshot.json`
//! - Tag format: `YYYYMMDDHHMMSS_adjective_hero` (or custom name)
//!
//! No journal file is used - migrations are discovered by scanning folders.

use crate::sqlite::statements::SqliteGenerator;
use crate::sqlite::{SQLiteSnapshot, SchemaDiff as SqliteSchemaDiff};
use crate::version::ORIGIN_UUID;
use crate::words::{PrefixMode, generate_migration_tag, validate_migration_name};
use drizzle_types::Dialect;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Publish a complete migration directory without exposing partially written files.
///
/// The callback writes into a unique sibling staging directory. The staging
/// directory is renamed to `tag` only after the callback succeeds.
///
/// # Errors
///
/// Returns a configuration error for an invalid or existing tag, and an I/O
/// error when staging, writing, or publishing fails.
#[doc(hidden)]
pub fn publish_migration_directory(
    out: &Path,
    tag: &str,
    write: impl FnOnce(&Path) -> Result<(), MigrationError>,
) -> Result<PathBuf, MigrationError> {
    validate_migration_name(tag).map_err(|error| MigrationError::ConfigError(error.to_string()))?;
    fs::create_dir_all(out).map_err(|error| MigrationError::IoError(error.to_string()))?;

    let destination = out.join(tag);
    if destination.exists() {
        return Err(MigrationError::ConfigError(format!(
            "migration `{tag}` already exists"
        )));
    }

    let staging = out.join(format!(".{tag}.{}.tmp", uuid::Uuid::new_v4()));
    fs::create_dir(&staging).map_err(|error| MigrationError::IoError(error.to_string()))?;

    if let Err(error) = write(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    if destination.exists() {
        let _ = fs::remove_dir_all(&staging);
        return Err(MigrationError::ConfigError(format!(
            "migration `{tag}` already exists"
        )));
    }

    match fs::rename(&staging, &destination) {
        Ok(()) => Ok(destination),
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            Err(MigrationError::IoError(error.to_string()))
        }
    }
}

// =============================================================================
// Migration Writer V3 (folder-based, matches drizzle-kit)
// =============================================================================

/// Low-level writer for creating migration files in V3 folder structure.
///
/// V3 format creates a folder per migration:
/// ```rust
/// # let _ = r####"
/// out/
///   20231220143052_initial_schema/
///     migration.sql
///     snapshot.json
///   20231221093015_add_users/
///     migration.sql
///     snapshot.json
/// # "####;
/// ```
pub struct Writer {
    /// Output directory for migrations
    out: PathBuf,
    /// Database dialect
    dialect: Dialect,
    /// Enable SQL statement breakpoints
    breakpoints: bool,
    /// Prefix mode for migration tags
    prefix_mode: PrefixMode,
    /// Optional custom name for migrations
    custom_name: Option<String>,
}

impl Writer {
    /// Create a new migration writer with the given settings
    pub fn new(out: impl Into<PathBuf>, dialect: Dialect) -> Self {
        Self {
            out: out.into(),
            dialect,
            breakpoints: true,
            prefix_mode: PrefixMode::Timestamp, // V3 default
            custom_name: None,
        }
    }

    /// Set whether to use breakpoints in generated SQL
    #[must_use]
    pub const fn with_breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }

    /// Set the prefix mode for migration tags
    #[must_use]
    pub const fn with_prefix_mode(mut self, mode: PrefixMode) -> Self {
        self.prefix_mode = mode;
        self
    }

    /// Set a custom name for the next migration
    #[must_use]
    pub fn with_custom_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    /// Get the migrations directory path
    #[must_use]
    pub fn migrations_dir(&self) -> &Path {
        &self.out
    }

    /// Get the dialect
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Ensure the migration directory exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations directory cannot be created (e.g.
    /// insufficient permissions or a conflicting non-directory file exists).
    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.migrations_dir())?;
        Ok(())
    }

    /// Get the path to a migration folder
    #[must_use]
    pub fn migration_folder_path(&self, tag: &str) -> PathBuf {
        self.out.join(tag)
    }

    /// Get the path to a migration SQL file (V3 format: folder/migration.sql)
    #[must_use]
    pub fn migration_sql_path(&self, tag: &str) -> PathBuf {
        self.migration_folder_path(tag).join("migration.sql")
    }

    /// Get the path to a snapshot file (V3 format: folder/snapshot.json)
    #[must_use]
    pub fn snapshot_path(&self, tag: &str) -> PathBuf {
        self.migration_folder_path(tag).join("snapshot.json")
    }

    /// Discover all existing migration folders, sorted by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations directory cannot be read.
    pub fn discover_migrations(&self) -> io::Result<Vec<String>> {
        if !self.out.exists() {
            return Ok(Vec::new());
        }

        let mut folders: Vec<String> = fs::read_dir(&self.out)?
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                // Check if it has a snapshot.json (indicates it's a migration folder)
                if entry.path().join("snapshot.json").exists() {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        folders.sort();
        Ok(folders)
    }

    /// Load the previous snapshot by scanning existing migration folders.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations directory cannot be read or the
    /// found snapshot cannot be parsed.
    pub fn load_previous_snapshot(&self) -> io::Result<SQLiteSnapshot> {
        let migrations = self.discover_migrations()?;

        let Some(last_tag) = migrations.last() else {
            return Ok(SQLiteSnapshot::new());
        };
        let snapshot_path = self.snapshot_path(last_tag);

        if snapshot_path.exists() {
            SQLiteSnapshot::load(&snapshot_path)
        } else {
            Ok(SQLiteSnapshot::new())
        }
    }

    /// Write a `SQLite` migration in V3 folder format.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::NoChanges`] if the diff produces no
    /// statements, or [`MigrationError::IoError`] if any filesystem
    /// operation fails during migration emission.
    pub fn write_sqlite_migration(
        &self,
        diff: &SqliteSchemaDiff,
        current_snapshot: &SQLiteSnapshot,
    ) -> Result<String, MigrationError> {
        // Ensure base directory exists
        self.ensure_dirs()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Discover existing migrations for indexing
        let existing = self
            .discover_migrations()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;
        let idx = u32::try_from(existing.len()).unwrap_or(u32::MAX);

        // Generate tag
        let tag = match self.prefix_mode {
            PrefixMode::Timestamp => generate_migration_tag(self.custom_name.as_deref()),
            _ => crate::words::generate_migration_tag_with_mode(
                self.prefix_mode,
                idx,
                self.custom_name.as_deref(),
            ),
        };

        // Generate SQL
        let generator = SqliteGenerator::new().with_breakpoints(self.breakpoints);
        let statements = generator.generate_migration(diff);

        if statements.is_empty() {
            return Err(MigrationError::NoChanges);
        }

        let sql = generator.statements_to_sql(&statements);

        // Create snapshot with proper chain
        let mut snapshot = current_snapshot.clone();
        let prev_ids = if existing.is_empty() {
            vec![ORIGIN_UUID.to_string()]
        } else {
            // Load previous snapshot to get its ID
            let prev_snapshot = self
                .load_previous_snapshot()
                .map_err(|e| MigrationError::IoError(e.to_string()))?;
            vec![prev_snapshot.id]
        };
        snapshot.prev_ids = prev_ids;
        snapshot.id = uuid::Uuid::new_v4().to_string();

        publish_migration_directory(&self.out, &tag, |folder| {
            fs::write(folder.join("migration.sql"), &sql)
                .map_err(|error| MigrationError::IoError(error.to_string()))?;
            snapshot
                .save(&folder.join("snapshot.json"))
                .map_err(|error| MigrationError::SnapshotError(error.to_string()))
        })?;

        Ok(tag)
    }

    /// Generate migration from comparing two snapshots.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::NoChanges`] if the snapshots diff is empty,
    /// or any error produced by [`Self::write_sqlite_migration`].
    pub fn generate_migration_from_snapshots(
        &self,
        prev: &SQLiteSnapshot,
        cur: &SQLiteSnapshot,
    ) -> Result<String, MigrationError> {
        let diff = crate::sqlite::diff_snapshots(prev, cur);

        if diff.is_empty() {
            return Err(MigrationError::NoChanges);
        }

        self.write_sqlite_migration(&diff, cur)
    }

    /// Write a custom (empty) migration for user SQL.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::IoError`] if directory creation or file
    /// writes fail while emitting the placeholder migration folder.
    pub fn write_custom_migration(&self) -> Result<String, MigrationError> {
        // Ensure base directory exists
        self.ensure_dirs()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Discover existing migrations for indexing
        let existing = self
            .discover_migrations()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;
        let idx = u32::try_from(existing.len()).unwrap_or(u32::MAX);

        // Generate tag
        let tag = match self.prefix_mode {
            PrefixMode::Timestamp => generate_migration_tag(self.custom_name.as_deref()),
            _ => crate::words::generate_migration_tag_with_mode(
                self.prefix_mode,
                idx,
                self.custom_name.as_deref(),
            ),
        };

        // Create a minimal snapshot
        let prev_snapshot = self
            .load_previous_snapshot()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        let mut snapshot = prev_snapshot.clone();
        snapshot.prev_ids = if existing.is_empty() {
            vec![ORIGIN_UUID.to_string()]
        } else {
            vec![prev_snapshot.id]
        };
        snapshot.id = uuid::Uuid::new_v4().to_string();

        publish_migration_directory(&self.out, &tag, |folder| {
            let sql = "-- Custom SQL migration file, put your code below! --\n";
            fs::write(folder.join("migration.sql"), sql)
                .map_err(|error| MigrationError::IoError(error.to_string()))?;
            snapshot
                .save(&folder.join("snapshot.json"))
                .map_err(|error| MigrationError::SnapshotError(error.to_string()))
        })?;

        Ok(tag)
    }
}

// =============================================================================
// Migration Errors
// =============================================================================

/// Migration errors
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("No schema changes detected")]
    NoChanges,

    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    #[error("Dialect mismatch: cannot diff snapshots from different dialects")]
    DialectMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_directory_is_complete_and_refuses_collisions() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let destination = publish_migration_directory(temp.path(), "0001_initial", |folder| {
            fs::write(folder.join("migration.sql"), "SELECT 1;")
                .map_err(|error| MigrationError::IoError(error.to_string()))?;
            fs::write(folder.join("snapshot.json"), "{}")
                .map_err(|error| MigrationError::IoError(error.to_string()))
        })
        .expect("publish migration");

        assert!(destination.join("migration.sql").is_file());
        assert!(destination.join("snapshot.json").is_file());

        let error = publish_migration_directory(temp.path(), "0001_initial", |_| Ok(()))
            .expect_err("collision must fail");
        assert!(matches!(error, MigrationError::ConfigError(_)));
        assert_eq!(
            fs::read_to_string(destination.join("migration.sql")).expect("read original"),
            "SELECT 1;"
        );
    }

    #[test]
    fn publish_directory_cleans_staging_after_write_failure() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let error = publish_migration_directory(temp.path(), "0002_broken", |folder| {
            fs::write(folder.join("migration.sql"), "SELECT 1;")
                .map_err(|error| MigrationError::IoError(error.to_string()))?;
            Err(MigrationError::SnapshotError("injected failure".into()))
        })
        .expect_err("write failure must propagate");

        assert!(matches!(error, MigrationError::SnapshotError(_)));
        assert!(!temp.path().join("0002_broken").exists());
        assert_eq!(fs::read_dir(temp.path()).expect("read output").count(), 0);
    }

    #[test]
    fn publish_directory_rejects_unsafe_tag_before_writing() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let mut called = false;
        let error = publish_migration_directory(temp.path(), "../escape", |_| {
            called = true;
            Ok(())
        })
        .expect_err("unsafe tag must fail");

        assert!(!called);
        assert!(matches!(error, MigrationError::ConfigError(_)));
        assert!(
            !temp
                .path()
                .parent()
                .expect("parent")
                .join("escape")
                .exists()
        );
    }
}
