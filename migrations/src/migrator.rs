//! Runtime migration runner for programmatic migrations
//!
//! Provides utilities to:
//! - Load migrations from disk
//! - Track applied migrations
//! - Apply pending migrations in order

use crate::config::DrizzleConfig;
use crate::journal::Journal;
use std::fs;
use std::path::Path;

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
    /// Configuration
    config: DrizzleConfig,
    /// Loaded journal
    journal: Journal,
    /// Loaded migrations
    migrations: Vec<Migration>,
}

impl Migrator {
    /// Create a new migrator from a drizzle.toml config file
    pub fn from_config_file(path: &Path) -> Result<Self, MigratorError> {
        let config = DrizzleConfig::from_file(path)
            .map_err(|e| MigratorError::ConfigError(e.to_string()))?;
        Self::new(config)
    }

    /// Create a new migrator with the given configuration
    pub fn new(config: DrizzleConfig) -> Result<Self, MigratorError> {
        let journal_path = config.journal_path();

        let journal = if journal_path.exists() {
            Journal::load(&journal_path).map_err(|e| MigratorError::JournalError(e.to_string()))?
        } else {
            return Err(MigratorError::NoMigrations);
        };

        let mut migrations = Vec::new();

        // Load all migration files
        for entry in &journal.entries {
            let sql_path = config.migrations_dir().join(format!("{}.sql", entry.tag));

            if !sql_path.exists() {
                return Err(MigratorError::MissingMigration(entry.tag.clone()));
            }

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
            config,
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
        let table_name = self.config.migrations_table();

        match self.config.dialect {
            crate::config::Dialect::Sqlite => format!(
                r#"CREATE TABLE IF NOT EXISTS "{}" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);"#,
                table_name
            ),
            crate::config::Dialect::Postgresql => format!(
                r#"CREATE TABLE IF NOT EXISTS "{}" (
    id SERIAL PRIMARY KEY,
    hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);"#,
                table_name
            ),
            crate::config::Dialect::Mysql => format!(
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
        let table_name = self.config.migrations_table();

        match self.config.dialect {
            crate::config::Dialect::Sqlite => {
                format!(r#"INSERT INTO "{}" (hash) VALUES ('{}');"#, table_name, tag)
            }
            crate::config::Dialect::Postgresql => {
                format!(r#"INSERT INTO "{}" (hash) VALUES ('{}');"#, table_name, tag)
            }
            crate::config::Dialect::Mysql => {
                format!(r#"INSERT INTO `{}` (hash) VALUES ('{}');"#, table_name, tag)
            }
        }
    }

    /// Get the SQL to query applied migrations
    pub fn query_applied_sql(&self) -> String {
        let table_name = self.config.migrations_table();

        match self.config.dialect {
            crate::config::Dialect::Sqlite => {
                format!(r#"SELECT hash FROM "{}" ORDER BY id;"#, table_name)
            }
            crate::config::Dialect::Postgresql => {
                format!(r#"SELECT hash FROM "{}" ORDER BY id;"#, table_name)
            }
            crate::config::Dialect::Mysql => {
                format!(r#"SELECT hash FROM `{}` ORDER BY id;"#, table_name)
            }
        }
    }

    /// Get configuration
    pub fn config(&self) -> &DrizzleConfig {
        &self.config
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
        let sql_path = dir.join(format!("{}.sql", entry.tag));

        if !sql_path.exists() {
            return Err(MigratorError::MissingMigration(entry.tag.clone()));
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_statements_with_breakpoints() {
        let migration = Migration {
            tag: "0000_test".to_string(),
            idx: 0,
            sql: "CREATE TABLE a;\n--> statement-breakpoint\nCREATE TABLE b;".to_string(),
            has_breakpoints: true,
        };

        let stmts = migration.statements();
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "CREATE TABLE a;");
        assert_eq!(stmts[1], "CREATE TABLE b;");
    }

    #[test]
    fn test_migration_statements_without_breakpoints() {
        let migration = Migration {
            tag: "0000_test".to_string(),
            idx: 0,
            sql: "CREATE TABLE a; CREATE TABLE b;".to_string(),
            has_breakpoints: false,
        };

        let stmts = migration.statements();
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "CREATE TABLE a");
        assert_eq!(stmts[1], "CREATE TABLE b");
    }
}
