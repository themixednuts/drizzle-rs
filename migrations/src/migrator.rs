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
//!     Migration::new("20231220143052_init", include_str!("../drizzle/20231220143052_init/migration.sql")),
//!     Migration::new("20231221093015_users", include_str!("../drizzle/20231221093015_users/migration.sql")),
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
//! // V3 format (folder-based, recommended)
//! let set = MigrationSet::from_dir("./drizzle", Dialect::SQLite)?;
//!
//! // Legacy format (journal-based)
//! let set = MigrationSet::from_dir_legacy("./drizzle", Dialect::SQLite)?;
//! ```

use drizzle_types::Dialect;
use sha2::{Digest, Sha256};
use std::path::Path;

/// A migration with its SQL content
///
/// Represents a single migration that can be applied to the database.
/// The `hash` field is used to track which migrations have been applied.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration tag (folder name)
    tag: String,
    /// Unique hash identifying this migration (computed from SQL content)
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
            tag: tag.to_string(),
            hash,
            created_at,
            sql: statements,
        }
    }

    /// Create a migration with explicit hash and timestamp
    pub fn with_hash(
        tag: impl Into<String>,
        hash: impl Into<String>,
        created_at: i64,
        sql: Vec<String>,
    ) -> Self {
        Self {
            tag: tag.into(),
            hash: hash.into(),
            created_at,
            sql,
        }
    }

    /// Get the migration tag (folder name)
    #[inline]
    pub fn tag(&self) -> &str {
        &self.tag
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

    /// Load migrations from a filesystem directory (V3 folder-based format)
    ///
    /// V3 format discovers migrations by scanning for folders containing `migration.sql`.
    ///
    /// Folders are sorted alphabetically (timestamp prefix ensures correct order).
    pub fn from_dir(dir: impl AsRef<Path>, dialect: Dialect) -> Result<Self, MigratorError> {
        let dir = dir.as_ref();

        if !dir.exists() {
            return Ok(Self::empty(dialect));
        }

        // Match drizzle-orm behavior:
        // if journal exists, treat as legacy format.
        let journal_path = dir.join("meta").join("_journal.json");
        if journal_path.exists() {
            return Self::from_dir_legacy(dir, dialect);
        }

        // Otherwise use V3 folder-based discovery.
        Ok(Self::new(discover_v3_migrations(dir)?, dialect))
    }

    /// Load migrations from a filesystem directory (legacy journal-based format)
    ///
    /// Legacy format uses:
    /// - `meta/_journal.json` - list of migrations
    /// - `{tag}.sql` or `{tag}/migration.sql` - SQL files
    pub fn from_dir_legacy(dir: impl AsRef<Path>, dialect: Dialect) -> Result<Self, MigratorError> {
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
            // Try folder/migration.sql first (hybrid format)
            let folder_path = dir.join(&entry.tag).join("migration.sql");
            // Fall back to {tag}.sql (old flat format)
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
                tag: entry.tag.clone(),
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

    /// Get migrations that haven't been applied yet, based on `created_at`.
    ///
    /// This matches drizzle-orm behavior, where migration execution is tracked
    /// by `created_at` (folder millis), not by hash.
    pub fn pending_by_created_at<'a>(
        &'a self,
        applied_created_at: &[i64],
    ) -> impl Iterator<Item = &'a Migration> {
        self.migrations
            .iter()
            .filter(move |m| !applied_created_at.contains(&m.created_at))
    }

    /// Check if there are any pending migrations
    pub fn has_pending(&self, applied_hashes: &[String]) -> bool {
        self.migrations
            .iter()
            .any(|m| !applied_hashes.contains(&m.hash))
    }

    /// Check if there are pending migrations based on `created_at`.
    pub fn has_pending_by_created_at(&self, applied_created_at: &[i64]) -> bool {
        self.migrations
            .iter()
            .any(|m| !applied_created_at.contains(&m.created_at))
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
    /// - SQLite: id (INTEGER PK AUTOINCREMENT), hash (text), created_at (numeric)
    /// - PostgreSQL: id (SERIAL PK), hash (TEXT), created_at (BIGINT)
    /// - MySQL: id (SERIAL PK), hash (text), created_at (BIGINT)
    pub fn create_table_sql(&self) -> String {
        let table = self.table_ident();

        match self.dialect {
            Dialect::SQLite => format!(
                r#"CREATE TABLE IF NOT EXISTS {} (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash text NOT NULL,
    created_at numeric
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
    id SERIAL PRIMARY KEY,
    hash text NOT NULL,
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

    /// Get the SQL to query all applied migrations (`hash`, `created_at`)
    pub fn query_all_applied_sql(&self) -> String {
        let table = self.table_ident();
        format!(r#"SELECT hash, created_at FROM {} ORDER BY id;"#, table)
    }

    /// Get the SQL to query all applied migration timestamps (`created_at`)
    pub fn query_all_created_at_sql(&self) -> String {
        let table = self.table_ident();
        format!(r#"SELECT created_at FROM {} ORDER BY id;"#, table)
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

// =============================================================================
// V3 Migration Discovery
// =============================================================================

/// Discover migrations in V3 folder-based format
///
/// Scans for directories containing `migration.sql`
fn discover_v3_migrations(dir: &Path) -> Result<Vec<Migration>, MigratorError> {
    use std::fs;

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| MigratorError::IoError(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|entry| {
            let folder_name = entry.file_name().to_string_lossy().to_string();
            let migration_path = entry.path().join("migration.sql");

            if migration_path.exists() {
                Some((folder_name, migration_path))
            } else {
                None
            }
        })
        .collect();

    // Sort by folder name (timestamp prefix ensures correct order)
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut migrations = Vec::with_capacity(entries.len());

    for (tag, sql_path) in entries {
        let sql_content =
            fs::read_to_string(&sql_path).map_err(|e| MigratorError::IoError(e.to_string()))?;

        let hash = compute_hash(&sql_content);
        let created_at = parse_timestamp_from_tag(&tag);
        let statements = split_statements(&sql_content);

        migrations.push(Migration {
            tag,
            hash,
            created_at,
            sql: statements,
        });
    }

    Ok(migrations)
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

/// Compute hash of the SQL content
fn compute_hash(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }

    out
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
    let mut pos = 0;

    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;
    let mut dollar_tag: Option<String> = None;

    while pos < sql.len() {
        // Line comment state
        if in_line_comment {
            let ch = sql[pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            current.push_str(&sql[pos..pos + ch_len]);
            pos += ch_len;
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }

        // Block comment state
        if block_comment_depth > 0 {
            if sql[pos..].starts_with("/*") {
                current.push_str("/*");
                pos += 2;
                block_comment_depth += 1;
                continue;
            }
            if sql[pos..].starts_with("*/") {
                current.push_str("*/");
                pos += 2;
                block_comment_depth = block_comment_depth.saturating_sub(1);
                continue;
            }

            let ch = sql[pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            current.push_str(&sql[pos..pos + ch_len]);
            pos += ch_len;
            continue;
        }

        // Dollar-quoted string state ($$...$$ or $tag$...$tag$)
        if let Some(tag) = dollar_tag.as_deref() {
            if sql[pos..].starts_with(tag) {
                current.push_str(tag);
                pos += tag.len();
                dollar_tag = None;
                continue;
            }

            let ch = sql[pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            current.push_str(&sql[pos..pos + ch_len]);
            pos += ch_len;
            continue;
        }

        // Single-quoted string state
        if in_single_quote {
            if sql[pos..].starts_with("''") {
                current.push_str("''");
                pos += 2;
                continue;
            }
            if sql[pos..].starts_with('\'') {
                current.push('\'');
                pos += 1;
                in_single_quote = false;
                continue;
            }

            let ch = sql[pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            current.push_str(&sql[pos..pos + ch_len]);
            pos += ch_len;
            continue;
        }

        // Double-quoted identifier/string state
        if in_double_quote {
            if sql[pos..].starts_with("\"\"") {
                current.push_str("\"\"");
                pos += 2;
                continue;
            }
            if sql[pos..].starts_with('"') {
                current.push('"');
                pos += 1;
                in_double_quote = false;
                continue;
            }

            let ch = sql[pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            current.push_str(&sql[pos..pos + ch_len]);
            pos += ch_len;
            continue;
        }

        // Enter comment states
        if sql[pos..].starts_with("--") {
            current.push_str("--");
            pos += 2;
            in_line_comment = true;
            continue;
        }
        if sql[pos..].starts_with("/*") {
            current.push_str("/*");
            pos += 2;
            block_comment_depth = 1;
            continue;
        }

        // Enter quote states
        if sql[pos..].starts_with('\'') {
            current.push('\'');
            pos += 1;
            in_single_quote = true;
            continue;
        }
        if sql[pos..].starts_with('"') {
            current.push('"');
            pos += 1;
            in_double_quote = true;
            continue;
        }

        // Enter dollar-quoted state if a valid tag starts here.
        if sql[pos..].starts_with('$')
            && let Some(tag) = parse_dollar_tag_start(sql, pos)
        {
            current.push_str(tag);
            pos += tag.len();
            dollar_tag = Some(tag.to_string());
            continue;
        }

        // Statement boundary
        if sql[pos..].starts_with(';') {
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
            pos += 1;
            continue;
        }

        let ch = sql[pos..].chars().next().unwrap_or('\0');
        let ch_len = ch.len_utf8();
        current.push_str(&sql[pos..pos + ch_len]);
        pos += ch_len;
    }

    // Don't forget the last statement (might not end with ;)
    let stmt = current.trim().to_string();
    if !stmt.is_empty() {
        statements.push(stmt);
    }

    statements
}

/// Parse a starting PostgreSQL dollar-quote delimiter at `pos`.
///
/// Returns the full delimiter (e.g. "$$" or "$func$") when valid.
fn parse_dollar_tag_start(sql: &str, pos: usize) -> Option<&str> {
    if !sql[pos..].starts_with('$') {
        return None;
    }

    let mut i = pos + 1;
    while i < sql.len() {
        let ch = sql[i..].chars().next()?;
        if ch == '$' {
            return Some(&sql[pos..i + 1]);
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            i += ch.len_utf8();
            continue;
        }
        return None;
    }

    None
}

/// Parse timestamp from migration tag
///
/// Supports both V3 format (YYYYMMDDHHMMSS_name) and legacy format (0000_name)
fn parse_timestamp_from_tag(tag: &str) -> i64 {
    // Try to extract timestamp from beginning of tag (V3 format: YYYYMMDDHHMMSS)
    if tag.len() >= 14
        && let Some(ts) = parse_timestamp_prefix_to_millis(&tag[0..14])
    {
        return ts;
    }

    // Try legacy format (0000)
    if tag.len() >= 4
        && let Ok(idx) = tag[0..4].parse::<i64>()
    {
        // Convert index to a pseudo-timestamp for ordering
        return idx;
    }

    // Fallback: use current time
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Parse a `YYYYMMDDHHMMSS` timestamp prefix to UTC milliseconds.
fn parse_timestamp_prefix_to_millis(prefix: &str) -> Option<i64> {
    if prefix.len() != 14 || !prefix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let year = prefix[0..4].parse::<i32>().ok()?;
    let month = prefix[4..6].parse::<u32>().ok()?;
    let day = prefix[6..8].parse::<u32>().ok()?;
    let hour = prefix[8..10].parse::<u32>().ok()?;
    let minute = prefix[10..12].parse::<u32>().ok()?;
    let second = prefix[12..14].parse::<u32>().ok()?;

    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return None;
    }

    let days = days_from_civil(year, month, day)?;
    let day_secs = i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    let secs = days.checked_mul(86_400)?.checked_add(day_secs)?;
    secs.checked_mul(1_000)
}

/// Days since Unix epoch (1970-01-01) from civil date, UTC.
///
/// Algorithm adapted from Howard Hinnant's civil calendar conversion.
fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    let m = i32::try_from(month).ok()?;
    let d = i32::try_from(day).ok()?;

    let y = year - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

    Some(i64::from(era) * 146_097 + i64::from(doe) - 719_468)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
///     ("20231220143052_init", include_str!("../drizzle/20231220143052_init/migration.sql")),
///     ("20231221093015_users", include_str!("../drizzle/20231221093015_users/migration.sql")),
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

#[cfg(test)]
mod tests {
    use super::{MigrationSet, compute_hash, parse_timestamp_from_tag, split_on_semicolons};
    use drizzle_types::Dialect;

    #[test]
    fn split_handles_strings_and_comments() {
        let sql = "\
            CREATE TABLE users(id INTEGER, note TEXT DEFAULT 'a;b');\n\
            -- comment with ; should not split\n\
            CREATE INDEX users_id_idx ON users(id);\n\
            /* block ; comment */\n\
            CREATE TABLE posts(id INTEGER);\
        ";

        let stmts = split_on_semicolons(sql);
        assert_eq!(stmts.len(), 3, "unexpected split: {stmts:?}");
        assert_eq!(
            stmts[0],
            "CREATE TABLE users(id INTEGER, note TEXT DEFAULT 'a;b')"
        );
        assert_eq!(
            stmts[1],
            "-- comment with ; should not split\nCREATE INDEX users_id_idx ON users(id)"
        );
        assert_eq!(
            stmts[2],
            "/* block ; comment */\nCREATE TABLE posts(id INTEGER)"
        );
    }

    #[test]
    fn split_handles_dollar_quoted_bodies() {
        let sql = "\
            CREATE FUNCTION f() RETURNS void AS $$\n\
            BEGIN\n\
              RAISE NOTICE 'x;y';\n\
            END;\n\
            $$ LANGUAGE plpgsql;\n\
            CREATE TABLE t(id INTEGER);\
        ";

        let stmts = split_on_semicolons(sql);
        assert_eq!(stmts.len(), 2, "unexpected split: {stmts:?}");
        assert_eq!(
            stmts[0],
            "CREATE FUNCTION f() RETURNS void AS $$\nBEGIN\nRAISE NOTICE 'x;y';\nEND;\n$$ LANGUAGE plpgsql"
        );
        assert_eq!(stmts[1], "CREATE TABLE t(id INTEGER)");
    }

    #[test]
    fn split_handles_tagged_dollar_quotes() {
        let sql = "\
            DO $body$\n\
            BEGIN\n\
              PERFORM 1;\n\
            END;\n\
            $body$;\n\
            CREATE TABLE tagged(id INTEGER);\
        ";

        let stmts = split_on_semicolons(sql);
        assert_eq!(stmts.len(), 2, "unexpected split: {stmts:?}");
        assert_eq!(stmts[0], "DO $body$\nBEGIN\nPERFORM 1;\nEND;\n$body$");
        assert_eq!(stmts[1], "CREATE TABLE tagged(id INTEGER)");
    }

    #[test]
    fn hash_is_stable_for_same_input() {
        let a = compute_hash("CREATE TABLE users(id INTEGER);");
        let b = compute_hash("CREATE TABLE users(id INTEGER);");
        let c = compute_hash("CREATE TABLE users(id INTEGER PRIMARY KEY);");

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn hash_matches_known_value() {
        let hash = compute_hash("CREATE TABLE users(id INTEGER);");
        assert_eq!(
            hash,
            "238b0b8f98ac8bb3155ac1081ad6a3ce07cfba14eeaa6beeebf2161091265fcc"
        );
    }

    #[test]
    fn parse_timestamp_tag_matches_drizzle_orm_millis() {
        let created_at = parse_timestamp_from_tag("20230331141203_test");
        assert_eq!(created_at, 1_680_271_923_000);
    }

    #[test]
    fn pending_by_created_at_ignores_hash_mismatches() {
        let set = MigrationSet::new(
            vec![super::Migration::with_hash(
                "20230331141203_test",
                "different_hash_than_db",
                1_680_271_923_000,
                vec!["CREATE TABLE users(id INTEGER PRIMARY KEY)".to_string()],
            )],
            Dialect::SQLite,
        );

        let applied_created_at = vec![1_680_271_923_000];
        assert!(!set.has_pending_by_created_at(&applied_created_at));
        assert_eq!(
            set.pending_by_created_at(&applied_created_at).count(),
            0,
            "migration should be considered applied by created_at"
        );
    }

    #[test]
    fn from_dir_discovers_v3_migration_without_snapshot_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let migration_dir = dir.path().join("20230331141203_test");
        std::fs::create_dir_all(&migration_dir).expect("create migration dir");
        std::fs::write(
            migration_dir.join("migration.sql"),
            "CREATE TABLE users(id INTEGER PRIMARY KEY);",
        )
        .expect("write migration.sql");

        let set = MigrationSet::from_dir(dir.path(), Dialect::SQLite).expect("load migrations");
        assert_eq!(set.all().len(), 1);
        assert_eq!(set.all()[0].created_at(), 1_680_271_923_000);
    }

    #[test]
    fn from_dir_prefers_legacy_journal_when_present() {
        let dir = tempfile::tempdir().expect("tempdir");

        let mut journal = crate::journal::Journal::new(Dialect::SQLite);
        journal.add_entry("0000_journal_first".to_string(), true);
        journal
            .save(&dir.path().join("meta").join("_journal.json"))
            .expect("write journal");

        std::fs::write(
            dir.path().join("0000_journal_first.sql"),
            "CREATE TABLE from_journal(id INTEGER PRIMARY KEY);",
        )
        .expect("write legacy migration file");

        // This v3 migration should be ignored because journal format takes precedence.
        let v3_dir = dir.path().join("20240101010101_v3_extra");
        std::fs::create_dir_all(&v3_dir).expect("create v3 dir");
        std::fs::write(
            v3_dir.join("migration.sql"),
            "CREATE TABLE from_v3(id INTEGER PRIMARY KEY);",
        )
        .expect("write v3 migration.sql");

        let set = MigrationSet::from_dir(dir.path(), Dialect::SQLite).expect("load migrations");
        assert_eq!(set.all().len(), 1);
        assert_eq!(set.all()[0].tag(), "0000_journal_first");
    }
}
