use crate::migrator::{
    Migration, MigratorError, compute_hash, parse_timestamp_from_tag, split_statements,
};
use std::path::PathBuf;

/// Filesystem migration discovery.
///
/// This is intended for build-time usage (`build.rs`, proc macros) where
/// migrations are discovered once and then embedded.
#[derive(Debug, Clone)]
pub struct MigrationDir {
    path: PathBuf,
}

impl MigrationDir {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Discover all migrations in this directory.
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

        let mut entries: Vec<_> = fs::read_dir(&self.path)
            .map_err(|e| MigratorError::IoError(e.to_string()))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|entry| {
                let tag = entry.file_name().to_string_lossy().to_string();
                let sql_path = entry.path().join("migration.sql");
                if sql_path.exists() {
                    Some((tag, sql_path))
                } else {
                    None
                }
            })
            .collect();

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
