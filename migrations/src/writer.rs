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
use crate::words::{PrefixMode, generate_migration_tag};
use drizzle_types::Dialect;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

        // Create migration folder
        let folder_path = self.migration_folder_path(&tag);
        fs::create_dir_all(&folder_path).map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Write SQL file
        let sql_path = self.migration_sql_path(&tag);
        fs::write(&sql_path, &sql).map_err(|e| MigrationError::IoError(e.to_string()))?;

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

        // Write snapshot
        let snapshot_path = self.snapshot_path(&tag);
        snapshot
            .save(&snapshot_path)
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

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

        // Create migration folder
        let folder_path = self.migration_folder_path(&tag);
        fs::create_dir_all(&folder_path).map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Write empty SQL file with placeholder
        let sql_path = self.migration_sql_path(&tag);
        let sql = "-- Custom SQL migration file, put your code below! --\n";
        fs::write(&sql_path, sql).map_err(|e| MigrationError::IoError(e.to_string()))?;

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

        let snapshot_path = self.snapshot_path(&tag);
        snapshot
            .save(&snapshot_path)
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

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
