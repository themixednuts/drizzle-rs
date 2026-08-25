//! Runtime migration runner for programmatic migrations
//!
//! Provides the low-level pieces behind runtime migration execution:
//! - [`Migration`] values holding SQL and metadata
//! - [`Migrations`] for tracking-table SQL and pending migration checks
//! - [`MigrationDir`](crate::MigrationDir) for filesystem discovery when embedding or testing
//!
//! # Usage
//!
//! ## Embedded Migrations (recommended for production/serverless)
//!
//! Use `drizzle::include_migrations!` or `include_str!` to embed migration SQL at compile time:
//!
//! ```rust
//! # let _ = r####"
//! use drizzle_migrations::{Migration, Migrations};
//! use drizzle_types::Dialect;
//!
//! const MIGRATIONS: &[Migration] = &[
//!     Migration::new("20231220143052_init", include_str!("../drizzle/20231220143052_init/migration.sql")),
//!     Migration::new("20231221093015_users", include_str!("../drizzle/20231221093015_users/migration.sql")),
//! ];
//!
//! async fn run_migrations(db: &Database) -> Result<(), MigratorError> {
//!     let set = Migrations::new(MIGRATIONS.to_vec(), Dialect::SQLite);
//!
//!     // Ensure migrations table exists
//!     db.execute(&set.create_table_sql()).await?;
//!
//!     // Get applied migration names (matches drizzle-orm beta.19+ semantics).
//!     let applied: Vec<String> = db.query_column::<String>(&set.applied_names_sql()).await?;
//!
//!     // Apply pending migrations by name set-difference
//!     for migration in set.pending(&applied) {
//!         for statement in migration.statements() {
//!             db.execute(statement).await?;
//!         }
//!         db.execute(&set.record_migration_sql(migration)).await?;
//!     }
//!     Ok(())
//! }
//! # "####;
//! ```
//!
//! ## Loading from Filesystem (for development)
//!
//! ```rust
//! # let _ = r####"
//! use drizzle_migrations::{MigrationDir, Migrations};
//! use drizzle_types::Dialect;
//!
//! let migrations = MigrationDir::new("./drizzle").discover()?;
//! let set = Migrations::new(migrations, Dialect::SQLite);
//! # "####;
//! ```

use crate::config::Tracking;
use drizzle_types::Dialect;
use sha2::{Digest, Sha256};

fn quote_identifier(dialect: Dialect, identifier: &str) -> String {
    match dialect {
        Dialect::MySQL => format!("`{}`", identifier.replace('`', "``")),
        _ => format!("\"{}\"", identifier.replace('"', "\"\"")),
    }
}

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

/// SQLite statements prepared for execution by a runtime adapter.
///
/// Generated table rebuilds carry `PRAGMA foreign_keys=OFF/ON` sentinels.
/// SQLite ignores those pragmas inside a transaction, so adapters must apply
/// the connection setting before opening their transaction and restore it
/// after completion. The sentinels are excluded from
/// [`SqliteMigrationExecution::statements`].
#[derive(Debug, Clone, Copy)]
pub struct SqliteMigrationExecution<'a> {
    statements: &'a [String],
    suspends_foreign_keys: bool,
}

impl<'a> SqliteMigrationExecution<'a> {
    /// Whether the adapter must disable foreign-key enforcement before its
    /// transaction and restore it afterward.
    #[inline]
    #[must_use]
    pub const fn suspends_foreign_keys(self) -> bool {
        self.suspends_foreign_keys
    }

    /// Statements to execute inside the migration transaction.
    pub fn statements(self) -> impl Iterator<Item = &'a str> + 'a {
        self.statements.iter().filter_map(|statement| {
            sqlite_foreign_keys_setting(statement)
                .expect("SQLite migration execution was validated before construction")
                .is_none()
                .then_some(statement.as_str())
        })
    }
}

/// Invalid SQLite foreign-key suspension sentinels in a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SqliteMigrationExecutionError {
    #[error("PRAGMA foreign_keys=OFF is nested without a matching ON")]
    NestedForeignKeysOff,
    #[error("PRAGMA foreign_keys=ON has no preceding OFF")]
    ForeignKeysOnWithoutOff,
    #[error("PRAGMA foreign_keys=OFF has no matching ON")]
    ForeignKeysOffWithoutOn,
    #[error("unsupported PRAGMA foreign_keys assignment in migration")]
    UnsupportedForeignKeysPragma,
}

/// Outcome of a successful `migrate(...)` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrateOutcome {
    /// The database was already in sync with the local migration set — no
    /// migrations were applied.
    UpToDate,
    /// Pending migrations ran successfully. `tags` contains the folder names
    /// of each applied migration, in execution order.
    Applied { tags: Vec<String> },
}

impl MigrateOutcome {
    /// Was the database already up to date with the local migration set?
    #[inline]
    #[must_use]
    pub const fn is_up_to_date(&self) -> bool {
        matches!(self, Self::UpToDate)
    }

    /// Number of migrations applied during this call (0 when up to date).
    #[inline]
    #[must_use]
    pub fn applied_count(&self) -> usize {
        match self {
            Self::UpToDate => 0,
            Self::Applied { tags } => tags.len(),
        }
    }

