//! Load migrations from a folder on disk with [`MigrationDir`].
//!
//! Used by build scripts, tests, and dev tools. Apps usually embed the same
//! folder at compile time with `drizzle::include_migrations!` instead.

use crate::migrator::{
    Migration, MigratorError, compute_hash, parse_timestamp_from_tag, split_statements,
};
use std::path::PathBuf;

/// A migrations folder on disk (for example `./drizzle`).
///
/// Each migration is a subfolder named by its tag that holds a
/// `migration.sql` file. Use this in `build.rs`, proc macros, tests, or dev
/// tools; production builds usually embed migrations at compile time instead.
///
/// # Examples
///
/// ```rust,no_run
/// use drizzle_migrations::MigrationDir;
///
/// let migrations = MigrationDir::new("./drizzle").discover()?;
/// for migration in &migrations {
///     println!("{}: {} statements", migration.tag(), migration.statements().len());
/// }
/// # Ok::<(), drizzle_migrations::MigratorError>(())
/// ```
#[derive(Debug, Clone)]
pub struct MigrationDir {
    path: PathBuf,
}

impl MigrationDir {
    /// Creates a handle for the folder at `path`. Nothing is read until
    /// [`discover`](Self::discover).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Reads every migration in the folder, sorted by tag.
    ///
    /// A missing folder yields an empty list.
    /// Subdirectories with neither `migration.sql` nor `snapshot.json`
    /// (editor artifacts, backup folders, staging leftovers) are not
    /// migrations and are skipped, the same way build-time discovery skips
    /// them. A folder with a snapshot but no SQL is a torn migration and
    /// fails closed.
    ///
    /// # Errors
    ///
    /// Returns [`MigratorError::JournalError`] if a legacy `meta/_journal.json`
    /// is found (run `drizzle up` to convert the folder layout),
    /// [`MigratorError::IoError`] if reading the directory fails, or
    /// [`MigratorError::MissingMigration`] if a migration folder has a
    /// `snapshot.json` but lacks its `migration.sql`.
    pub fn discover(&self) -> Result<Vec<Migration>, MigratorError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let journal_path = self.path.join("meta").join("_journal.json");
        if journal_path.exists() {
            return Err(MigratorError::JournalError(
                "We detected old drizzle-kit migration folders. Upgrade them before loading migrations."
                    .to_string(),
            ));
        }

        self.discover_v3()
    }

    fn discover_v3(&self) -> Result<Vec<Migration>, MigratorError> {
        use std::fs;

        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.path).map_err(|e| MigratorError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| MigratorError::IoError(e.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|e| MigratorError::IoError(e.to_string()))?;
            if !file_type.is_dir() {
                continue;
            }

            let tag = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let sql_path = path.join("migration.sql");
            if !sql_path.is_file() {
                // A folder with a snapshot but no SQL is a torn migration —
                // fail closed. Anything else is not a migration folder.
                if path.join("snapshot.json").is_file() {
                    return Err(MigratorError::MissingMigration(tag));
                }
                continue;
            }
            entries.push((tag, sql_path));
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut migrations = Vec::with_capacity(entries.len());
        for (tag, sql_path) in entries {
            let sql_content =
                fs::read_to_string(&sql_path).map_err(|e| MigratorError::IoError(e.to_string()))?;
            let hash = compute_hash(&sql_content);
            let created_at = parse_timestamp_from_tag(&tag);
            let statements = split_statements(&sql_content);

            migrations.push(Migration::with_hash(tag, hash, created_at, statements));
        }

        Ok(migrations)
    }
}
