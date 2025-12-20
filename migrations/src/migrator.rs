//! Runtime migration runner for programmatic migrations
//!
//! Provides utilities to:
//! - Load migrations from various sources (embedded, filesystem, remote)
//! - Track applied migrations in the database
//! - Apply pending migrations in order
//!
//! # Usage
//!
//! ## Embedded Migrations (recommended for production/serverless)
//!
//! Use `include_str!` to embed migration SQL files at compile time:
//!
//! ```ignore
//! use drizzle_migrations::{Migration, MigrationSet};
//! use drizzle_types::Dialect;
//!
//! const MIGRATIONS: &[Migration] = &[
//!     Migration::new("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
//!     Migration::new("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
//! ];
//!
//! async fn run_migrations(db: &Database) -> Result<(), MigratorError> {
//!     let set = MigrationSet::new(MIGRATIONS.to_vec(), Dialect::SQLite);
//!     
//!     // Ensure migrations table exists
//!     db.execute(&set.create_table_sql()).await?;
//!     
//!     // Get applied migrations
//!     let applied = db.query_column::<String>(&set.query_applied_sql()).await?;
//!     
//!     // Apply pending migrations
//!     for migration in set.pending(&applied) {
//!         for statement in migration.statements() {
//!             db.execute(statement).await?;
//!         }
//!         db.execute(&set.record_migration_sql(migration.hash())).await?;
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Loading from Filesystem (for development)
//!
//! ```ignore
//! use drizzle_migrations::MigrationSet;
//!
//! let set = MigrationSet::from_dir("./drizzle", Dialect::SQLite)?;
//! ```

use drizzle_types::Dialect;
use std::path::Path;

/// A migration with its SQL content
///
/// Represents a single migration that can be applied to the database.
/// The `hash` field is used to track which migrations have been applied.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Unique hash identifying this migration (typically SHA-256 of the SQL)
    hash: String,
    /// Timestamp or folder millis for ordering
    created_at: i64,
    /// SQL statements to execute (pre-split if breakpoints were used)
    sql: Vec<String>,
}

impl Migration {
    /// Create a new migration from embedded SQL
    ///
    /// The hash is computed from the SQL content.
    /// SQL is split on `"--> statement-breakpoint"` markers.
    pub fn new(tag: &str, sql: &str) -> Self {
        let hash = compute_hash(sql);
        let created_at = parse_timestamp_from_tag(tag);
        let statements = split_statements(sql);

        Self {
            hash,
            created_at,
            sql: statements,
        }
    }

    /// Create a migration with explicit hash and timestamp
    pub fn with_hash(hash: impl Into<String>, created_at: i64, sql: Vec<String>) -> Self {
        Self {
            hash: hash.into(),
            created_at,
            sql,
        }
    }

    /// Get the migration hash (used for tracking)
    #[inline]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Get the creation timestamp
    #[inline]
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Get the SQL statements (already split)
    #[inline]
    pub fn statements(&self) -> &[String] {
        &self.sql
    }

    /// Check if this migration is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sql.is_empty() || self.sql.iter().all(|s| s.trim().is_empty())
    }
}

/// A collection of migrations ready to be applied
#[derive(Debug, Clone)]
pub struct MigrationSet {
    /// Ordered list of migrations
    migrations: Vec<Migration>,
    /// Database dialect
    dialect: Dialect,
    /// Migrations table name
    table: String,
    /// Migrations schema (PostgreSQL only)
    schema: Option<String>,
}

impl MigrationSet {
    /// Create a new migration set from migrations
    pub fn new(migrations: Vec<Migration>, dialect: Dialect) -> Self {
        Self {
            migrations,
            dialect,
            table: "__drizzle_migrations".to_string(),
            schema: match dialect {
                Dialect::PostgreSQL => Some("drizzle".to_string()),
                _ => None,
            },
        }
    }

    /// Create an empty migration set
    pub fn empty(dialect: Dialect) -> Self {
        Self::new(Vec::new(), dialect)
    }

    /// Set a custom migrations table name
    pub fn with_table(mut self, table: impl Into<String>) -> Self {
        self.table = table.into();
        self
    }