    /// Tags of migrations applied during this call (empty when up to date).
    #[inline]
    #[must_use]
    pub fn applied_tags(&self) -> &[String] {
        match self {
            Self::UpToDate => &[],
            Self::Applied { tags } => tags,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMigrationMetadata {
    pub id: Option<i64>,
    pub hash: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedMigrationMetadata {
    pub id: Option<i64>,
    pub hash: String,
    pub created_at: i64,
    pub name: String,
}

impl Migration {
    /// Create a new migration from embedded SQL
    ///
    /// The hash is computed from the SQL content.
    /// SQL is split on `"--> statement-breakpoint"` markers.
    #[must_use]
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
    #[must_use]
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Get the migration folder name used by drizzle-orm tracking metadata.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.tag
    }

    /// Get the migration hash (used for tracking)
    #[inline]
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Get the creation timestamp
    #[inline]
    #[must_use]
    pub const fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Get the raw SQL statements (already split).
    ///
    /// SQLite transaction-owning adapters must use [`Self::sqlite_execution`]
    /// instead so foreign-key suspension sentinels are handled outside the
    /// transaction.
    #[inline]
    #[must_use]
    pub fn statements(&self) -> &[String] {
        &self.sql
    }

    /// Validate SQLite foreign-key suspension sentinels and prepare the
    /// statement stream for a transaction-owning runtime adapter.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteMigrationExecutionError`] when foreign-key suspension
    /// pragmas are nested, unbalanced, or use an unsupported assignment form.
    pub fn sqlite_execution(
        &self,
    ) -> Result<SqliteMigrationExecution<'_>, SqliteMigrationExecutionError> {
        let mut foreign_keys_disabled = false;
        let mut suspends_foreign_keys = false;
        for statement in &self.sql {
            match sqlite_foreign_keys_setting(statement)? {
                Some(false) if foreign_keys_disabled => {
                    return Err(SqliteMigrationExecutionError::NestedForeignKeysOff);
                }
                Some(false) => {
                    foreign_keys_disabled = true;
                    suspends_foreign_keys = true;
                }
                Some(true) if !foreign_keys_disabled => {
                    return Err(SqliteMigrationExecutionError::ForeignKeysOnWithoutOff);
                }
                Some(true) => foreign_keys_disabled = false,
                None => {}
            }
        }
        if foreign_keys_disabled {
            return Err(SqliteMigrationExecutionError::ForeignKeysOffWithoutOn);
        }
        Ok(SqliteMigrationExecution {
            statements: &self.sql,
            suspends_foreign_keys,
        })
    }

    /// Check if this migration is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sql.is_empty() || self.sql.iter().all(|s| s.trim().is_empty())
    }

    /// Whether this migration contains a PostgreSQL concurrent-index command.
    #[must_use]
    pub fn has_postgres_concurrent_index(&self) -> bool {
        self.sql
            .iter()
            .any(|statement| is_postgres_concurrent_index_statement(statement))
    }
}

fn sqlite_foreign_keys_setting(
    statement: &str,
) -> Result<Option<bool>, SqliteMigrationExecutionError> {
    let normalized: String = strip_sql_comments(statement)
        .trim()
        .trim_end_matches(';')
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    match sqlite_foreign_keys_assignment(&normalized) {
        Some("=off" | "=0" | "=false" | "=no" | "(off)" | "(0)" | "(false)" | "(no)") => {
            Ok(Some(false))
        }
        Some("=on" | "=1" | "=true" | "=yes" | "(on)" | "(1)" | "(true)" | "(yes)") => {
            Ok(Some(true))
        }
        Some(_) => Err(SqliteMigrationExecutionError::UnsupportedForeignKeysPragma),
        _ => Ok(None),
    }
}

fn sqlite_foreign_keys_assignment(normalized: &str) -> Option<&str> {
    let pragma = normalized.strip_prefix("pragma")?;
    for name in [
        "foreign_keys",
        "\"foreign_keys\"",
        "'foreign_keys'",
        "`foreign_keys`",
        "[foreign_keys]",
    ] {
        if let Some(assignment) = pragma.strip_prefix(name)
            && matches!(assignment.as_bytes().first(), Some(b'=' | b'('))
        {
            return Some(assignment);
        }
        if let Some((_, assignment)) = pragma.rsplit_once(&format!(".{name}"))
            && matches!(assignment.as_bytes().first(), Some(b'=' | b'('))
        {
            return Some(assignment);
        }
    }
    None
}

fn strip_sql_comments(statement: &str) -> String {
    let mut output = String::with_capacity(statement.len());
    let mut characters = statement.chars().peekable();
    let mut quote = None;

    while let Some(character) = characters.next() {
        if let Some(terminator) = quote {
            output.push(character);
            if character == terminator {
                if terminator != ']' && characters.peek() == Some(&terminator) {
                    output.push(characters.next().expect("peeked quote is present"));
                } else {
                    quote = None;
                }
            }
            continue;
        }

        match character {
            '\'' | '"' | '`' => {
                quote = Some(character);
                output.push(character);
            }
            '[' => {
                quote = Some(']');
                output.push(character);
            }
            '-' if characters.peek() == Some(&'-') => {
                characters.next();
                for comment_character in characters.by_ref() {
                    if comment_character == '\n' {
                        output.push('\n');
                        break;
                    }
                }
            }
            '/' if characters.peek() == Some(&'*') => {
                characters.next();
                let mut closed = false;
                while let Some(comment_character) = characters.next() {
                    if comment_character == '*' && characters.peek() == Some(&'/') {
                        characters.next();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    output.push_str("/*");
                }
            }
            _ => output.push(character),
        }
    }

    output
}

/// A collection of migrations ready to be applied
#[derive(Debug, Clone)]
pub struct Migrations {
    /// Ordered list of migrations
    list: Vec<Migration>,
    /// Database dialect
    dialect: Dialect,
    /// Migrations table name
    table: String,
    /// Migrations schema (`PostgreSQL` only)
    schema: Option<String>,
}

impl Migrations {
    /// Create a new migration set from migrations
    #[must_use]
    pub fn new(migrations: Vec<Migration>, dialect: Dialect) -> Self {
        Self {
            list: migrations,
            dialect,
            table: "__drizzle_migrations".to_string(),
            schema: match dialect {
                Dialect::PostgreSQL => Some("drizzle".to_string()),
                _ => None,
            },
        }
    }

    pub fn with_tracking(migrations: Vec<Migration>, dialect: Dialect, tracking: Tracking) -> Self {
        Self {
            list: migrations,
            dialect,
            table: tracking.table.into_owned(),
            schema: tracking.schema.map(std::borrow::Cow::into_owned),
        }
    }

    /// Create an empty migration set
    #[must_use]
    pub fn empty(dialect: Dialect) -> Self {
        Self::new(Vec::new(), dialect)
    }

    /// Get all migrations
    #[inline]
    #[must_use]
    pub fn all(&self) -> &[Migration] {
        &self.list
    }

    /// Get migrations that haven't been applied yet, by set-difference on name.
    ///
    /// Mirrors drizzle-orm's beta.19 `getMigrationsToRun`: a local migration is
    /// pending iff its `name` (folder name) does not appear in the DB's
    /// migrations table. This is resilient to same-second `created_at`
    /// collisions and re-applies out-of-order migrations (e.g. after a
    /// branch merge) instead of silently skipping them.
    ///
    /// `applied_names` should contain the non-null `name` column values from
    /// the migrations tracking table, typically loaded via
    /// [`Migrations::applied_names_sql`].
    pub fn pending<'a, S>(&'a self, applied_names: &'a [S]) -> impl Iterator<Item = &'a Migration>
    where
        S: AsRef<str>,
    {
        self.list.iter().filter(move |m| {
            let name = m.name();
            !applied_names.iter().any(|applied| applied.as_ref() == name)
        })
    }

    /// Check if there are pending migrations, by name set-difference.
    pub fn has_pending<S>(&self, applied_names: &[S]) -> bool
    where
        S: AsRef<str>,
    {
        self.pending(applied_names).next().is_some()
    }

    /// Get the dialect
    #[inline]
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Get the migrations tracking table name.
    #[inline]
    #[must_use]
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Get the migrations tracking schema, if any.
    #[inline]
    #[must_use]
    pub fn schema_name(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Get the SQL table identifier used in queries.
    #[inline]
    #[must_use]
    pub fn table_ident_sql(&self) -> String {
        self.table_ident()
    }

    /// Stable advisory-lock key for serializing PostgreSQL migration runners.
    #[must_use]
    pub fn postgres_advisory_lock_key(&self) -> i64 {
        let digest =
            Sha256::digest(format!("drizzle-rs:migrate:{}", self.table_ident()).as_bytes());
        i64::from_be_bytes(
            digest[..8]
                .try_into()
                .expect("SHA-256 prefix is eight bytes"),
        )
    }

    /// Whether any migration requires execution outside a PostgreSQL transaction.
    #[must_use]
    pub fn has_postgres_concurrent_index(&self) -> bool {
        self.list
            .iter()
            .any(Migration::has_postgres_concurrent_index)
    }

    /// Create a partial unique index that prevents duplicate non-null names in
    /// the migration tracking table.
    #[must_use]
    pub fn create_name_unique_index_sql(&self) -> Option<String> {
        if self.dialect == Dialect::MySQL {
            return None;
        }
        let digest = Sha256::digest(self.table_ident().as_bytes());
        let suffix = digest[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let index = quote_identifier(self.dialect, &format!("drizzle_migration_name_{suffix}"));
        Some(format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS {index} ON {} (\"name\") WHERE \"name\" IS NOT NULL;",
            self.table_ident()
        ))
    }

    /// Get the full table identifier (with schema for `PostgreSQL`)
    fn table_ident(&self) -> String {
        match (&self.dialect, &self.schema) {
            (Dialect::PostgreSQL, Some(schema)) => format!(
                "{}.{}",
                quote_identifier(self.dialect, schema),
                quote_identifier(self.dialect, &self.table)
            ),
            _ => quote_identifier(self.dialect, &self.table),
        }
    }

    /// Get the SQL to create the migrations schema (`PostgreSQL` only)
    #[must_use]
    pub fn create_schema_sql(&self) -> Option<String> {
        self.schema.as_ref().map(|schema| {
            format!(
                "CREATE SCHEMA IF NOT EXISTS {};",
                quote_identifier(self.dialect, schema)
            )
        })
    }

    /// Get the SQL to create the migrations tracking table
    ///
    /// Table schema matches current drizzle-orm:
    /// - `SQLite`: id (INTEGER PK), hash, `created_at`, name, `applied_at`
    /// - `PostgreSQL`: id (SERIAL PK), hash, `created_at`, name, `applied_at`
    /// - `MySQL`: id (SERIAL PK), hash, `created_at`, name, `applied_at`
    #[must_use]
    pub fn create_table_sql(&self) -> String {
        let table = self.table_ident();

        match self.dialect {
            Dialect::SQLite => format!(
                r"CREATE TABLE IF NOT EXISTS {table} (
    id INTEGER PRIMARY KEY,
    hash text NOT NULL,
    created_at numeric,
    name text,
    applied_at TEXT
);"
            ),
            Dialect::PostgreSQL => format!(
                r"CREATE TABLE IF NOT EXISTS {table} (
    id SERIAL PRIMARY KEY,
    hash TEXT NOT NULL,
    created_at BIGINT,
    name TEXT,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);"
            ),
            Dialect::MySQL => format!(
                r"CREATE TABLE IF NOT EXISTS {table} (
    id SERIAL PRIMARY KEY,
    hash text NOT NULL,
    created_at BIGINT,
    name text,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);"
            ),
        }
    }

    /// Get the SQL to record a migration as applied.
    #[must_use]
    pub fn record_migration_sql(&self, migration: &Migration) -> String {
        let table = self.table_ident();
        let hash = escape_sql_string(migration.hash());
        let name = escape_sql_string(migration.name());
        let created_at = migration.created_at();

        match self.dialect {
            Dialect::SQLite | Dialect::PostgreSQL => {
                format!(
                    r#"INSERT INTO {table} ("hash", "created_at", "name", "applied_at") VALUES ('{hash}', {created_at}, '{name}', CURRENT_TIMESTAMP);"#
                )
            }
            Dialect::MySQL => {
                format!(
                    r"INSERT INTO {table} (`hash`, `created_at`, `name`, `applied_at`) VALUES ('{hash}', {created_at}, '{name}', CURRENT_TIMESTAMP);"
                )
            }
        }
    }

    /// Get the SQL to record a migration as *started* (phase 1 of two-phase
    /// tracking on non-transactional paths).
    ///
    /// The row is written with `applied_at` explicitly `NULL`, which marks the
    /// migration **dirty**: its statements are about to run but have not been
    /// confirmed. [`Migrations::record_migration_finished_sql`] clears the
    /// marker once they have. A crash between the two leaves the dirty row
    /// behind, which is exactly the signal
    /// [`Migrations::interrupted_migration_error`] reports.
    ///
    /// Transactional paths must keep using
    /// [`Migrations::record_migration_sql`] — a single insert inside the same
    /// transaction as the statements is already atomic.
    ///
    /// `applied_at` is written explicitly because the `PostgreSQL` column
    /// carries `DEFAULT CURRENT_TIMESTAMP`; omitting it would silently mark
    /// the migration complete before it ran.
    #[must_use]
    pub fn record_migration_started_sql(&self, migration: &Migration) -> String {
        let table = self.table_ident();
        let hash = escape_sql_string(migration.hash());
        let name = escape_sql_string(migration.name());
        let created_at = migration.created_at();

        match self.dialect {
            Dialect::SQLite | Dialect::PostgreSQL => {
                format!(
                    r#"INSERT INTO {table} ("hash", "created_at", "name", "applied_at") VALUES ('{hash}', {created_at}, '{name}', NULL);"#
                )
            }
            Dialect::MySQL => {
                format!(
                    r"INSERT INTO {table} (`hash`, `created_at`, `name`, `applied_at`) VALUES ('{hash}', {created_at}, '{name}', NULL);"
                )
            }
        }
    }

    /// Get the SQL to mark a started migration as finished (phase 3 of
    /// two-phase tracking).
    ///
    /// Only clears rows that are still dirty, so a concurrent runner that
    /// already completed the migration is not re-stamped.
    #[must_use]
    pub fn record_migration_finished_sql(&self, migration: &Migration) -> String {
        let table = self.table_ident();
        let name = escape_sql_string(migration.name());

        match self.dialect {
            Dialect::MySQL => format!(
                r"UPDATE {table} SET `applied_at` = CURRENT_TIMESTAMP WHERE `name` = '{name}' AND `applied_at` IS NULL;"
            ),
            _ => format!(
                r#"UPDATE {table} SET "applied_at" = CURRENT_TIMESTAMP WHERE "name" = '{name}' AND "applied_at" IS NULL;"#
            ),
        }
    }

    /// Get the SQL to drop a migration's dirty marker.
    ///
    /// Used when a non-transactional run fails on its *first* statement, where
    /// nothing can have been applied and leaving a dirty row would demand a
    /// pointless repair. Never touches a completed row.
    #[must_use]
    pub fn clear_migration_started_sql(&self, migration: &Migration) -> String {
        let table = self.table_ident();
        let name = escape_sql_string(migration.name());

        match self.dialect {
            Dialect::MySQL => {
                format!(r"DELETE FROM {table} WHERE `name` = '{name}' AND `applied_at` IS NULL;")
            }
            _ => {
                format!(r#"DELETE FROM {table} WHERE "name" = '{name}' AND "applied_at" IS NULL;"#)
            }
        }
    }

    /// Get the SQL to backfill `name`/`applied_at` on a legacy tracking row.
    ///
    /// The v0 tracking table had only `id`/`hash`/`created_at`; the upgrade
    /// adds `name` and `applied_at` and backfills both. `applied_at` is derived
    /// from the row's `created_at` rather than left `NULL` — a `NULL` here
    /// would be indistinguishable from an interrupted migration and would make
    /// every upgraded database look dirty.
    #[must_use]
    pub fn backfill_migration_metadata_sql(&self, row: &MatchedMigrationMetadata) -> String {
        let table = self.table_ident();
        let name = escape_sql_string(&row.name);
        let created_at = row.created_at;

        let (name_column, applied_column, applied_expr, where_clause) = match self.dialect {
            Dialect::MySQL => (
                "`name`",
                "`applied_at`",
                format!("FROM_UNIXTIME({created_at} / 1000)"),
                row.id.map_or_else(
                    || {
                        format!(
                            "`created_at` = {created_at} AND `hash` = '{}'",
                            escape_sql_string(&row.hash)
                        )
                    },
                    |id| format!("`id` = {id}"),
                ),
            ),
            Dialect::PostgreSQL => (
                "\"name\"",
                "\"applied_at\"",
                format!("to_timestamp({created_at}::double precision / 1000.0)"),
                row.id.map_or_else(
                    || {
                        format!(
                            "\"created_at\" = {created_at} AND \"hash\" = '{}'",
                            escape_sql_string(&row.hash)
                        )
                    },
                    |id| format!("\"id\" = {id}"),
                ),
            ),
            Dialect::SQLite => (
                "\"name\"",
                "\"applied_at\"",
                format!("datetime({created_at} / 1000, 'unixepoch')"),
                row.id.map_or_else(
                    || {
                        format!(
                            "\"created_at\" = {created_at} AND \"hash\" = '{}'",
                            escape_sql_string(&row.hash)
                        )
                    },
                    |id| format!("\"id\" = {id}"),
                ),
            ),
        };

        format!(
            "UPDATE {table} SET {name_column} = '{name}', {applied_column} = {applied_expr} WHERE {where_clause}"
        )
    }

    /// Get the SQL to query applied migration names.
    ///
    /// A row counts as applied only when it has both a non-null `name` *and* a
    /// non-null `applied_at`:
    ///
    /// * `name IS NULL` — written before the v0 → v1 tracking-table upgrade
    ///   (which backfills `name`), so it cannot be matched to a local
    ///   migration.
    /// * `applied_at IS NULL` — a two-phase **dirty marker**: the migration
    ///   started but was never confirmed complete. Reporting it as applied
    ///   would silently skip a half-applied migration. See
    ///   [`Migrations::dirty_names_sql`].
    ///
    /// Pair with [`Migrations::pending`].
    #[must_use]
    pub fn applied_names_sql(&self) -> String {
        let table = self.table_ident();
        match self.dialect {
            Dialect::MySQL => {
                format!(
                    "SELECT `name` FROM {table} WHERE `name` IS NOT NULL AND `applied_at` IS NOT NULL ORDER BY id;"
                )
            }
            _ => format!(
                r#"SELECT "name" FROM {table} WHERE "name" IS NOT NULL AND "applied_at" IS NOT NULL ORDER BY id;"#
            ),
        }
    }

    /// Get the SQL to query full applied-migration records: `hash`, `name`,
    /// and a `dirty` flag (`applied_at IS NULL` — started but never finished).
    ///
    /// Unlike [`Migrations::applied_names_sql`] this returns interrupted rows
    /// too, so integrity checks can report drift, missing-local, and
    /// interrupted migrations from a single query.
    #[must_use]
    pub fn applied_records_sql(&self) -> String {
        let table = self.table_ident();
        match self.dialect {
            Dialect::MySQL => {
                format!(
                    "SELECT `hash`, `name`, (`applied_at` IS NULL) AS dirty FROM {table} WHERE `name` IS NOT NULL ORDER BY id;"
                )
            }
            _ => format!(
                r#"SELECT "hash", "name", ("applied_at" IS NULL) AS dirty FROM {table} WHERE "name" IS NOT NULL ORDER BY id;"#
            ),
        }
    }

    /// Get the SQL to query interrupted ("dirty") migration names.
    ///
    /// These are rows whose `name` is known but whose `applied_at` is `NULL` —
    /// a migration that started on a non-transactional path and never reported
    /// completion.
    #[must_use]
    pub fn dirty_names_sql(&self) -> String {
        let table = self.table_ident();
        match self.dialect {
            Dialect::MySQL => {
                format!(
                    "SELECT `name` FROM {table} WHERE `name` IS NOT NULL AND `applied_at` IS NULL ORDER BY id;"
                )
            }
            _ => format!(
                r#"SELECT "name" FROM {table} WHERE "name" IS NOT NULL AND "applied_at" IS NULL ORDER BY id;"#
            ),
        }
    }

    /// Build the standard error for interrupted migrations, or `None` when
    /// `dirty_names` is empty.
    ///
    /// Every driver calls this after loading
    /// [`Migrations::dirty_names_sql`] so the message is identical everywhere.
    #[must_use]
    pub fn interrupted_migration_error<S: AsRef<str>>(
        &self,
        dirty_names: &[S],
    ) -> Option<MigratorError> {
        if dirty_names.is_empty() {
            return None;
        }

        let table = self.table_ident();
        let names = dirty_names
            .iter()
            .map(|name| format!("`{}`", name.as_ref()))
            .collect::<Vec<_>>()
            .join(", ");
        let plural = if dirty_names.len() == 1 { "" } else { "s" };
        let first = escape_sql_string(dirty_names[0].as_ref());

        Some(MigratorError::InterruptedMigration(format!(
            "migration{plural} {names} {} interrupted mid-apply: the tracking row in {table} has \
             a NULL `applied_at`, so an earlier run recorded the migration as started but never \
             recorded it as finished. The database may be in a partially-migrated state, and \
             re-running the migration as-is would fail (for example with `table already exists`).\n\
             Recovery options:\n  \
             1. re-run with repair enabled (`drizzle migrate --repair`, or `migrate_with_repair` \
             on the driver) to reconcile each remaining statement against the live schema\n  \
             2. resolve the partial state by hand, then either complete the row \
             (UPDATE {table} SET \"applied_at\" = CURRENT_TIMESTAMP WHERE \"name\" = '{first}';) \
             or discard it and re-run from scratch \
             (DELETE FROM {table} WHERE \"name\" = '{first}';)",
            if dirty_names.len() == 1 {
                "was"
            } else {
                "were"
            },
        )))
    }

    /// Resolve dirty tracking-row names to their local migrations, in local
    /// execution order.
    ///
    /// # Errors
    ///
    /// Returns [`MigratorError::UnrepairableMigration`] when a dirty row names
    /// a migration that is not present locally — repair cannot reconcile
    /// statements it does not have.
    pub fn resolve_dirty_migrations<S: AsRef<str>>(
        &self,
        dirty_names: &[S],
    ) -> Result<Vec<&Migration>, MigratorError> {
        let mut unknown = Vec::new();
        for name in dirty_names {
            if !self.list.iter().any(|m| m.name() == name.as_ref()) {
                unknown.push(name.as_ref().to_string());
            }
        }

        if !unknown.is_empty() {
            return Err(MigratorError::UnrepairableMigration(format!(
                "cannot repair: the tracking table in {} marks migration(s) {} as interrupted, \
                 but they are not present in the local migration set, so their statements are \
                 unknown. Restore the migration folder(s) and retry, or resolve the partial state \
                 by hand and delete the row(s) from {}.",
                self.table_ident(),
                unknown
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.table_ident(),
            )));
        }

        Ok(self
            .list
            .iter()
            .filter(|m| dirty_names.iter().any(|name| name.as_ref() == m.name()))
            .collect())
    }

    /// Get the SQL to check if migrations table exists
    #[must_use]
    pub fn table_exists_sql(&self) -> String {
        let table = self.table.replace('\'', "''");
        match self.dialect {
            Dialect::SQLite => format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{table}';"
            ),
            Dialect::PostgreSQL => self.schema.as_ref().map_or_else(
                || {
                    format!(
                        "SELECT table_name FROM information_schema.tables WHERE table_name='{table}';"
                    )
                },
                |schema| {
                    let schema = schema.replace('\'', "''");
                    format!(
                        "SELECT table_name FROM information_schema.tables WHERE table_schema='{schema}' AND table_name='{table}';"
                    )
                },
            ),
            Dialect::MySQL => format!(
                "SELECT table_name FROM information_schema.tables WHERE table_name='{table}';"
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

    /// A tracking row exists with `applied_at` NULL: the migration started but
    /// never reported completion. Produced by
    /// [`Migrations::interrupted_migration_error`].
    #[error("{0}")]
    InterruptedMigration(String),

    /// Repair could not reconcile every statement of an interrupted migration.
    /// Produced by [`crate::repair::Plan::into_executable`].
    #[error("{0}")]
    UnrepairableMigration(String),
}

/// Detect PostgreSQL `CREATE/DROP INDEX CONCURRENTLY` statements.
#[must_use]
pub fn is_postgres_concurrent_index_statement(sql: &str) -> bool {
    let tokens = sql
        .split_whitespace()
        .take(4)
        .map(|token| token.trim_matches(|character: char| !character.is_ascii_alphabetic()))
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>();

    matches!(
        tokens.as_slice(),
        [create, index, concurrently, ..]
            if create == "CREATE" && index == "INDEX" && concurrently == "CONCURRENTLY"
    ) || matches!(
        tokens.as_slice(),
        [create, unique, index, concurrently, ..]
            if create == "CREATE"
                && unique == "UNIQUE"
                && index == "INDEX"
                && concurrently == "CONCURRENTLY"
    ) || matches!(
        tokens.as_slice(),
        [drop, index, concurrently, ..]
            if drop == "DROP" && index == "INDEX" && concurrently == "CONCURRENTLY"
    )
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute hash of the SQL content
pub(crate) fn compute_hash(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }

    out
}

/// Split SQL content into individual statements
pub(crate) fn split_statements(sql: &str) -> Vec<String> {
    split_on_semicolons(sql)
}

/// Per-statement token context for [`split_on_semicolons`].
///
/// Tracks whether the statement being accumulated is a compound-bodied
/// object (`CREATE TRIGGER|PROCEDURE|FUNCTION|EVENT ... BEGIN ...; END` or a
/// PostgreSQL `BEGIN ATOMIC ...; END` body) so its internal semicolons are
/// not treated as statement boundaries. Mirrors SQLite's
/// `sqlite3_complete()`: a compound body terminates only at an `END` token
/// that directly follows a body semicolon (which keeps `CASE ... END` inside
/// the body inert), itself followed by a semicolon.
#[derive(Default)]
struct StatementState {
    /// First few identifier tokens of the statement (lowercased).
    header_tokens: Vec<String>,
    /// Header names an object kind that can carry a `BEGIN ... END` body.
    compound_header: bool,
    /// Nesting depth of compound bodies within the current statement.
    compound_depth: usize,
    /// Last token was `BEGIN` (a following `ATOMIC` opens a body).
    pending_begin: bool,
    /// Saw a body-terminating `END`; the next semicolon closes one level.
    pending_end: bool,
    /// Positioned at the start of a body statement (right after `BEGIN` or a
    /// body semicolon), where `END` may legally terminate the body.
    at_body_start: bool,
    /// Previous consumed character was part of a word (guards token starts).
    last_char_wordy: bool,
}

impl StatementState {
    /// Kinds of `CREATE` statements that may contain compound bodies.
    const COMPOUND_KINDS: [&'static str; 4] = ["trigger", "procedure", "function", "event"];
    /// `CREATE <kind>` statements that never do (guards against objects
    /// merely *named* `function` etc.).
    const PLAIN_KINDS: [&'static str; 5] = ["table", "index", "view", "schema", "virtual"];

    /// Record significant (non-whitespace, non-comment) content that is not
    /// an identifier token.
    fn note_significant(&mut self) {
        self.pending_begin = false;
        self.pending_end = false;
        self.at_body_start = false;
    }

    /// Process an identifier token encountered in normal state.
    fn note_token(&mut self, token: &str) {
        let lower = token.to_ascii_lowercase();
        let was_pending_begin = self.pending_begin;
        let was_at_body_start = self.at_body_start;
        self.note_significant();

        if self.header_tokens.len() < 6 {
            self.header_tokens.push(lower.clone());
            if self.header_tokens[0] == "create"
                && self.header_tokens.len() > 1
                && !Self::PLAIN_KINDS.contains(&self.header_tokens[1].as_str())
                && self.header_tokens[1..]
                    .iter()
                    .any(|t| Self::COMPOUND_KINDS.contains(&t.as_str()))
            {
                self.compound_header = true;
            }
        }

        match lower.as_str() {
            "begin" if self.compound_header && self.compound_depth == 0 => {
                self.compound_depth = 1;
                self.at_body_start = true;
            }
            "begin" => self.pending_begin = true,
            "atomic" if was_pending_begin => {
                self.compound_depth += 1;
                self.at_body_start = true;
            }
            "end" if self.compound_depth > 0 && was_at_body_start => {
                self.pending_end = true;
            }
            _ => {}
        }
        self.last_char_wordy = true;
    }
}

/// Split SQL on `--> statement-breakpoint` markers and top-level semicolons.
///
/// State-aware: quotes, comments, dollar-quoted bodies, and compound
/// statement bodies (trigger/procedure/function bodies, `BEGIN ATOMIC`)
/// keep their internal semicolons.
fn split_on_semicolons(sql: &str) -> Vec<String> {
    const BREAKPOINT: &str = "--> statement-breakpoint";

    let mut statements = Vec::new();
    let mut current = String::new();
    let mut pos = 0;

    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;
    let mut dollar_tag: Option<String> = None;
    let mut state = StatementState::default();

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
                state.last_char_wordy = true;
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
                state.last_char_wordy = true;
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
                state.last_char_wordy = true;
                continue;
            }

            let ch = sql[pos..].chars().next().unwrap_or('\0');
            let ch_len = ch.len_utf8();
            current.push_str(&sql[pos..pos + ch_len]);
            pos += ch_len;
            continue;
        }

        // Enter comment states
        if sql[pos..].starts_with(BREAKPOINT) && line_prefix_is_whitespace(sql, pos) {
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
            state = StatementState::default();
            pos += BREAKPOINT.len();
            continue;
        }
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
            state.note_significant();
            state.last_char_wordy = false;
            continue;
        }
        if sql[pos..].starts_with('"') {
            current.push('"');
            pos += 1;
            in_double_quote = true;
            state.note_significant();
            state.last_char_wordy = false;
            continue;
        }

        // Enter dollar-quoted state if a valid tag starts here.
        if sql[pos..].starts_with('$')
            && let Some(tag) = parse_dollar_tag_start(sql, pos)
        {
            current.push_str(tag);
            pos += tag.len();
            dollar_tag = Some(tag.to_string());
            state.note_significant();
            state.last_char_wordy = false;
            continue;
        }

        // Statement boundary (inert inside compound bodies)
        if sql[pos..].starts_with(';') {
            pos += 1;
            if state.compound_depth > 0 {
                if state.pending_end {
                    state.pending_end = false;
                    state.compound_depth -= 1;
                }
                if state.compound_depth > 0 {
                    current.push(';');
                    state.at_body_start = true;
                    state.last_char_wordy = false;
                    continue;
                }
                // Depth reached zero: this semicolon closes the compound
                // statement, so fall through to the boundary handling.
            }
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
            state = StatementState::default();
            continue;
        }

        let ch = sql[pos..].chars().next().unwrap_or('\0');
        if !state.last_char_wordy && (ch.is_ascii_alphabetic() || ch == '_') {
            let rest = &sql[pos..];
            let token_len = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            current.push_str(&rest[..token_len]);
            pos += token_len;
            state.note_token(&rest[..token_len]);
            continue;
        }

        let ch_len = ch.len_utf8();
        current.push_str(&sql[pos..pos + ch_len]);
        pos += ch_len;
        if !ch.is_whitespace() {
            state.note_significant();
        }
        state.last_char_wordy = ch.is_ascii_alphanumeric() || ch == '_';
    }

    // Don't forget the last statement (might not end with ;)
    let stmt = current.trim().to_string();
    if !stmt.is_empty() {
        statements.push(stmt);
    }

    statements
}

fn line_prefix_is_whitespace(sql: &str, pos: usize) -> bool {
    let line_start = sql[..pos].rfind('\n').map_or(0, |index| index + 1);
    sql[line_start..pos].chars().all(char::is_whitespace)
}

/// Match applied database rows to local migrations for migration-table upgrades.
///
/// # Errors
///
/// Returns [`MigratorError::ExecutionError`] when one or more `applied_rows`
/// cannot be matched to any local migration by `created_at` or `hash`.
pub fn match_applied_migration_metadata(
    local_migrations: &[Migration],
    applied_rows: &[AppliedMigrationMetadata],
) -> Result<Vec<MatchedMigrationMetadata>, MigratorError> {
    use std::collections::HashMap;

    let mut by_created_at = HashMap::<i64, Vec<&Migration>>::new();
    let mut by_hash = HashMap::<&str, &Migration>::new();

    for migration in local_migrations {
        by_created_at
            .entry(migration.created_at())
            .or_default()
            .push(migration);
        by_hash.insert(migration.hash(), migration);
    }

    let mut matched = Vec::with_capacity(applied_rows.len());
    let mut unmatched = Vec::new();

    for row in applied_rows {
        let migration = match by_created_at.get(&row.created_at) {
            Some(candidates) if candidates.len() == 1 => Some(candidates[0]),
            Some(candidates) if candidates.len() > 1 => {
                candidates.iter().copied().find(|m| m.hash() == row.hash)
            }
            _ => by_hash.get(row.hash.as_str()).copied(),
        };

        if let Some(migration) = migration {
            matched.push(MatchedMigrationMetadata {
                id: row.id,
                hash: row.hash.clone(),
                created_at: row.created_at,
                name: migration.name().to_string(),
            });
        } else {
            unmatched.push(format!(
                "[id: {:?}, created_at: {}, hash: {}]",
                row.id, row.created_at, row.hash
            ));
        }
    }

    if unmatched.is_empty() {
        Ok(matched)
    } else {
        Err(MigratorError::ExecutionError(format!(
            "database contains applied migrations that do not match local migrations: {}",
            unmatched.join(", ")
        )))
    }
}

fn escape_sql_string(value: &str) -> String {
    value.replace('\'', "''")
}

/// Parse a starting `PostgreSQL` dollar-quote delimiter at `pos`.
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
            return Some(&sql[pos..=i]);
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
/// Supports both V3 format (`YYYYMMDDHHMMSS_name`) and legacy format (`0000_name`)
pub(crate) fn parse_timestamp_from_tag(tag: &str) -> i64 {
    // Try to extract timestamp from beginning of tag (V3 format: YYYYMMDDHHMMSS)
    if let Some(prefix) = tag.get(0..14)
        && let Some(ts) = parse_timestamp_prefix_to_millis(prefix)
    {
        return ts;
    }

    // Try legacy format (0000)
    if let Some(prefix) = tag.get(0..4)
        && let Ok(idx) = prefix.parse::<i64>()
    {
        // Convert index to a pseudo-timestamp for ordering
        return idx;
    }

    // No timestamp or index prefix (e.g. `PrefixMode::None` tags): use a
    // stable sentinel so `created_at` is deterministic across processes;
    // name/hash matching identifies these rows instead.
    0
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

    let y = year - i32::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

    Some(i64::from(era) * 146_097 + i64::from(doe) - 719_468)
}

const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// =============================================================================
// Macro for embedding migrations
// =============================================================================

/// Macro to create a vector of migrations from embedded SQL files
///
/// ```rust
/// # let _ = r####"
/// use drizzle_migrations::migrations;
///
/// let my_migrations = migrations![
///     ("20231220143052_init", include_str!("../drizzle/20231220143052_init/migration.sql")),
///     ("20231221093015_users", include_str!("../drizzle/20231221093015_users/migration.sql")),
/// ];
/// # "####;
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
    use super::{
        AppliedMigrationMetadata, Migration, Migrations, SqliteMigrationExecutionError,
        compute_hash, is_postgres_concurrent_index_statement, match_applied_migration_metadata,
        parse_timestamp_from_tag, split_on_semicolons, split_statements,
    };
    use crate::config::Tracking;
    use crate::dir::MigrationDir;
    use drizzle_types::Dialect;

    #[test]
    fn sqlite_execution_lifts_foreign_key_pragmas_out_of_transactions() {
        let migration = Migration::new(
            "0001_rebuild",
            "PRAGMA foreign_keys = OFF;\n--> statement-breakpoint\nCREATE TABLE records (id INTEGER);\n--> statement-breakpoint\nPRAGMA foreign_keys=ON;",
        );

        let execution = migration.sqlite_execution().expect("valid suspension");
        assert!(execution.suspends_foreign_keys());
        assert_eq!(
            execution.statements().collect::<Vec<_>>(),
            vec!["CREATE TABLE records (id INTEGER)"]
        );
    }

    #[test]
    fn sqlite_execution_rejects_unbalanced_foreign_key_pragmas() {
        let missing_on = Migration::new("0001", "PRAGMA foreign_keys=OFF;");
        assert_eq!(
            missing_on.sqlite_execution().unwrap_err(),
            SqliteMigrationExecutionError::ForeignKeysOffWithoutOn
        );

        let missing_off = Migration::new("0002", "PRAGMA foreign_keys=ON;");
        assert_eq!(
            missing_off.sqlite_execution().unwrap_err(),
            SqliteMigrationExecutionError::ForeignKeysOnWithoutOff
        );

        let nested = Migration::new(
            "0003",
            "PRAGMA foreign_keys=OFF;\n--> statement-breakpoint\nPRAGMA foreign_keys=OFF;\n--> statement-breakpoint\nPRAGMA foreign_keys=ON;",
        );
        assert_eq!(
            nested.sqlite_execution().unwrap_err(),
            SqliteMigrationExecutionError::NestedForeignKeysOff
        );

        let unsupported = Migration::new(
            "0004",
            "PRAGMA foreign_keys=disabled;\n--> statement-breakpoint\nPRAGMA foreign_keys=ON;",
        );
        assert_eq!(
            unsupported.sqlite_execution().unwrap_err(),
            SqliteMigrationExecutionError::UnsupportedForeignKeysPragma
        );
    }

    #[test]
    fn sqlite_execution_accepts_parenthesized_and_commented_pragmas() {
        let migration = Migration::new(
            "0001_rebuild",
            "-- generated rebuild guard\nPRAGMA /* suspend enforcement */ main.'foreign_keys'(OFF);\n--> statement-breakpoint\nCREATE TABLE records (id INTEGER);\n--> statement-breakpoint\nPRAGMA \"main\".\"foreign_keys\" /* restore enforcement */ (ON);",
        );

        let execution = migration.sqlite_execution().expect("valid suspension");
        assert!(execution.suspends_foreign_keys());
        assert_eq!(
            execution.statements().collect::<Vec<_>>(),
            vec!["CREATE TABLE records (id INTEGER)"]
        );
    }

    #[test]
    fn migration_tracking_identifiers_are_escaped_per_dialect() {
        let sqlite = Migrations::with_tracking(
            Vec::new(),
            Dialect::SQLite,
            Tracking::new("migration\"records", None::<String>),
        );
        assert_eq!(sqlite.table_ident_sql(), "\"migration\"\"records\"");
        assert!(
            sqlite
                .create_table_sql()
                .starts_with("CREATE TABLE IF NOT EXISTS \"migration\"\"records\"")
        );

        let postgres = Migrations::with_tracking(
            Vec::new(),
            Dialect::PostgreSQL,
            Tracking::new("migration\"records", Some("audit\"schema")),
        );
        assert_eq!(
            postgres.table_ident_sql(),
            "\"audit\"\"schema\".\"migration\"\"records\""
        );
        assert_eq!(
            postgres.create_schema_sql().as_deref(),
            Some("CREATE SCHEMA IF NOT EXISTS \"audit\"\"schema\";")
        );

        let mysql = Migrations::with_tracking(
            Vec::new(),
            Dialect::MySQL,
            Tracking::new("migration`records", None::<String>),
        );
        assert_eq!(mysql.table_ident_sql(), "`migration``records`");
    }

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
    fn split_keeps_sqlite_trigger_bodies_intact() {
        let sql = "\
            CREATE TABLE logs(msg TEXT);\n\
            CREATE TRIGGER users_ai AFTER INSERT ON users FOR EACH ROW BEGIN\n\
              INSERT INTO logs(msg) VALUES ('added;removed');\n\
              UPDATE counters SET n = n + 1 WHERE id = 1;\n\
            END;\n\
            CREATE INDEX logs_msg_idx ON logs(msg);\
        ";

        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 3, "unexpected split: {stmts:?}");
        assert_eq!(stmts[0], "CREATE TABLE logs(msg TEXT)");
        assert!(stmts[1].starts_with("CREATE TRIGGER users_ai"));
        assert!(
            stmts[1].ends_with("END"),
            "trigger body truncated: {}",
            stmts[1]
        );
        assert!(stmts[1].contains("VALUES ('added;removed');"));
        assert!(stmts[1].contains("WHERE id = 1;"));
        assert_eq!(stmts[2], "CREATE INDEX logs_msg_idx ON logs(msg)");
    }

