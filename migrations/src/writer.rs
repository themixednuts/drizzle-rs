//! Low-level writer for migration folders, plus [`MigrationError`].
//!
//! Prefer [`build::run`](crate::build::run) in `build.rs`. Use this module
//! for custom generation flows.
//!
//! Folder layout (V3, matches drizzle-kit):
//! - each migration has its own folder `out/{tag}/`
//! - SQL in `out/{tag}/migration.sql`
//! - snapshot in `out/{tag}/snapshot.json`
//! - tags look like `YYYYMMDDHHMMSS_adjective_hero` (or a custom name)
//!
//! There is no journal file: migrations are found by scanning folders.

use crate::naming::{PrefixMode, generate_migration_tag, validate_migration_name};
use crate::sqlite::statements::Generator as SqliteGenerator;
use crate::sqlite::{SQLiteSnapshot, SchemaDiff as SqliteSchemaDiff};
use crate::version::ORIGIN_UUID;
use drizzle_types::Dialect;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Creates `out/{tag}/` atomically, so readers never see a half-written folder.
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

/// Writes migration folders in the V3 layout.
///
/// Diff-based writes ([`write_sqlite_migration`](Self::write_sqlite_migration),
/// [`generate_migration_from_snapshots`](Self::generate_migration_from_snapshots))
/// and [`write_custom_migration`](Self::write_custom_migration) use SQLite
/// snapshots.
///
/// ```text
/// out/
///   20231220143052_initial_schema/
///     migration.sql
///     snapshot.json
///   20231221093015_add_users/
///     migration.sql
///     snapshot.json
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
    /// Creates a writer for the folder `out`, with breakpoints on and
    /// timestamp tag prefixes.
    pub fn new(out: impl Into<PathBuf>, dialect: Dialect) -> Self {
        Self {
            out: out.into(),
            dialect,
            breakpoints: true,
            prefix_mode: PrefixMode::Timestamp, // V3 default
            custom_name: None,
        }
    }

    /// Sets whether `--> statement-breakpoint` lines separate statements.
    #[must_use]
    pub const fn with_breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }

    /// Sets how migration tags are prefixed.
    #[must_use]
    pub const fn with_prefix_mode(mut self, mode: PrefixMode) -> Self {
        self.prefix_mode = mode;
        self
    }

    /// Uses `name` as the tag suffix instead of a random `adjective_hero`.
    #[must_use]
    pub fn with_custom_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    /// Returns the migrations folder.
    #[must_use]
    pub fn migrations_dir(&self) -> &Path {
        &self.out
    }

    /// Returns the dialect.
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Creates the migrations folder if it is missing.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations directory cannot be created (e.g.
    /// insufficient permissions or a conflicting non-directory file exists).
    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.migrations_dir())?;
        Ok(())
    }

    /// Returns `out/{tag}`.
    #[must_use]
    pub fn migration_folder_path(&self, tag: &str) -> PathBuf {
        self.out.join(tag)
    }

    /// Returns `out/{tag}/migration.sql`.
    #[must_use]
    pub fn migration_sql_path(&self, tag: &str) -> PathBuf {
        self.migration_folder_path(tag).join("migration.sql")
    }

    /// Returns `out/{tag}/snapshot.json`.
    #[must_use]
    pub fn snapshot_path(&self, tag: &str) -> PathBuf {
        self.migration_folder_path(tag).join("snapshot.json")
    }

    /// Returns the tags of all folders that contain a `migration.sql`, sorted.
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
                // migration.sql marks a migration folder; custom migrations
                // have no snapshot.json but still occupy an index slot.
                if entry.path().join("migration.sql").exists() {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        folders.sort();
        Ok(folders)
    }

    /// Loads the newest SQLite `snapshot.json`, or an empty snapshot if there
    /// is none.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations directory cannot be read or the
    /// found snapshot cannot be parsed.
    pub fn load_previous_snapshot(&self) -> io::Result<SQLiteSnapshot> {
        let migrations = self.discover_migrations()?;

        // The newest folder with a snapshot is the baseline; snapshot-less
        // custom migrations in between must not reset it to empty.
        for tag in migrations.iter().rev() {
            let snapshot_path = self.snapshot_path(tag);
            if snapshot_path.exists() {
                return SQLiteSnapshot::load(&snapshot_path);
            }
        }

        Ok(SQLiteSnapshot::new())
    }

    /// Writes a SQLite migration folder for `diff` and returns its tag.
    ///
    /// The new snapshot is `current_snapshot` with a fresh ID, chained to the
    /// previous snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::NoChanges`] if the diff produces no
    /// statements, [`MigrationError::ConfigError`] if the tag already exists,
    /// or [`MigrationError::IoError`] / [`MigrationError::SnapshotError`] if
    /// a filesystem operation fails.
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
            _ => crate::naming::generate_migration_tag_with_mode(
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

    /// Diffs two SQLite snapshots and writes the result as a migration
    /// folder, returning its tag.
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

    /// Writes a placeholder migration for hand-written SQL and returns its tag.
    ///
    /// The `migration.sql` holds only a comment; the snapshot is a copy of
    /// the previous one, so the next diff is unaffected.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::ConfigError`] if the tag already exists, or
    /// [`MigrationError::IoError`] / [`MigrationError::SnapshotError`] if a
    /// filesystem operation fails.
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
            _ => crate::naming::generate_migration_tag_with_mode(
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

/// Errors from diffing snapshots and writing migration folders.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// Invalid input: bad or existing tag, unusable rename hint, or a
    /// snapshot that cannot be diffed or rendered.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// A filesystem operation failed.
    #[error("IO error: {0}")]
    IoError(String),

    /// The diff produced no statements.
    #[error("No schema changes detected")]
    NoChanges,

    /// A snapshot could not be read or written.
    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    /// The two snapshots use different dialects.
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
