//! Compile-time embedded migrations
//!
//! This module provides types for migrations that are embedded into the binary
//! at compile time using the `include_migrations!` macro.

use crate::config::Dialect;

/// A single embedded migration entry
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedMigration {
    /// Migration tag (e.g., "0000_steep_colossus")
    pub tag: &'static str,
    /// SQL content (embedded via include_str!)
    pub sql: &'static str,
    /// Index for ordering
    pub idx: u32,
}

impl EmbeddedMigration {
    /// Create a new embedded migration entry
    pub const fn new(tag: &'static str, sql: &'static str, idx: u32) -> Self {
        Self { tag, sql, idx }
    }

    /// Split the SQL into individual statements using breakpoints
    pub fn statements(&self) -> Vec<&str> {
        // Check for drizzle-kit style breakpoints
        if self.sql.contains("--> statement-breakpoint") {
            self.sql
                .split("--> statement-breakpoint")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            // Fall back to semicolon splitting
            self.sql
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        }
    }
}

/// Collection of embedded migrations
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedMigrations {
    /// The embedded migration entries (sorted by idx)
    entries: &'static [EmbeddedMigration],
    /// Dialect for SQL generation
    dialect: Dialect,
}

impl EmbeddedMigrations {
    /// Create a new embedded migrations collection
    pub const fn new(entries: &'static [EmbeddedMigration], dialect: Dialect) -> Self {
        Self { entries, dialect }
    }

    /// Create SQLite migrations
    pub const fn sqlite(entries: &'static [EmbeddedMigration]) -> Self {
        Self::new(entries, Dialect::Sqlite)
    }

    /// Create PostgreSQL migrations
    pub const fn postgresql(entries: &'static [EmbeddedMigration]) -> Self {
        Self::new(entries, Dialect::Postgresql)
    }

    /// Get all migration entries
    pub const fn entries(&self) -> &'static [EmbeddedMigration] {
        self.entries
    }

    /// Get the dialect
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Get the number of migrations
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if there are no migrations
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get migrations that haven't been applied yet
    pub fn pending<'a>(&'a self, applied_tags: &[String]) -> Vec<&'a EmbeddedMigration> {
        self.entries
            .iter()
            .filter(|m| !applied_tags.iter().any(|t| t == m.tag))
            .collect()
    }

    /// Get the SQL to create the migrations tracking table
    pub fn create_table_sql(&self) -> &'static str {
        match self.dialect {
            Dialect::Sqlite => {
                r#"CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);"#
            }
            Dialect::Postgresql => {
                r#"CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
    id SERIAL PRIMARY KEY,
    hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);"#
            }
            Dialect::Mysql => {
                r#"CREATE TABLE IF NOT EXISTS `__drizzle_migrations` (
    id INT PRIMARY KEY AUTO_INCREMENT,
    hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);"#
            }
        }
    }

    /// Get the SQL to query applied migrations
    pub fn query_applied_sql(&self) -> &'static str {
        match self.dialect {
            Dialect::Sqlite | Dialect::Postgresql => {
                r#"SELECT hash FROM "__drizzle_migrations" ORDER BY id;"#
            }
            Dialect::Mysql => r#"SELECT hash FROM `__drizzle_migrations` ORDER BY id;"#,
        }
    }

    /// Get the SQL to record a migration as applied (with placeholder)
    pub fn record_migration_sql(&self) -> &'static str {
        match self.dialect {
            Dialect::Sqlite => r#"INSERT INTO "__drizzle_migrations" (hash) VALUES (?1);"#,
            Dialect::Postgresql => r#"INSERT INTO "__drizzle_migrations" (hash) VALUES ($1);"#,
            Dialect::Mysql => r#"INSERT INTO `__drizzle_migrations` (hash) VALUES (?);"#,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_migration_statements() {
        let migration = EmbeddedMigration::new(
            "0000_test",
            "CREATE TABLE a;\n--> statement-breakpoint\nCREATE TABLE b;",
            0,
        );

        let stmts = migration.statements();
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "CREATE TABLE a;");
        assert_eq!(stmts[1], "CREATE TABLE b;");
    }

    #[test]
    fn test_embedded_migrations_pending() {
        static ENTRIES: &[EmbeddedMigration] = &[
            EmbeddedMigration::new("0000_first", "CREATE TABLE a;", 0),
            EmbeddedMigration::new("0001_second", "CREATE TABLE b;", 1),
            EmbeddedMigration::new("0002_third", "CREATE TABLE c;", 2),
        ];

        let migrations = EmbeddedMigrations::sqlite(ENTRIES);
        let applied = vec!["0000_first".to_string()];
        let pending = migrations.pending(&applied);

        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].tag, "0001_second");
        assert_eq!(pending[1].tag, "0002_third");
    }
}
