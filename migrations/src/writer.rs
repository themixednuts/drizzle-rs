//! Migration file writer

use crate::config::DrizzleConfig;
use crate::journal::Journal;
use crate::sqlgen::sqlite::SqliteGenerator;
use crate::sqlite::{SQLiteSnapshot, SchemaDiff as SqliteSchemaDiff};
use crate::words::generate_migration_tag;

use std::fs;
use std::io;
use std::path::Path;

/// Migration writer for creating migration files
pub struct MigrationWriter {
    config: DrizzleConfig,
}

impl MigrationWriter {
    /// Create a new migration writer with the given configuration
    pub fn new(config: DrizzleConfig) -> Self {
        Self { config }
    }

    /// Create from a config file path
    pub fn from_config_file(path: &Path) -> Result<Self, MigrationError> {
        let config = DrizzleConfig::from_file(path)
            .map_err(|e| MigrationError::ConfigError(e.to_string()))?;
        Ok(Self::new(config))
    }

    /// Ensure the migration directories exist
    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.config.migrations_dir())?;
        fs::create_dir_all(self.config.meta_dir())?;
        Ok(())
    }

    /// Load or create the journal
    pub fn load_journal(&self) -> io::Result<Journal> {
        let dialect = match self.config.dialect {
            crate::config::Dialect::Sqlite => "sqlite",
            crate::config::Dialect::Postgresql => "postgresql",
            crate::config::Dialect::Mysql => "mysql",
        };
        Journal::load_or_create(&self.config.journal_path(), dialect)
    }

    /// Get the path to a snapshot file
    pub fn snapshot_path(&self, idx: u32) -> std::path::PathBuf {
        self.config
            .meta_dir()
            .join(format!("{:04}_snapshot.json", idx))
    }

    /// Get the path to a migration SQL file
    pub fn migration_path(&self, tag: &str) -> std::path::PathBuf {
        self.config.migrations_dir().join(format!("{}.sql", tag))
    }

    /// Load the previous snapshot, or return an empty one if none exists
    pub fn load_previous_snapshot(&self) -> io::Result<SQLiteSnapshot> {
        let journal = self.load_journal()?;

        if journal.entries.is_empty() {
            return Ok(SQLiteSnapshot::new());
        }

        let last_idx = journal.entries.last().unwrap().idx;
        let snapshot_path = self.snapshot_path(last_idx);

        if snapshot_path.exists() {
            SQLiteSnapshot::load(&snapshot_path)
        } else {
            Ok(SQLiteSnapshot::new())
        }
    }

    /// Write a SQLite migration
    pub fn write_sqlite_migration(
        &self,
        diff: &SqliteSchemaDiff,
        current_snapshot: &SQLiteSnapshot,
    ) -> Result<String, MigrationError> {
        // Ensure directories exist
        self.ensure_dirs()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Load journal
        let mut journal = self
            .load_journal()
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Generate tag
        let idx = journal.next_idx();
        let tag = generate_migration_tag(idx);

        // Generate SQL
        let generator = SqliteGenerator::new().with_breakpoints(self.config.breakpoints);
        let statements = generator.generate_migration(diff);

        if statements.is_empty() {
            return Err(MigrationError::NoChanges);
        }

        let sql = generator.statements_to_sql(&statements);

        // Write SQL file
        let sql_path = self.migration_path(&tag);
        fs::write(&sql_path, &sql).map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Create snapshot with proper chain
        let mut snapshot = current_snapshot.clone();
        let prev_id = if journal.entries.is_empty() {
            SQLiteSnapshot::ORIGIN_UUID.to_string()
        } else {
            // Load previous snapshot to get its ID
            let prev_snapshot = self
                .load_previous_snapshot()
                .map_err(|e| MigrationError::IoError(e.to_string()))?;
            prev_snapshot.id.clone()
        };
        snapshot.prev_id = prev_id;
        snapshot.id = uuid::Uuid::new_v4().to_string();

        // Write snapshot
        let snapshot_path = self.snapshot_path(idx);
        snapshot
            .save(&snapshot_path)
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Update journal
        journal.add_entry(tag.clone(), self.config.breakpoints);
        journal
            .save(&self.config.journal_path())
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        Ok(tag)
    }

    /// Generate migration from comparing two snapshots
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
}

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_config() -> DrizzleConfig {
        DrizzleConfig {
            out: PathBuf::from("./test_drizzle_output"),
            ..Default::default()
        }
    }

    #[test]
    fn test_migration_writer_creation() {
        let config = temp_config();
        let writer = MigrationWriter::new(config.clone());
        assert_eq!(writer.config.out, config.out);
    }

    #[test]
    fn test_snapshot_path() {
        let config = temp_config();
        let writer = MigrationWriter::new(config);

        let path = writer.snapshot_path(0);
        assert!(path.to_string_lossy().contains("0000_snapshot.json"));

        let path = writer.snapshot_path(42);
        assert!(path.to_string_lossy().contains("0042_snapshot.json"));
    }
}