    #[test]
    fn split_trigger_body_with_case_end_stays_intact() {
        let sql = "\
            CREATE TRIGGER t1 BEFORE UPDATE ON t WHEN (new.n > old.n) BEGIN\n\
              UPDATE t SET status = CASE WHEN new.n > 0 THEN 'pos' ELSE 'neg' END;\n\
              DELETE FROM audit WHERE id = old.id;\n\
            END;\n\
            CREATE TABLE afterwards(id INTEGER);\
        ";

        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 2, "unexpected split: {stmts:?}");
        assert!(stmts[0].starts_with("CREATE TRIGGER t1"));
        assert!(
            stmts[0].ends_with("END"),
            "trigger body truncated: {}",
            stmts[0]
        );
        assert!(stmts[0].contains("ELSE 'neg' END;"));
        assert_eq!(stmts[1], "CREATE TABLE afterwards(id INTEGER)");
    }

    #[test]
    fn split_keeps_begin_atomic_bodies_intact() {
        let sql = "\
            CREATE FUNCTION add_one(x int) RETURNS int LANGUAGE SQL BEGIN ATOMIC\n\
              SELECT x + 1;\n\
            END;\n\
            CREATE TABLE t(id INTEGER);\
        ";

        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 2, "unexpected split: {stmts:?}");
        assert!(stmts[0].starts_with("CREATE FUNCTION add_one"));
        assert!(
            stmts[0].ends_with("END"),
            "atomic body truncated: {}",
            stmts[0]
        );
        assert!(stmts[0].contains("SELECT x + 1;"));
        assert_eq!(stmts[1], "CREATE TABLE t(id INTEGER)");
    }

    #[test]
    fn split_plain_begin_transaction_still_splits() {
        let sql = "BEGIN;\nUPDATE t SET a = 1;\nCOMMIT;";

        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 3, "unexpected split: {stmts:?}");
        assert_eq!(stmts[0], "BEGIN");
        assert_eq!(stmts[1], "UPDATE t SET a = 1");
        assert_eq!(stmts[2], "COMMIT");
    }

    #[test]
    fn split_markers_and_trigger_bodies_coexist() {
        let sql = "\
            CREATE TABLE users(id INTEGER);\n\
            --> statement-breakpoint\n\
            CREATE TRIGGER trg AFTER DELETE ON users BEGIN\n\
              INSERT INTO audit(msg) VALUES ('gone');\n\
            END;\n\
            --> statement-breakpoint\n\
            CREATE TABLE audit(msg TEXT);\
        ";

        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 3, "unexpected split: {stmts:?}");
        assert!(stmts[1].starts_with("CREATE TRIGGER trg"));
        assert!(
            stmts[1].ends_with("END"),
            "trigger body truncated: {}",
            stmts[1]
        );
    }

    #[test]
    fn breakpoints_split_only_at_top_level_marker_lines() {
        let sql = r#"
            CREATE TABLE notes(value TEXT DEFAULT '--> statement-breakpoint');
            -- ordinary comment containing --> statement-breakpoint
            CREATE FUNCTION marker_text() RETURNS text AS $$
            BEGIN
              RETURN '--> statement-breakpoint';
            END;
            $$ LANGUAGE plpgsql;
            --> statement-breakpoint
            CREATE TABLE users(id INTEGER);
        "#;

        let statements = split_statements(sql);
        assert_eq!(statements.len(), 3, "unexpected split: {statements:?}");
        assert!(statements[1].contains("ordinary comment containing"));
        assert!(statements[1].contains("RETURN '--> statement-breakpoint'"));
        assert_eq!(statements[2], "CREATE TABLE users(id INTEGER)");
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
    fn concurrent_index_detection_is_token_aware() {
        assert!(is_postgres_concurrent_index_statement(
            "CREATE INDEX CONCURRENTLY users_email ON users (email)"
        ));
        assert!(is_postgres_concurrent_index_statement(
            "CREATE UNIQUE INDEX CONCURRENTLY users_email ON users (email)"
        ));
        assert!(is_postgres_concurrent_index_statement(
            "DROP INDEX CONCURRENTLY users_email"
        ));
        assert!(!is_postgres_concurrent_index_statement(
            "SELECT 'CREATE INDEX CONCURRENTLY hidden in text'"
        ));
    }

    #[test]
    fn postgres_advisory_lock_key_is_stable_per_tracking_table() {
        let first = Migrations::with_tracking(
            Vec::new(),
            Dialect::PostgreSQL,
            Tracking::new("migrations", Some("audit")),
        );
        let same = first.clone();
        let different = Migrations::with_tracking(
            Vec::new(),
            Dialect::PostgreSQL,
            Tracking::new("other_migrations", Some("audit")),
        );

        assert_eq!(
            first.postgres_advisory_lock_key(),
            same.postgres_advisory_lock_key()
        );
        assert_ne!(
            first.postgres_advisory_lock_key(),
            different.postgres_advisory_lock_key()
        );
    }

    #[test]
    fn parse_timestamp_tag_matches_drizzle_orm_millis() {
        let created_at = parse_timestamp_from_tag("20230331141203_test");
        assert_eq!(created_at, 1_680_271_923_000);
    }

    #[test]
    fn pending_is_set_difference_by_folder_name() {
        // Mirrors drizzle-orm beta.19 `getMigrationsToRun`: two migrations in
        // the same wall-second must both run if only one has been applied.
        let set = Migrations::new(
            vec![
                super::Migration::with_hash(
                    "20230331141203_alpha",
                    "hash_a",
                    1_680_271_923_000,
                    vec!["A".into()],
                ),
                super::Migration::with_hash(
                    "20230331141203_beta",
                    "hash_b",
                    1_680_271_923_000,
                    vec!["B".into()],
                ),
                super::Migration::with_hash(
                    "20230331141500_gamma",
                    "hash_c",
                    1_680_272_100_000,
                    vec!["C".into()],
                ),
            ],
            Dialect::SQLite,
        );

        let applied_names = vec!["20230331141203_alpha".to_string()];
        let pending: Vec<_> = set
            .pending(&applied_names)
            .map(|m| m.tag().to_string())
            .collect();

        assert_eq!(
            pending,
            vec![
                "20230331141203_beta".to_string(),
                "20230331141500_gamma".to_string()
            ],
            "beta shares a created_at with alpha but must still run"
        );
        assert!(set.has_pending(&applied_names));
    }

    #[test]
    fn pending_skips_already_applied_out_of_order() {
        // Upstream behavior: a later migration being applied first (e.g. after
        // a branch merge) does not cause earlier pending migrations to be
        // skipped.
        let set = Migrations::new(
            vec![
                super::Migration::with_hash(
                    "20240101010101_feature_a",
                    "hash_a",
                    1_704_070_861_000,
                    vec!["A".into()],
                ),
                super::Migration::with_hash(
                    "20240102010101_feature_b",
                    "hash_b",
                    1_704_157_261_000,
                    vec!["B".into()],
                ),
            ],
            Dialect::SQLite,
        );

        let applied_names = vec!["20240102010101_feature_b".to_string()];
        let pending: Vec<_> = set
            .pending(&applied_names)
            .map(|m| m.tag().to_string())
            .collect();

        assert_eq!(pending, vec!["20240101010101_feature_a".to_string()]);
    }

    #[test]
    fn applied_names_sql_selects_only_non_null_rows() {
        let set = Migrations::new(Vec::new(), Dialect::PostgreSQL);
        let sql = set.applied_names_sql();
        assert!(sql.contains("\"name\" IS NOT NULL"));
        assert!(sql.contains("ORDER BY id"));
        // PostgreSQL sets use schema-qualified identifiers by default.
        assert!(sql.contains("\"drizzle\".\"__drizzle_migrations\""));
    }

    #[test]
    fn applied_records_sql_exposes_hash_and_dirty_flag() {
        let set = Migrations::new(Vec::new(), Dialect::PostgreSQL);
        let sql = set.applied_records_sql();
        assert!(sql.contains("\"hash\""));
        assert!(sql.contains("(\"applied_at\" IS NULL) AS dirty"));
        // Unlike applied_names_sql, dirty rows are included so integrity
        // checks can report them.
        assert!(!sql.contains("\"applied_at\" IS NOT NULL"));

        let mysql = Migrations::new(Vec::new(), Dialect::MySQL);
        let sql = mysql.applied_records_sql();
        assert!(sql.contains("`hash`"));
        assert!(sql.contains("(`applied_at` IS NULL) AS dirty"));
    }

    fn sample_migration() -> super::Migration {
        super::Migration::with_hash(
            "20230331141203_test",
            "abc123",
            1_680_271_923_000,
            vec!["CREATE TABLE users(id INTEGER PRIMARY KEY)".to_string()],
        )
    }

    #[test]
    fn applied_names_sql_excludes_dirty_rows() {
        for dialect in [Dialect::SQLite, Dialect::PostgreSQL, Dialect::MySQL] {
            let set = Migrations::new(Vec::new(), dialect);
            let applied = set.applied_names_sql();
            let dirty = set.dirty_names_sql();

            if dialect == Dialect::MySQL {
                assert!(applied.contains("`applied_at` IS NOT NULL"), "{applied}");
                assert!(dirty.contains("`applied_at` IS NULL"), "{dirty}");
                assert!(dirty.contains("`name` IS NOT NULL"), "{dirty}");
            } else {
                assert!(applied.contains("\"applied_at\" IS NOT NULL"), "{applied}");
                assert!(dirty.contains("\"applied_at\" IS NULL"), "{dirty}");
                assert!(dirty.contains("\"name\" IS NOT NULL"), "{dirty}");
            }
            assert!(dirty.contains("ORDER BY id"));
        }
    }

    #[test]
    fn two_phase_tracking_sql_marks_then_clears_dirty() {
        let migration = sample_migration();
        let set = Migrations::new(vec![migration.clone()], Dialect::SQLite);

        let started = set.record_migration_started_sql(&migration);
        assert!(started.starts_with("INSERT INTO"));
        assert!(
            started.contains("'20230331141203_test', NULL)"),
            "phase 1 must write applied_at NULL explicitly: {started}"
        );

        let finished = set.record_migration_finished_sql(&migration);
        assert!(finished.starts_with("UPDATE"));
        assert!(finished.contains("\"applied_at\" = CURRENT_TIMESTAMP"));
        assert!(
            finished.contains("\"applied_at\" IS NULL"),
            "phase 3 must only clear a still-dirty row: {finished}"
        );

        let cleared = set.clear_migration_started_sql(&migration);
        assert!(cleared.starts_with("DELETE FROM"));
        assert!(cleared.contains("\"applied_at\" IS NULL"));
    }

    #[test]
    fn two_phase_tracking_sql_quotes_per_dialect() {
        let migration = sample_migration();

        let postgres = Migrations::new(vec![migration.clone()], Dialect::PostgreSQL);
        assert!(
            postgres
                .record_migration_started_sql(&migration)
                .contains("\"drizzle\".\"__drizzle_migrations\"")
        );

        let mysql = Migrations::with_tracking(
            vec![migration.clone()],
            Dialect::MySQL,
            Tracking::new("__drizzle_migrations", None::<String>),
        );
        let started = mysql.record_migration_started_sql(&migration);
        assert!(started.contains("`hash`"), "{started}");
        assert!(started.contains("NULL)"), "{started}");
        assert!(
            mysql
                .record_migration_finished_sql(&migration)
                .contains("`applied_at` = CURRENT_TIMESTAMP")
        );
    }

    #[test]
    fn started_row_is_not_reported_as_applied() {
        // The started/finished pair is the only difference between "pending",
        // "dirty" and "applied", so the predicates must be exact complements.
        let set = Migrations::new(Vec::new(), Dialect::SQLite);
        assert_ne!(set.applied_names_sql(), set.dirty_names_sql());
        assert!(!set.applied_names_sql().contains("IS NULL ORDER"));
    }

    #[test]
    fn interrupted_migration_error_is_none_when_clean() {
        let set = Migrations::new(Vec::new(), Dialect::SQLite);
        assert!(
            set.interrupted_migration_error::<String>(&[]).is_none(),
            "no dirty rows means no error"
        );
    }

    #[test]
    fn interrupted_migration_error_names_migration_and_recovery() {
        let set = Migrations::new(Vec::new(), Dialect::SQLite);
        let error = set
            .interrupted_migration_error(&["20230331141203_test"])
            .expect("dirty row must produce an error");
        let text = error.to_string();

        assert!(text.contains("`20230331141203_test`"), "{text}");
        assert!(text.contains("interrupted mid-apply"), "{text}");
        assert!(text.contains("NULL `applied_at`"), "{text}");
        assert!(text.contains("drizzle migrate --repair"), "{text}");
        assert!(text.contains("migrate_with_repair"), "{text}");
        assert!(
            text.contains("UPDATE \"__drizzle_migrations\" SET"),
            "{text}"
        );
        assert!(
            text.contains("DELETE FROM \"__drizzle_migrations\""),
            "{text}"
        );
        assert!(matches!(
            error,
            super::MigratorError::InterruptedMigration(_)
        ));
    }

    #[test]
    fn interrupted_migration_error_pluralizes_and_lists_all() {
        let set = Migrations::new(Vec::new(), Dialect::SQLite);
        let text = set
            .interrupted_migration_error(&["a_one", "b_two"])
            .expect("dirty rows")
            .to_string();
        assert!(text.contains("migrations `a_one`, `b_two` were"), "{text}");
    }

    #[test]
    fn backfill_metadata_sql_sets_applied_at_from_created_at() {
        let row = super::MatchedMigrationMetadata {
            id: Some(7),
            hash: "abc".to_string(),
            created_at: 1_680_271_923_000,
            name: "20230331141203_test".to_string(),
        };

        let sqlite =
            Migrations::new(Vec::new(), Dialect::SQLite).backfill_migration_metadata_sql(&row);
        assert!(
            sqlite.contains("\"name\" = '20230331141203_test'"),
            "{sqlite}"
        );
        assert!(
            sqlite.contains("\"applied_at\" = datetime(1680271923000 / 1000, 'unixepoch')"),
            "legacy rows must not look dirty: {sqlite}"
        );
        assert!(sqlite.contains("\"id\" = 7"), "{sqlite}");
        assert!(!sqlite.contains("= NULL"), "{sqlite}");

        let postgres =
            Migrations::new(Vec::new(), Dialect::PostgreSQL).backfill_migration_metadata_sql(&row);
        assert!(postgres.contains("to_timestamp("), "{postgres}");
        assert!(!postgres.contains("= NULL"), "{postgres}");
    }

    #[test]
    fn backfill_metadata_sql_falls_back_to_hash_when_id_is_missing() {
        let row = super::MatchedMigrationMetadata {
            id: None,
            hash: "ab'c".to_string(),
            created_at: 12,
            name: "tag".to_string(),
        };
        let sql =
            Migrations::new(Vec::new(), Dialect::SQLite).backfill_migration_metadata_sql(&row);
        assert!(sql.contains("\"created_at\" = 12"), "{sql}");
        assert!(sql.contains("\"hash\" = 'ab''c'"), "{sql}");
    }

    #[test]
    fn record_migration_sql_includes_name_and_applied_at() {
        let migration = super::Migration::with_hash(
            "20230331141203_test",
            "abc123",
            1_680_271_923_000,
            vec!["CREATE TABLE users(id INTEGER PRIMARY KEY)".to_string()],
        );
        let set = Migrations::new(vec![migration.clone()], Dialect::SQLite);

        let sql = set.record_migration_sql(&migration);
        assert!(sql.contains("\"name\""));
        assert!(sql.contains("\"applied_at\""));
        assert!(sql.contains("20230331141203_test"));
    }

    #[test]
    fn match_applied_metadata_prefers_hash_when_created_at_collides() {
        let migrations = vec![
            super::Migration::with_hash(
                "20230331141203_alpha",
                "hash_a",
                1_680_271_923_000,
                vec!["A".to_string()],
            ),
            super::Migration::with_hash(
                "20230331141203_beta",
                "hash_b",
                1_680_271_923_000,
                vec!["B".to_string()],
            ),
        ];

        let matched = match_applied_migration_metadata(
            &migrations,
            &[AppliedMigrationMetadata {
                id: Some(1),
                hash: "hash_b".to_string(),
                created_at: 1_680_271_923_000,
            }],
        )
        .expect("match metadata");

        assert_eq!(matched[0].name, "20230331141203_beta");
    }

    #[test]
    fn match_applied_metadata_errors_for_unmatched_rows() {
        let migrations = vec![super::Migration::with_hash(
            "20230331141203_alpha",
            "hash_a",
            1_680_271_923_000,
            vec!["A".to_string()],
        )];

        let err = match_applied_migration_metadata(
            &migrations,
            &[AppliedMigrationMetadata {
                id: Some(9),
                hash: "missing_hash".to_string(),
                created_at: 1_680_271_924_000,
            }],
        )
        .expect_err("should reject unmatched metadata");

        assert!(err.to_string().contains("do not match local migrations"));
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

        let migrations = MigrationDir::new(dir.path())
            .discover()
            .expect("load migrations");
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].created_at(), 1_680_271_923_000);
    }

    #[test]
    fn from_dir_prefers_v3_when_both_formats_present() {
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

        // V3 migration should be preferred over legacy journal metadata when both are present.
        let v3_dir = dir.path().join("20240101010101_v3_extra");
        std::fs::create_dir_all(&v3_dir).expect("create v3 dir");
        std::fs::write(
            v3_dir.join("migration.sql"),
            "CREATE TABLE from_v3(id INTEGER PRIMARY KEY);",
        )
        .expect("write v3 migration.sql");

        let migrations = MigrationDir::new(dir.path())
            .discover()
            .expect_err("legacy journal should be rejected");
        assert!(
            migrations
                .to_string()
                .contains("old drizzle-kit migration folders")
        );
    }

    #[test]
    fn from_dir_rejects_legacy_journal_when_no_v3_dirs() {
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
        let err = MigrationDir::new(dir.path())
            .discover()
            .expect_err("legacy journal should be rejected");
        assert!(
            err.to_string()
                .contains("old drizzle-kit migration folders")
        );
    }
}
