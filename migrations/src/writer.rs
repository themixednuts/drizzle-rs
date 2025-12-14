//! Migration file writer

use crate::journal::Journal;
use crate::sqlite::statements::SqliteGenerator;
use crate::sqlite::{SQLiteSnapshot, SchemaDiff as SqliteSchemaDiff};
use crate::version::ORIGIN_UUID;
use crate::words::generate_migration_tag;
use drizzle_types::Dialect;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Migration writer for creating migration files
pub struct MigrationWriter {
    /// Output directory for migrations
    out: PathBuf,
    /// Database dialect
    dialect: Dialect,
    /// Enable SQL statement breakpoints
    breakpoints: bool,
}

impl MigrationWriter {
    /// Create a new migration writer with the given settings
    pub fn new(out: impl Into<PathBuf>, dialect: Dialect) -> Self {
        Self {
            out: out.into(),
            dialect,
            breakpoints: true,
        }
    }

    /// Set whether to use breakpoints in generated SQL
    pub fn with_breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }

    /// Get the migrations directory path
    pub fn migrations_dir(&self) -> &Path {
        &self.out
    }

    /// Get the meta directory path
    pub fn meta_dir(&self) -> PathBuf {
        self.out.join("meta")
    }

    /// Get the journal file path
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Ensure the migration directories exist
    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.migrations_dir())?;
        fs::create_dir_all(self.meta_dir())?;
        Ok(())
    }

    /// Load or create the journal
    pub fn load_journal(&self) -> io::Result<Journal> {
        Journal::load_or_create(&self.journal_path(), self.dialect)
    }

    /// Get the path to a snapshot file
    pub fn snapshot_path(&self, idx: u32) -> PathBuf {
        self.meta_dir().join(format!("{:04}_snapshot.json", idx))
    }

    /// Get the path to a migration SQL file
    pub fn migration_path(&self, tag: &str) -> PathBuf {
        self.out.join(format!("{}.sql", tag))
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
        let generator = SqliteGenerator::new().with_breakpoints(self.breakpoints);
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
        let prev_ids = if journal.entries.is_empty() {
            vec![ORIGIN_UUID.to_string()]
        } else {
            // Load previous snapshot to get its ID
            let prev_snapshot = self
                .load_previous_snapshot()
                .map_err(|e| MigrationError::IoError(e.to_string()))?;
            vec![prev_snapshot.id.clone()]
        };
        snapshot.prev_ids = prev_ids;
        snapshot.id = uuid::Uuid::new_v4().to_string();

        // Write snapshot
        let snapshot_path = self.snapshot_path(idx);
        snapshot
            .save(&snapshot_path)
            .map_err(|e| MigrationError::IoError(e.to_string()))?;

        // Update journal
        journal.add_entry(tag.clone(), self.breakpoints);
        journal
            .save(&self.journal_path())
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