    /// Set a custom migrations schema (PostgreSQL only)
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Load migrations from a filesystem directory
    ///
    /// Reads the journal and migration files from disk.
    /// Uses the new folder structure: `{migrations_dir}/{tag}/migration.sql`
    pub fn from_dir(dir: impl AsRef<Path>, dialect: Dialect) -> Result<Self, MigratorError> {
        use crate::journal::Journal;
        use std::fs;

        let dir = dir.as_ref();
        let journal_path = dir.join("meta").join("_journal.json");

        // No journal = no migrations
        if !journal_path.exists() {
            return Ok(Self::empty(dialect));
        }

        let journal =
            Journal::load(&journal_path).map_err(|e| MigratorError::JournalError(e.to_string()))?;

        let mut migrations = Vec::with_capacity(journal.entries.len());

        for entry in &journal.entries {
            // Try folder/migration.sql first (new format)
            let folder_path = dir.join(&entry.tag).join("migration.sql");
            // Fall back to {tag}.sql (old format)
            let flat_path = dir.join(format!("{}.sql", entry.tag));

            let sql_path = if folder_path.exists() {
                folder_path
            } else if flat_path.exists() {
                flat_path
            } else {
                return Err(MigratorError::MissingMigration(entry.tag.clone()));
            };

            let sql_content =
                fs::read_to_string(&sql_path).map_err(|e| MigratorError::IoError(e.to_string()))?;

            let hash = compute_hash(&sql_content);
            let statements = split_statements(&sql_content);

            migrations.push(Migration {
                hash,
                created_at: entry.when as i64,
                sql: statements,
            });
        }

        Ok(Self::new(migrations, dialect))
    }

    /// Get all migrations
    #[inline]
    pub fn all(&self) -> &[Migration] {
        &self.migrations
    }

    /// Get migrations that haven't been applied yet
    ///
    /// `applied_hashes` should contain the hashes of migrations in the database.
    pub fn pending<'a>(&'a self, applied_hashes: &[String]) -> impl Iterator<Item = &'a Migration> {
        self.migrations
            .iter()
            .filter(move |m| !applied_hashes.contains(&m.hash))
    }

    /// Check if there are any pending migrations
    pub fn has_pending(&self, applied_hashes: &[String]) -> bool {
        self.migrations
            .iter()
            .any(|m| !applied_hashes.contains(&m.hash))
    }

    /// Get the dialect
    #[inline]
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Get the full table identifier (with schema for PostgreSQL)
    fn table_ident(&self) -> String {
        match (&self.dialect, &self.schema) {
            (Dialect::PostgreSQL, Some(schema)) => format!("\"{}\".\"{}\"", schema, self.table),
            (Dialect::MySQL, _) => format!("`{}`", self.table),
            _ => format!("\"{}\"", self.table),
        }
    }

    /// Get the SQL to create the migrations schema (PostgreSQL only)
    pub fn create_schema_sql(&self) -> Option<String> {
        self.schema
            .as_ref()
            .map(|schema| format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema))
    }

    /// Get the SQL to create the migrations tracking table
    ///
    /// Table schema matches drizzle-orm:
    /// - SQLite: id (INTEGER PK), hash (TEXT), created_at (numeric/INTEGER)
    /// - PostgreSQL: id (SERIAL PK), hash (TEXT), created_at (BIGINT)
    /// - MySQL: id (INT PK AUTO_INCREMENT), hash (VARCHAR(255)), created_at (BIGINT)
    pub fn create_table_sql(&self) -> String {
        let table = self.table_ident();

        match self.dialect {
            Dialect::SQLite => format!(
                r#"CREATE TABLE IF NOT EXISTS {} (
    id INTEGER PRIMARY KEY,
    hash TEXT NOT NULL,
    created_at INTEGER
);"#,
                table
            ),
            Dialect::PostgreSQL => format!(
                r#"CREATE TABLE IF NOT EXISTS {} (
    id SERIAL PRIMARY KEY,
    hash TEXT NOT NULL,
    created_at BIGINT
);"#,
                table
            ),
            Dialect::MySQL => format!(
                r#"CREATE TABLE IF NOT EXISTS {} (
    id INT PRIMARY KEY AUTO_INCREMENT,
    hash VARCHAR(255) NOT NULL,
    created_at BIGINT
);"#,
                table
            ),
        }
    }

    /// Get the SQL to record a migration as applied
    pub fn record_migration_sql(&self, hash: &str, created_at: i64) -> String {
        let table = self.table_ident();

        match self.dialect {
            Dialect::SQLite | Dialect::PostgreSQL => {
                format!(
                    r#"INSERT INTO {} ("hash", "created_at") VALUES ('{}', {});"#,
                    table, hash, created_at
                )
            }
            Dialect::MySQL => {
                format!(
                    r#"INSERT INTO {} (`hash`, `created_at`) VALUES ('{}', {});"#,
                    table, hash, created_at
                )
            }
        }
    }

    /// Get the SQL to query applied migrations (ordered by created_at DESC, limit 1)
    ///
    /// Returns: id, hash, created_at
    pub fn query_last_applied_sql(&self) -> String {
        let table = self.table_ident();

        match self.dialect {
            Dialect::SQLite | Dialect::PostgreSQL => {
                format!(
                    r#"SELECT id, hash, created_at FROM {} ORDER BY created_at DESC LIMIT 1;"#,
                    table
                )
            }
            Dialect::MySQL => {
                format!(
                    r#"SELECT id, hash, created_at FROM {} ORDER BY created_at DESC LIMIT 1;"#,
                    table
                )
            }
        }
    }

    /// Get the SQL to query all applied migration hashes
    pub fn query_all_hashes_sql(&self) -> String {
        let table = self.table_ident();
        format!(r#"SELECT hash FROM {} ORDER BY id;"#, table)
    }

    /// Get the SQL to check if migrations table exists
    pub fn table_exists_sql(&self) -> String {
        match self.dialect {
            Dialect::SQLite => format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}';",
                self.table
            ),
            Dialect::PostgreSQL => {
                if let Some(ref schema) = self.schema {
                    format!(
                        "SELECT table_name FROM information_schema.tables WHERE table_schema='{}' AND table_name='{}';",
                        schema, self.table
                    )
                } else {
                    format!(
                        "SELECT table_name FROM information_schema.tables WHERE table_name='{}';",
                        self.table
                    )
                }
            }
            Dialect::MySQL => format!(
                "SELECT table_name FROM information_schema.tables WHERE table_name='{}';",
                self.table
            ),
        }
    }
}

