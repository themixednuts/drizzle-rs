//! Load migrations from a folder on disk with [`MigrationDir`].
//!
//! Used by build scripts, tests, and dev tools. Apps usually embed the same
//! folder at compile time with `drizzle::include_migrations!` instead.

use crate::migrator::{Migration, MigratorError};
use drizzle_types::Dialect;
use std::path::{Path, PathBuf};

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
    dialect: Option<Dialect>,
}

impl MigrationDir {
    /// Creates a handle for the folder at `path`. Nothing is read until
    /// [`discover`](Self::discover).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            dialect: None,
        }
    }

    /// Sets the dialect used to split `migration.sql` files that have no
    /// `--> statement-breakpoint` markers.
    ///
    /// Without it, each folder's `snapshot.json` names the dialect; a folder
    /// with no snapshot is split with dialect-neutral rules (and re-split once
    /// the migrations join a [`Migrations`](crate::Migrations) set).
    #[must_use]
    pub const fn dialect(mut self, dialect: Dialect) -> Self {
        self.dialect = Some(dialect);
        self
    }

    /// Reads every migration in the folder, sorted by tag.
    ///
    /// Files are split into statements as [`Migration::new`] describes:
    /// on `--> statement-breakpoint` markers when present, otherwise on
    /// top-level semicolons with the dialect's rules (see
    /// [`dialect`](Self::dialect)).
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
            entries.push((tag, path));
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut migrations = Vec::with_capacity(entries.len());
        for (tag, path) in entries {
            let sql_content = fs::read_to_string(path.join("migration.sql"))
                .map_err(|e| MigratorError::IoError(e.to_string()))?;
            let dialect = self
                .dialect
                .or_else(|| snapshot_dialect(&path.join("snapshot.json")));
            migrations.push(Migration::from_sql(tag, &sql_content, dialect));
        }

        Ok(migrations)
    }
}

/// Reads the `dialect` field of a `snapshot.json`, if there is a readable
/// one.
fn snapshot_dialect(path: &Path) -> Option<Dialect> {
    let contents = std::fs::read_to_string(path).ok()?;
    let snapshot: serde_json::Value = serde_json::from_str(&contents).ok()?;
    match snapshot.get("dialect")?.as_str()? {
        "sqlite" | "turso" => Some(Dialect::SQLite),
        "postgresql" | "postgres" => Some(Dialect::PostgreSQL),
        "mysql" => Some(Dialect::MySQL),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::MigrationDir;
    use drizzle_types::Dialect;

    fn write_migration(root: &std::path::Path, tag: &str, sql: &str, dialect: Option<&str>) {
        let folder = root.join(tag);
        std::fs::create_dir_all(&folder).expect("migration folder");
        std::fs::write(folder.join("migration.sql"), sql).expect("migration.sql");
        if let Some(dialect) = dialect {
            std::fs::write(
                folder.join("snapshot.json"),
                format!(r#"{{"version":"6","dialect":"{dialect}","id":"x","prevIds":[],"ddl":[]}}"#),
            )
            .expect("snapshot.json");
        }
    }

    const MYSQL_SQL: &str = "INSERT INTO t VALUES ('a\\';b');\n# it's a comment; really\nSELECT 1;";

    #[test]
    fn discover_splits_with_the_snapshot_dialect() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_migration(dir.path(), "20240101000000_mysql", MYSQL_SQL, Some("mysql"));

        let migrations = MigrationDir::new(dir.path()).discover().expect("discover");
        assert_eq!(
            migrations[0].statements(),
            ["INSERT INTO t VALUES ('a\\';b')", "# it's a comment; really\nSELECT 1"]
        );
    }

    #[test]
    fn discover_uses_an_explicit_dialect_for_folders_without_snapshots() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_migration(dir.path(), "20240101000000_custom", MYSQL_SQL, None);

        let migrations = MigrationDir::new(dir.path())
            .dialect(Dialect::MySQL)
            .discover()
            .expect("discover");
        assert_eq!(migrations[0].statements().len(), 2);

        // Without one, joining a set re-splits with the set's dialect.
        let migrations = MigrationDir::new(dir.path()).discover().expect("discover");
        let set = crate::Migrations::new(migrations, Dialect::MySQL);
        assert_eq!(set.all()[0].statements().len(), 2);
    }
}
