//! Runtime migration runner for programmatic migrations
//!
//! Provides utilities to:
//! - Load migrations from disk
//! - Track applied migrations
//! - Apply pending migrations in order

use crate::config::Dialect;
use crate::journal::Journal;
use std::fs;
use std::path::{Path, PathBuf};

/// A migration file with its SQL content
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration tag (e.g., "0000_steep_colossus")
    pub tag: String,
    /// Index from the journal
    pub idx: u32,
    /// SQL statements to execute
    pub sql: String,
    /// Whether this migration uses breakpoints
    pub has_breakpoints: bool,
}

impl Migration {
    /// Split the SQL into individual statements using breakpoints
    pub fn statements(&self) -> Vec<&str> {
        if self.has_breakpoints {
            self.sql
                .split("--> statement-breakpoint")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            // Try to split on semicolons, keeping them
            self.sql
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        }
    }
}

/// Runtime migrator for applying migrations programmatically
pub struct Migrator {
    /// Database dialect
    dialect: Dialect,
    /// Migrations directory
    migrations_dir: PathBuf,
    /// Migrations table name
    migrations_table: String,
    /// Loaded journal
    journal: Journal,
    /// Loaded migrations
    migrations: Vec<Migration>,
}

impl Migrator {
    /// Create a new migrator from a migrations directory
    pub fn from_dir(dir: impl Into<PathBuf>, dialect: Dialect) -> Result<Self, MigratorError> {
        Self::from_dir_with_table(dir, dialect, "__drizzle_migrations".to_string())
    }

    /// Create a new migrator with a custom migrations table name
    pub fn from_dir_with_table(
        dir: impl Into<PathBuf>,
        dialect: Dialect,
        migrations_table: String,
    ) -> Result<Self, MigratorError> {
        let migrations_dir = dir.into();
        let journal_path = migrations_dir.join("meta").join("_journal.json");

        let journal = if journal_path.exists() {
            Journal::load(&journal_path).map_err(|e| MigratorError::JournalError(e.to_string()))?
        } else {
            return Err(MigratorError::NoMigrations);
        };

        let mut migrations = Vec::new();

        // Load all migration files - try folder/migration.sql first, fall back to {tag}.sql
        for entry in &journal.entries {
            let folder_path = migrations_dir.join(&entry.tag).join("migration.sql");
            let flat_path = migrations_dir.join(format!("{}.sql", entry.tag));

            let sql_path = if folder_path.exists() {
                folder_path
            } else if flat_path.exists() {
                flat_path
            } else {
                return Err(MigratorError::MissingMigration(entry.tag.clone()));
            };

            let sql =
                fs::read_to_string(&sql_path).map_err(|e| MigratorError::IoError(e.to_string()))?;

            migrations.push(Migration {
                tag: entry.tag.clone(),
                idx: entry.idx,
                sql,
                has_breakpoints: entry.when > 0, // Non-zero timestamp means breakpoints
            });
        }

        Ok(Self {
            dialect,
            migrations_dir,
            migrations_table,
            journal,
            migrations,
        })
    }

    /// Get all migrations
    pub fn all_migrations(&self) -> &[Migration] {
        &self.migrations
    }

    /// Get migrations that haven't been applied yet
    ///
    /// `applied_tags` should contain the tags of migrations already in the database
    pub fn pending_migrations(&self, applied_tags: &[String]) -> Vec<&Migration> {
        self.migrations
            .iter()
            .filter(|m| !applied_tags.contains(&m.tag))
            .collect()
    }

    /// Get the SQL to create the migrations tracking table
    pub fn create_migrations_table_sql(&self) -> String {
        let table_name = &self.migrations_table;

        match self.dialect {
            Dialect::SQLite => format!(
                r#"CREATE TABLE IF NOT EXISTS "{}" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);"#,
                table_name
            ),
            Dialect::PostgreSQL => format!(
                r#"CREATE TABLE IF NOT EXISTS "{}" (
    id SERIAL PRIMARY KEY,
    hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);"#,
                table_name
            ),
            Dialect::MySQL => format!(
                r#"CREATE TABLE IF NOT EXISTS `{}` (
    id INT PRIMARY KEY AUTO_INCREMENT,
    hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);"#,
                table_name
            ),
        }
    }

    /// Get the SQL to record a migration as applied
    pub fn record_migration_sql(&self, tag: &str) -> String {
        let table_name = &self.migrations_table;

        match self.dialect {
            Dialect::SQLite => {
                format!(r#"INSERT INTO "{}" (hash) VALUES ('{}');"#, table_name, tag)
            }
            Dialect::PostgreSQL => {
                format!(r#"INSERT INTO "{}" (hash) VALUES ('{}');"#, table_name, tag)
            }
            Dialect::MySQL => {
                format!(r#"INSERT INTO `{}` (hash) VALUES ('{}');"#, table_name, tag)
            }
        }
    }

    /// Get the SQL to query applied migrations
    pub fn query_applied_sql(&self) -> String {
        let table_name = &self.migrations_table;

        match self.dialect {
            Dialect::SQLite => {
                format!(r#"SELECT hash FROM "{}" ORDER BY id;"#, table_name)
            }
            Dialect::PostgreSQL => {
                format!(r#"SELECT hash FROM "{}" ORDER BY id;"#, table_name)
            }
            Dialect::MySQL => {
                format!(r#"SELECT hash FROM `{}` ORDER BY id;"#, table_name)
            }
        }
    }

    /// Get the dialect
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Get the migrations directory
    pub fn migrations_dir(&self) -> &Path {
        &self.migrations_dir
    }

    /// Get journal
    pub fn journal(&self) -> &Journal {
        &self.journal
    }
}

/// Load migrations from a directory without a config file
pub fn load_migrations_from_dir(dir: &Path) -> Result<Vec<Migration>, MigratorError> {
    let journal_path = dir.join("meta").join("_journal.json");

    if !journal_path.exists() {
        return Err(MigratorError::NoMigrations);
    }

    let journal =
        Journal::load(&journal_path).map_err(|e| MigratorError::JournalError(e.to_string()))?;

    let mut migrations = Vec::new();

    for entry in &journal.entries {
        // Try folder/migration.sql first, fall back to {tag}.sql
        let folder_path = dir.join(&entry.tag).join("migration.sql");
        let flat_path = dir.join(format!("{}.sql", entry.tag));

        let sql_path = if folder_path.exists() {
            folder_path
        } else if flat_path.exists() {
            flat_path
        } else {
            return Err(MigratorError::MissingMigration(entry.tag.clone()));
        };

        let sql =
            fs::read_to_string(&sql_path).map_err(|e| MigratorError::IoError(e.to_string()))?;

        migrations.push(Migration {
            tag: entry.tag.clone(),
            idx: entry.idx,
            sql,
            has_breakpoints: entry.when > 0,
        });
    }

    Ok(migrations)
}

/// Errors that can occur during migration
#[derive(Debug, thiserror::Error)]
pub enum MigratorError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Journal error: {0}")]
    JournalError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("No migrations found")]
    NoMigrations,

    #[error("Missing migration file: {0}")]
    MissingMigration(String),

    #[error("Migration failed: {0}")]
    ExecutionError(String),
}