/// Errors that can occur during migration
#[derive(Debug, thiserror::Error)]
pub enum MigratorError {
    #[error("Journal error: {0}")]
    JournalError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Missing migration file: {0}")]
    MissingMigration(String),

    #[error("Migration failed: {0}")]
    ExecutionError(String),
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute SHA-256 hash of the SQL content
fn compute_hash(sql: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Use a simple hash for now - in production you might want SHA-256
    let mut hasher = DefaultHasher::new();
    sql.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Split SQL content into individual statements
fn split_statements(sql: &str) -> Vec<String> {
    if sql.contains("--> statement-breakpoint") {
        // Use explicit breakpoint markers
        sql.split("--> statement-breakpoint")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        // Fall back to splitting on semicolons
        // This is a simple approach - a full SQL parser would handle edge cases
        // like semicolons in strings, but this works for typical DDL statements
        split_on_semicolons(sql)
    }
}

/// Split SQL on semicolons, handling basic cases
fn split_on_semicolons(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = ' ';

    for ch in sql.chars() {
        match ch {
            '\'' | '"' if !in_string => {
                in_string = true;
                string_char = ch;
                current.push(ch);
            }
            c if in_string && c == string_char => {
                in_string = false;
                current.push(ch);
            }
            ';' if !in_string => {
                let stmt = current.trim().to_string();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // Don't forget the last statement (might not end with ;)
    let stmt = current.trim().to_string();
    if !stmt.is_empty() {
        statements.push(stmt);
    }

    statements
}

/// Parse timestamp from migration tag (format: YYYYMMDDHHMMSS_name)
fn parse_timestamp_from_tag(tag: &str) -> i64 {
    // Try to extract timestamp from beginning of tag
    if tag.len() >= 14
        && let Ok(ts) = tag[0..14].parse::<i64>()
    {
        return ts;
    }
    // Fallback: use current time
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// =============================================================================
// Macro for embedding migrations
// =============================================================================

/// Macro to create a vector of migrations from embedded SQL files
///
/// ```ignore
/// use drizzle_migrations::migrations;
///
/// let my_migrations = migrations![
///     ("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
///     ("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
/// ];
/// ```
#[macro_export]
macro_rules! migrations {
    [$(($tag:expr, $sql:expr)),* $(,)?] => {
        vec![
            $(
                $crate::Migration::new($tag, $sql),
            )*
        ]
    };
}
