//! Runtime migration runner: the pieces drivers use to apply migrations.
//!
//! - [`Migration`] holds one migration's tag, hash, and split SQL statements.
//! - [`Migrations`] is an ordered set of migrations plus the SQL for the
//!   tracking table (create, record, query applied/dirty rows).
//! - [`MigrationDir`](crate::MigrationDir) discovers migrations on disk.
//!
//! Most apps never use these directly: the drizzle drivers call them from
//! `db.migrate(...)`. Use them when writing your own runner.
//!
//! # Examples
//!
//! A minimal runner loop. `execute` and `query_names` stand in for your
//! database calls.
//!
//! ```rust
//! use drizzle_migrations::{Migration, Migrations};
//! use drizzle_types::Dialect;
//!
//! # fn execute(_sql: &str) {}
//! # fn query_names(_sql: &str) -> Vec<String> { vec!["20231220143052_init".into()] }
//! let set = Migrations::new(
//!     vec![
//!         Migration::new("20231220143052_init", "CREATE TABLE users (id INTEGER);"),
//!         Migration::new("20231221093015_posts", "CREATE TABLE posts (id INTEGER);"),
//!     ],
//!     Dialect::SQLite,
//! );
//!
//! // 1. Make sure the tracking table exists.
//! execute(&set.create_table_sql());
//!
//! // 2. Load the names of migrations that already ran.
//! let applied = query_names(&set.applied_names_sql());
//!
//! // 3. Run each pending migration, then record it.
//! let pending: Vec<_> = set.pending(&applied).collect();
//! assert_eq!(pending.len(), 1);
//! for migration in pending {
//!     for statement in migration.statements() {
//!         execute(statement);
//!     }
//!     execute(&set.record_migration_sql(migration));
//! }
//! ```
//!
//! Embed SQL files at compile time with `drizzle::include_migrations!`, or
//! load them from disk during development:
//!
//! ```rust,no_run
//! use drizzle_migrations::{MigrationDir, Migrations};
//! use drizzle_types::Dialect;
//!
//! let migrations = MigrationDir::new("./drizzle").discover()?;
//! let set = Migrations::new(migrations, Dialect::SQLite);
//! # let _ = set;
//! # Ok::<(), drizzle_migrations::MigratorError>(())
//! ```

use crate::config::Tracking;
use drizzle_types::Dialect;
use sha2::{Digest, Sha256};

pub(crate) fn quote_identifier(dialect: Dialect, identifier: &str) -> String {
    match dialect {
        Dialect::MySQL => format!("`{}`", identifier.replace('`', "``")),
        _ => format!("\"{}\"", identifier.replace('"', "\"\"")),
    }
}

/// One migration: its tag (folder name), content hash, and SQL statements.
///
/// Pending migrations are found by [`name`](Self::name). The
/// [`hash`](Self::hash) (SHA-256 of the SQL file) is stored alongside it in
/// the tracking table and is used to detect drift and to match legacy rows.
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
    /// The file contents, kept only when they had no breakpoints and were
    /// split without knowing the dialect, so [`Migrations`] can re-split
    /// them with its dialect's rules.
    unsplit: Option<String>,
}

/// SQLite statements prepared for execution by a runtime adapter.
///
/// Built by [`Migration::sqlite_execution`]. Generated table rebuilds carry `PRAGMA foreign_keys=OFF/ON` sentinels.
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
    /// `PRAGMA foreign_keys=OFF` appears again before the matching `ON`.
    #[error("PRAGMA foreign_keys=OFF is nested without a matching ON")]
    NestedForeignKeysOff,
    /// `PRAGMA foreign_keys=ON` appears without an earlier `OFF`.
    #[error("PRAGMA foreign_keys=ON has no preceding OFF")]
    ForeignKeysOnWithoutOff,
    /// `PRAGMA foreign_keys=OFF` is never turned back `ON`.
    #[error("PRAGMA foreign_keys=OFF has no matching ON")]
    ForeignKeysOffWithoutOn,
    /// The pragma assigns a value other than on/off, 1/0, true/false, yes/no.
    #[error("unsupported PRAGMA foreign_keys assignment in migration")]
    UnsupportedForeignKeysPragma,
}

/// Result of a successful `migrate(...)` call.
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
    /// Returns `true` when no migrations had to be applied.
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

/// A row read from a legacy tracking table that has no `name` column.
///
/// Input to [`match_applied_migration_metadata`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMigrationMetadata {
    /// Row id, when the table has one.
    pub id: Option<i64>,
    /// Stored migration hash.
    pub hash: String,
    /// Stored `created_at` value.
    pub created_at: i64,
}

/// A legacy tracking row matched to a local migration name.
///
/// Output of [`match_applied_migration_metadata`]; feed it to
/// [`Migrations::backfill_migration_metadata_sql`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedMigrationMetadata {
    /// Row id, when the table has one.
    pub id: Option<i64>,
    /// Stored migration hash.
    pub hash: String,
    /// Stored `created_at` value.
    pub created_at: i64,
    /// Name (tag) of the local migration this row belongs to.
    pub name: String,
}

impl Migration {
    /// Creates a migration from a tag and the contents of its `migration.sql`.
    ///
    /// The hash is the SHA-256 of `sql`. `created_at` comes from the tag's
    /// `YYYYMMDDHHMMSS` prefix (UTC millis), a legacy `0000` index prefix, or
    /// `0` when the tag has neither.
    ///
    /// When the SQL contains `--> statement-breakpoint` markers it is split
    /// on those markers only, like drizzle-orm: each trimmed chunk is one
    /// statement and runs whole, even if it holds several SQL statements.
    /// Without markers it is split on top-level semicolons; semicolons inside
    /// strings, comments, parentheses, and trigger/function bodies are kept.
    /// The dialect is not known here, so dialect-specific syntax (MySQL
    /// backslash escapes and `#` comments, for example) is resolved when the
    /// migration joins a [`Migrations`] set, or up front with
    /// [`for_dialect`](Self::for_dialect).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use drizzle_migrations::Migration;
    ///
    /// let m = Migration::new(
    ///     "20231220143052_init",
    ///     "CREATE TABLE users (id INTEGER);\n--> statement-breakpoint\nCREATE TABLE posts (id INTEGER);",
    /// );
    /// assert_eq!(m.tag(), "20231220143052_init");
    /// assert_eq!(m.statements(), ["CREATE TABLE users (id INTEGER);", "CREATE TABLE posts (id INTEGER);"]);
    /// assert_eq!(m.created_at(), 1_703_082_652_000);
    /// ```
    #[must_use]
    pub fn new(tag: &str, sql: &str) -> Self {
        Self::from_sql(tag.to_string(), sql, None)
    }

    /// Creates a migration like [`new`](Self::new), splitting SQL that has no
    /// breakpoint markers with `dialect`'s quoting and comment rules.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use drizzle_migrations::Migration;
    /// use drizzle_types::Dialect;
    ///
    /// // MySQL strings take backslash escapes, so the `;` stays quoted.
    /// let m = Migration::for_dialect(
    ///     "20231220143052_seed",
    ///     "INSERT INTO t VALUES ('a\\';b');\nINSERT INTO t VALUES ('c');",
    ///     Dialect::MySQL,
    /// );
    /// assert_eq!(m.statements().len(), 2);
    /// ```
    #[must_use]
    pub fn for_dialect(tag: &str, sql: &str, dialect: Dialect) -> Self {
        Self::from_sql(tag.to_string(), sql, Some(dialect))
    }

    /// Hashes and splits a migration file. Without a dialect, a file with no
    /// breakpoints keeps its text so a [`Migrations`] set can re-split it.
    pub(crate) fn from_sql(tag: String, sql: &str, dialect: Option<Dialect>) -> Self {
        let unsplit = (dialect.is_none() && !sql.contains(STATEMENT_BREAKPOINT))
            .then(|| sql.to_string());
        Self {
            created_at: parse_timestamp_from_tag(&tag),
            tag,
            hash: compute_hash(sql),
            sql: split_statements_for(sql, dialect),
            unsplit,
        }
    }

    /// Re-splits a migration that was split without knowing its dialect.
    fn resplit_for(mut self, dialect: Dialect) -> Self {
        if let Some(sql) = self.unsplit.take() {
            self.sql = split_statements_for(&sql, Some(dialect));
        }
        self
    }

    /// Creates a migration from already-computed parts.
    ///
    /// No hashing or splitting is done: `sql` is used as the statement list.
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
            unsplit: None,
        }
    }

    /// Returns the migration tag (folder name).
    #[inline]
    #[must_use]
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Returns the name stored in the tracking table's `name` column.
    ///
    /// Same value as [`tag`](Self::tag).
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.tag
    }

    /// Returns the SHA-256 hash (hex) of the migration SQL.
    #[inline]
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Returns the `created_at` value derived from the tag (see [`new`](Self::new)).
    #[inline]
    #[must_use]
    pub const fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Returns the SQL statements, already split.
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

    /// Returns `true` when the migration has no non-blank statements.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sql.is_empty() || self.sql.iter().all(|s| s.trim().is_empty())
    }

    /// Returns `true` if any statement is a PostgreSQL
    /// `CREATE/DROP INDEX CONCURRENTLY`, which cannot run in a transaction.
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

/// An ordered set of migrations plus the SQL for their tracking table.
///
/// The tracking table defaults to `__drizzle_migrations` (in schema
/// `drizzle` on PostgreSQL); use [`with_tracking`](Self::with_tracking) to
/// change it.
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

/// Re-splits, with `dialect`'s rules, migrations that were split without
/// knowing it (see [`Migration::new`]).
fn resplit_for(migrations: Vec<Migration>, dialect: Dialect) -> Vec<Migration> {
    migrations
        .into_iter()
        .map(|migration| migration.resplit_for(dialect))
        .collect()
}

impl Migrations {
    /// Creates a set using the default tracking table.
    ///
    /// Migrations built by [`Migration::new`] from SQL without breakpoints
    /// are re-split with `dialect`'s rules.
    #[must_use]
    pub fn new(migrations: Vec<Migration>, dialect: Dialect) -> Self {
        Self {
            list: resplit_for(migrations, dialect),
            dialect,
            table: "__drizzle_migrations".to_string(),
            schema: match dialect {
                Dialect::PostgreSQL => Some("drizzle".to_string()),
                _ => None,
            },
        }
    }

    /// Creates a set that tracks applied migrations in the table (and schema)
    /// named by `tracking`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use drizzle_migrations::{Migrations, Tracking};
    /// use drizzle_types::Dialect;
    ///
    /// let set = Migrations::with_tracking(Vec::new(), Dialect::SQLite, Tracking::SQLITE.table("app_migrations"));
    /// assert_eq!(set.table_name(), "app_migrations");
    /// ```
    pub fn with_tracking(migrations: Vec<Migration>, dialect: Dialect, tracking: Tracking) -> Self {
        Self {
            list: resplit_for(migrations, dialect),
            dialect,
            table: tracking.table.into_owned(),
            schema: tracking.schema.map(std::borrow::Cow::into_owned),
        }
    }

    /// Creates a set with no migrations.
    #[must_use]
    pub fn empty(dialect: Dialect) -> Self {
        Self::new(Vec::new(), dialect)
    }

    /// Returns all migrations, in order.
    #[inline]
    #[must_use]
    pub fn all(&self) -> &[Migration] {
        &self.list
    }

    /// Returns the migrations whose name is not in `applied_names`.
    ///
    /// Mirrors drizzle-orm's beta.19 `getMigrationsToRun`: a local migration is
    /// pending if its `name` (folder name) does not appear in the DB's
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

    /// Returns `true` if [`pending`](Self::pending) would yield anything.
    pub fn has_pending<S>(&self, applied_names: &[S]) -> bool
    where
        S: AsRef<str>,
    {
        self.pending(applied_names).next().is_some()
    }

    /// Returns the dialect.
    #[inline]
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Returns the tracking table name (unquoted).
    #[inline]
    #[must_use]
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Returns the tracking schema name, if any.
    #[inline]
    #[must_use]
    pub fn schema_name(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Returns the quoted tracking table identifier, schema-qualified on
    /// PostgreSQL (for example `"drizzle"."__drizzle_migrations"`).
    #[inline]
    #[must_use]
    pub fn table_ident_sql(&self) -> String {
        self.table_ident()
    }

    /// Returns a stable advisory-lock key for serializing PostgreSQL migration
    /// runners that share this tracking table.
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

    /// Returns the `GET_LOCK` name used to serialize MySQL migration runners.
    ///
    /// `GET_LOCK` names are limited to 64 characters. A SHA-256 prefix keeps
    /// the name stable per database and tracking table without leaking a long
    /// qualified identifier into that limit. MySQL lock names are server-wide,
    /// so including the selected database prevents unrelated databases from
    /// serializing each other's migrations.
    #[must_use]
    pub fn mysql_advisory_lock_name(&self, database: &str) -> String {
        use std::fmt::Write as _;

        let digest = Sha256::digest(
            format!("drizzle-rs:migrate:{database}:{}", self.table_ident()).as_bytes(),
        );
        let mut name = String::from("drizzle-rs:migrate:");
        for byte in &digest[..20] {
            write!(&mut name, "{byte:02x}").expect("writing to String cannot fail");
        }
        name
    }

    /// Returns `true` if any migration must run outside a PostgreSQL
    /// transaction (see [`Migration::has_postgres_concurrent_index`]).
    #[must_use]
    pub fn has_postgres_concurrent_index(&self) -> bool {
        self.list
            .iter()
            .any(Migration::has_postgres_concurrent_index)
    }

    /// Returns SQL for a partial unique index that blocks duplicate non-null
    /// names in the tracking table, or `None` on MySQL (no partial indexes).
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

    /// Quoted table identifier, schema-qualified on PostgreSQL.
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

    /// Returns `CREATE SCHEMA IF NOT EXISTS` for the tracking schema, or
    /// `None` when there is no schema (SQLite, MySQL).
    #[must_use]
    pub fn create_schema_sql(&self) -> Option<String> {
        self.schema.as_ref().map(|schema| {
            format!(
                "CREATE SCHEMA IF NOT EXISTS {};",
                quote_identifier(self.dialect, schema)
            )
        })
    }

    /// Returns `CREATE TABLE IF NOT EXISTS` for the tracking table.
    ///
    /// Columns match current drizzle-orm:
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

    /// Returns the `INSERT` that records `migration` as applied, with
    /// `applied_at = CURRENT_TIMESTAMP`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use drizzle_migrations::{Migration, Migrations};
    /// use drizzle_types::Dialect;
    ///
    /// let m = Migration::new("0000_init", "CREATE TABLE t (id INTEGER);");
    /// let set = Migrations::new(vec![m.clone()], Dialect::SQLite);
    /// let sql = set.record_migration_sql(&m);
    /// assert!(sql.starts_with(r#"INSERT INTO "__drizzle_migrations" ("hash", "created_at", "name", "applied_at")"#));
    /// ```
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

    /// Returns the `INSERT` that records `migration` as *started* (phase 1 of two-phase
    /// tracking on non-transactional paths).
    ///
    /// The row is written with both `applied_at` and `created_at` explicitly
    /// `NULL`, which marks the migration **dirty**: its statements are about
    /// to run but have not been confirmed. The `NULL` `created_at` is what
    /// identifies the marker: drizzle-orm always writes `created_at`, but its
    /// own v0 → v1 tracking upgrade leaves `applied_at` `NULL` on every row it
    /// upgrades, so `applied_at IS NULL` alone cannot tell an interrupted
    /// drizzle-rs run from a database migrated by drizzle-orm.
    /// [`Migrations::record_migration_finished_sql`] clears the
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

        match self.dialect {
            Dialect::SQLite | Dialect::PostgreSQL => {
                format!(
                    r#"INSERT INTO {table} ("hash", "created_at", "name", "applied_at") VALUES ('{hash}', NULL, '{name}', NULL);"#
                )
            }
            Dialect::MySQL => {
                format!(
                    r"INSERT INTO {table} (`hash`, `created_at`, `name`, `applied_at`) VALUES ('{hash}', NULL, '{name}', NULL);"
                )
            }
        }
    }

    /// Returns the `UPDATE` that marks a started migration as finished
    /// (phase 3 of two-phase tracking; phase 2 runs the statements).
    ///
    /// Sets `applied_at` and fills in `created_at`. Only touches rows that are
    /// still dirty, so a concurrent runner that already completed the
    /// migration is not re-stamped.
    #[must_use]
    pub fn record_migration_finished_sql(&self, migration: &Migration) -> String {
        let table = self.table_ident();
        let name = escape_sql_string(migration.name());
        let created_at = migration.created_at();
        let dirty = self.dirty_predicate();

        match self.dialect {
            Dialect::MySQL => format!(
                r"UPDATE {table} SET `applied_at` = CURRENT_TIMESTAMP, `created_at` = {created_at} WHERE `name` = '{name}' AND {dirty};"
            ),
            _ => format!(
                r#"UPDATE {table} SET "applied_at" = CURRENT_TIMESTAMP, "created_at" = {created_at} WHERE "name" = '{name}' AND {dirty};"#
            ),
        }
    }

    /// SQL predicate matching a two-phase dirty marker: `applied_at` and
    /// `created_at` both `NULL` (see
    /// [`record_migration_started_sql`](Self::record_migration_started_sql)).
    const fn dirty_predicate(&self) -> &'static str {
        match self.dialect {
            Dialect::MySQL => "(`applied_at` IS NULL AND `created_at` IS NULL)",
            Dialect::SQLite | Dialect::PostgreSQL => {
                r#"("applied_at" IS NULL AND "created_at" IS NULL)"#
            }
        }
    }

    /// Returns the `DELETE` that drops a migration's dirty marker.
    ///
    /// Used when a non-transactional run fails on its *first* statement, where
    /// nothing can have been applied and leaving a dirty row would demand a
    /// pointless repair. Never touches a completed row.
    #[must_use]
    pub fn clear_migration_started_sql(&self, migration: &Migration) -> String {
        let table = self.table_ident();
        let name = escape_sql_string(migration.name());
        let dirty = self.dirty_predicate();

        match self.dialect {
            Dialect::MySQL => {
                format!(r"DELETE FROM {table} WHERE `name` = '{name}' AND {dirty};")
            }
            _ => {
                format!(r#"DELETE FROM {table} WHERE "name" = '{name}' AND {dirty};"#)
            }
        }
    }

    /// Returns the `UPDATE` that backfills `name`/`applied_at` on a legacy
    /// tracking row.
    ///
    /// The v0 tracking table had only `id`/`hash`/`created_at`; the upgrade
    /// adds `name` and `applied_at` and backfills both. `applied_at` is derived
    /// from the row's `created_at` rather than left `NULL`, so the row reads
    /// as applied without relying on the `created_at` sentinel.
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

    /// Returns the `SELECT` that loads applied migration names.
    ///
    /// A row counts as applied when it has a non-null `name` and is not a
    /// two-phase dirty marker:
    ///
    /// * `name IS NULL` — written before the v0 → v1 tracking-table upgrade
    ///   (which backfills `name`), so it cannot be matched to a local
    ///   migration.
    /// * `applied_at IS NULL AND created_at IS NULL` — a **dirty marker**: the
    ///   migration started but was never confirmed complete. Reporting it as
    ///   applied would silently skip a half-applied migration. See
    ///   [`Migrations::dirty_names_sql`].
    ///
    /// A named row with a `NULL` `applied_at` but a `created_at` is applied:
    /// drizzle-orm's tracking-table upgrade writes exactly such rows, and
    /// drizzle-orm itself treats every named row as applied.
    ///
    /// Pair with [`Migrations::pending`].
    #[must_use]
    pub fn applied_names_sql(&self) -> String {
        let table = self.table_ident();
        let dirty = self.dirty_predicate();
        match self.dialect {
            Dialect::MySQL => {
                format!(
                    "SELECT `name` FROM {table} WHERE `name` IS NOT NULL AND NOT {dirty} ORDER BY id;"
                )
            }
            _ => format!(
                r#"SELECT "name" FROM {table} WHERE "name" IS NOT NULL AND NOT {dirty} ORDER BY id;"#
            ),
        }
    }

    /// Returns the `SELECT` that loads `hash`, `name`, and a `dirty` flag
    /// (`applied_at` and `created_at` both `NULL`: started but never
    /// finished) for every named row.
    ///
    /// Unlike [`Migrations::applied_names_sql`] this returns interrupted rows
    /// too, so integrity checks can report drift, missing-local, and
    /// interrupted migrations from a single query.
    #[must_use]
    pub fn applied_records_sql(&self) -> String {
        let table = self.table_ident();
        let dirty = self.dirty_predicate();
        match self.dialect {
            Dialect::MySQL => {
                format!(
                    "SELECT `hash`, `name`, {dirty} AS dirty FROM {table} WHERE `name` IS NOT NULL ORDER BY id;"
                )
            }
            _ => format!(
                r#"SELECT "hash", "name", {dirty} AS dirty FROM {table} WHERE "name" IS NOT NULL ORDER BY id;"#
            ),
        }
    }

    /// Returns the `SELECT` that loads interrupted ("dirty") migration names.
    ///
    /// These are named rows whose `applied_at` and `created_at` are both
    /// `NULL` — a migration that started on a non-transactional path and never
    /// reported completion. Rows where only `applied_at` is `NULL` come from
    /// drizzle-orm's tracking-table upgrade and count as applied.
    #[must_use]
    pub fn dirty_names_sql(&self) -> String {
        let table = self.table_ident();
        let dirty = self.dirty_predicate();
        match self.dialect {
            Dialect::MySQL => {
                format!(
                    "SELECT `name` FROM {table} WHERE `name` IS NOT NULL AND {dirty} ORDER BY id;"
                )
            }
            _ => format!(
                r#"SELECT "name" FROM {table} WHERE "name" IS NOT NULL AND {dirty} ORDER BY id;"#
            ),
        }
    }

    /// Builds the standard [`MigratorError::InterruptedMigration`] for
    /// `dirty_names`, or `None` when it is empty.
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
        let (name_column, applied_at_column) = match self.dialect {
            Dialect::MySQL => ("`name`", "`applied_at`"),
            Dialect::SQLite | Dialect::PostgreSQL => ("\"name\"", "\"applied_at\""),
        };

        let recovery = if self.dialect == Dialect::MySQL {
            format!(
                "Recovery options:\n  \
                 1. inspect the partially applied DDL and reconcile the schema by hand\n  \
                 2. after reconciliation, either complete the row \
                 (UPDATE {table} SET {applied_at_column} = CURRENT_TIMESTAMP WHERE {name_column} = '{first}';) \
                 or discard it and re-run from scratch \
                 (DELETE FROM {table} WHERE {name_column} = '{first}';)"
            )
        } else {
            format!(
                "Recovery options:\n  \
                 1. re-run with repair enabled (`drizzle migrate --repair`, or `migrate_with_repair` \
                 on the driver) to reconcile each remaining statement against the live schema\n  \
                 2. resolve the partial state by hand, then either complete the row \
                 (UPDATE {table} SET {applied_at_column} = CURRENT_TIMESTAMP WHERE {name_column} = '{first}';) \
                 or discard it and re-run from scratch \
                 (DELETE FROM {table} WHERE {name_column} = '{first}';)"
            )
        };

        Some(MigratorError::InterruptedMigration(format!(
            "migration{plural} {names} {} interrupted mid-apply: the tracking row in {table} has \
             NULL `applied_at` and `created_at`, so an earlier run recorded the migration as \
             started but never recorded it as finished. The database may be in a partially-migrated state, and \
             re-running the migration as-is would fail (for example with `table already exists`).\n\
             {recovery}",
            if dirty_names.len() == 1 {
                "was"
            } else {
                "were"
            },
        )))
    }

    /// Maps dirty tracking-row names to their local migrations, in local order.
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

    /// Returns a `SELECT` that yields one row if the tracking table exists.
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
                "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name='{table}';"
            ),
        }
    }
}

/// Errors from loading, tracking, or running migrations.
#[derive(Debug, thiserror::Error)]
pub enum MigratorError {
    /// A legacy drizzle-kit `meta/_journal.json` layout was found; run
    /// `drizzle up` to convert it.
    #[error("Journal error: {0}")]
    JournalError(String),

    /// Reading the migrations folder failed.
    #[error("IO error: {0}")]
    IoError(String),

    /// A migration folder has `snapshot.json` but no `migration.sql`.
    #[error("Missing migration file: {0}")]
    MissingMigration(String),

    /// A statement failed, or tracking rows could not be matched to local
    /// migrations.
    #[error("Migration failed: {0}")]
    ExecutionError(String),

    /// A tracking row exists with `applied_at` NULL: the migration started but
    /// never reported completion. Produced by
    /// [`Migrations::interrupted_migration_error`].
    #[error("{0}")]
    InterruptedMigration(String),

    /// Repair could not reconcile an interrupted migration. Produced by
    /// [`crate::repair::Plan::into_executable`] and
    /// [`Migrations::resolve_dirty_migrations`].
    #[error("{0}")]
    UnrepairableMigration(String),
}

/// Returns `true` if `sql` starts with PostgreSQL `CREATE [UNIQUE] INDEX
/// CONCURRENTLY` or `DROP INDEX CONCURRENTLY`.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::is_postgres_concurrent_index_statement;
///
/// assert!(is_postgres_concurrent_index_statement("CREATE INDEX CONCURRENTLY idx ON t (a);"));
/// assert!(!is_postgres_concurrent_index_statement("CREATE INDEX idx ON t (a);"));
/// ```
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

/// SHA-256 of the SQL content, as lowercase hex.
pub(crate) fn compute_hash(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }

    out
}

/// The marker drizzle-kit writes between generated statements.
const STATEMENT_BREAKPOINT: &str = "--> statement-breakpoint";

/// Splits SQL content into individual statements without knowing the
/// dialect. See [`split_statements_for`].
#[cfg(test)]
pub(crate) fn split_statements(sql: &str) -> Vec<String> {
    split_statements_for(sql, None)
}

/// Splits a migration file into the statements a runner executes.
///
/// A file that contains `--> statement-breakpoint` is split on those markers
/// only, exactly like drizzle-orm's migrator: each chunk is trimmed, empty
/// chunks are dropped, and a chunk is never split further (runners execute
/// it whole). A file without markers (hand-written SQL, or generated with
/// breakpoints off) is split on top-level semicolons using `dialect`'s
/// quoting and comment rules; `None` uses rules that are safe across
/// dialects.
pub(crate) fn split_statements_for(sql: &str, dialect: Option<Dialect>) -> Vec<String> {
    if sql.contains(STATEMENT_BREAKPOINT) {
        return sql
            .split(STATEMENT_BREAKPOINT)
            .map(str::trim)
            .filter(|chunk| !chunk.is_empty())
            .map(str::to_string)
            .collect();
    }
    split_on_semicolons(sql, SplitRules::for_dialect(dialect))
}

/// Lexical rules the semicolon splitter applies for one dialect.
#[derive(Clone, Copy, Debug)]
struct SplitRules {
    /// `'...'` and `"..."` strings take backslash escapes (MySQL).
    backslash_strings: bool,
    /// `E'...'` strings take backslash escapes (PostgreSQL).
    escape_string_prefix: bool,
    /// `$tag$ ... $tag$` quoting (PostgreSQL).
    dollar_quotes: bool,
    /// `[identifier]` quoting (SQLite).
    bracket_identifiers: bool,
    /// `# ...` line comments (MySQL).
    hash_comments: bool,
    /// `--` starts a comment only when followed by whitespace (MySQL).
    dash_comment_needs_space: bool,
    /// `/* /* */ */` comments nest (PostgreSQL).
    nested_block_comments: bool,
    /// Any `BEGIN` inside a compound body opens a nested block, including
    /// `DECLARE ... HANDLER FOR ... BEGIN` (MySQL stored programs, where
    /// `BEGIN` is never a transaction statement).
    begin_anywhere_nests: bool,
}

impl SplitRules {
    const fn for_dialect(dialect: Option<Dialect>) -> Self {
        match dialect {
            Some(Dialect::MySQL) => Self {
                backslash_strings: true,
                escape_string_prefix: false,
                dollar_quotes: false,
                bracket_identifiers: false,
                hash_comments: true,
                dash_comment_needs_space: true,
                nested_block_comments: false,
                begin_anywhere_nests: true,
            },
            Some(Dialect::PostgreSQL) => Self {
                backslash_strings: false,
                escape_string_prefix: true,
                dollar_quotes: true,
                bracket_identifiers: false,
                hash_comments: false,
                dash_comment_needs_space: false,
                nested_block_comments: true,
                begin_anywhere_nests: false,
            },
            Some(Dialect::SQLite) => Self {
                backslash_strings: false,
                escape_string_prefix: false,
                dollar_quotes: false,
                bracket_identifiers: true,
                hash_comments: false,
                dash_comment_needs_space: false,
                nested_block_comments: false,
                begin_anywhere_nests: false,
            },
            // Unknown dialect: only rules that cannot misread another
            // dialect's valid SQL. `E'...'` and `$tag$` only parse in
            // PostgreSQL; nested comments are a superset for files that never
            // nest them.
            None => Self {
                backslash_strings: false,
                escape_string_prefix: true,
                dollar_quotes: true,
                bracket_identifiers: false,
                hash_comments: false,
                dash_comment_needs_space: false,
                nested_block_comments: true,
                begin_anywhere_nests: false,
            },
        }
    }
}

/// Per-statement token context for [`split_on_semicolons`].
///
/// Tracks whether the statement being accumulated is a compound-bodied
/// object (`CREATE TRIGGER|PROCEDURE|FUNCTION|EVENT ... BEGIN ...; END` or a
/// PostgreSQL `BEGIN ATOMIC ...; END` body) so its internal semicolons are
/// not treated as statement boundaries. Mirrors SQLite's
/// `sqlite3_complete()`: a compound body terminates only at an `END` token
/// that directly follows a body semicolon (which keeps `CASE ... END` inside
/// the body inert), itself followed by a semicolon. `END IF` / `END LOOP` /
/// `END WHILE` / `END REPEAT` / `END CASE` close MySQL control blocks, not
/// bodies; `END label` closes a labeled body.
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
    /// Parenthesis nesting; semicolons inside parentheses (PostgreSQL
    /// `CREATE RULE ... DO ALSO (a; b)`) are not boundaries.
    paren_depth: usize,
}

impl StatementState {
    /// Kinds of `CREATE` statements that may contain compound bodies.
    const COMPOUND_KINDS: [&'static str; 4] = ["trigger", "procedure", "function", "event"];
    /// `CREATE <kind>` statements that never do (guards against objects
    /// merely *named* `function` etc.).
    const PLAIN_KINDS: [&'static str; 5] = ["table", "index", "view", "schema", "virtual"];
    /// Words that follow `END` when it closes a MySQL control block rather
    /// than a `BEGIN ... END` body.
    const END_CONTROL: [&'static str; 5] = ["if", "loop", "while", "repeat", "case"];

    /// Record significant (non-whitespace, non-comment) content that is not
    /// an identifier token.
    fn note_significant(&mut self) {
        self.pending_begin = false;
        self.pending_end = false;
        self.at_body_start = false;
    }

    /// Process an identifier token encountered in normal state.
    fn note_token(&mut self, token: &str, rules: SplitRules) {
        let lower = token.to_ascii_lowercase();
        let was_pending_begin = self.pending_begin;
        let was_pending_end = self.pending_end;
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
            "begin"
                if self.compound_depth > 0 && (was_at_body_start || rules.begin_anywhere_nests) =>
            {
                self.compound_depth += 1;
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
            control if was_pending_end && Self::END_CONTROL.contains(&control) => {}
            // `END label;` still closes the (labeled) body.
            _ if was_pending_end => self.pending_end = true,
            _ => {}
        }
        self.last_char_wordy = true;
    }
}

/// `true` for characters that continue an identifier (`$` included, as in
/// PostgreSQL and MySQL identifiers such as `a$b`).
const fn is_word_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '$'
}

/// Returns the byte offset just past a quoted run that starts at `pos`
/// (which holds the opening `quote`), or the end of `sql` when it is
/// unterminated. A doubled closing quote is an escaped quote; with
/// `backslash`, `\x` escapes `x`.
fn skip_quoted(sql: &str, pos: usize, close: char, backslash: bool) -> usize {
    let mut chars = sql[pos..].char_indices().skip(1).peekable();
    while let Some((offset, character)) = chars.next() {
        if backslash && character == '\\' {
            chars.next();
            continue;
        }
        if character == close {
            if close != ']' && chars.peek().is_some_and(|&(_, next)| next == close) {
                chars.next();
                continue;
            }
            return pos + offset + close.len_utf8();
        }
    }
    sql.len()
}

/// Returns the byte offset just past a block comment starting at `pos`.
fn skip_block_comment(sql: &str, pos: usize, nested: bool) -> usize {
    let bytes = sql.as_bytes();
    let mut depth = 0usize;
    let mut index = pos;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'*' && (nested || depth == 0) {
            depth += 1;
            index += 2;
        } else if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += 1;
        }
    }
    sql.len()
}

/// Parse a starting `PostgreSQL` dollar-quote delimiter at `pos`.
///
/// Returns the full delimiter (`$$` or `$tag$`, where the tag starts with a
/// letter or `_`), or `None` for anything else such as a `$1` parameter.
fn parse_dollar_tag_start(sql: &str, pos: usize) -> Option<&str> {
    let rest = sql.get(pos..)?.strip_prefix('$')?;
    for (offset, character) in rest.char_indices() {
        if character == '$' {
            // `$` at `pos`, the tag, then the closing `$` at `pos + 1 + offset`.
            return Some(&sql[pos..pos + offset + 2]);
        }
        let valid = if offset == 0 {
            character.is_ascii_alphabetic() || character == '_'
        } else {
            character.is_ascii_alphanumeric() || character == '_'
        };
        if !valid {
            return None;
        }
    }
    None
}

/// Split SQL without breakpoints on top-level semicolons.
///
/// Quotes, comments, dollar-quoted bodies, parenthesized groups, and compound
/// statement bodies (trigger/procedure/function bodies, `BEGIN ATOMIC`) keep
/// their internal semicolons; `rules` selects the dialect's quoting and
/// comment syntax.
fn split_on_semicolons(sql: &str, rules: SplitRules) -> Vec<String> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut pos = 0;
    let mut state = StatementState::default();

    let push = |statements: &mut Vec<String>, text: &str| {
        let text = text.trim();
        if !text.is_empty() {
            statements.push(text.to_string());
        }
    };

    while pos < sql.len() {
        let rest = &sql[pos..];
        let Some(character) = rest.chars().next() else {
            break;
        };

        // Comments separate tokens but are otherwise ignored.
        let dash_comment = rest.starts_with("--")
            && (!rules.dash_comment_needs_space
                || rest[2..]
                    .chars()
                    .next()
                    .is_none_or(|next| next.is_whitespace() || next.is_control()));
        if dash_comment || (rules.hash_comments && character == '#') {
            pos += rest.find('\n').map_or(rest.len(), |index| index + 1);
            state.last_char_wordy = false;
            continue;
        }
        if rest.starts_with("/*") {
            pos = skip_block_comment(sql, pos, rules.nested_block_comments);
            state.last_char_wordy = false;
            continue;
        }

        // Quoted strings and identifiers.
        let quote = match character {
            '\'' => {
                let escaped = rules.backslash_strings
                    || (rules.escape_string_prefix
                        && pos > 0
                        && matches!(sql.as_bytes()[pos - 1], b'e' | b'E')
                        && sql[..pos - 1].chars().next_back().is_none_or(|c| !is_word_char(c)));
                Some(('\'', escaped))
            }
            '"' => Some(('"', rules.backslash_strings)),
            '`' => Some(('`', false)),
            '[' if rules.bracket_identifiers => Some((']', false)),
            _ => None,
        };
        if let Some((close, backslash)) = quote {
            pos = skip_quoted(sql, pos, close, backslash);
            state.note_significant();
            state.last_char_wordy = true;
            continue;
        }

        if rules.dollar_quotes
            && character == '$'
            && !state.last_char_wordy
            && let Some(tag) = parse_dollar_tag_start(sql, pos)
        {
            let body = pos + tag.len();
            pos = sql[body..]
                .find(tag)
                .map_or(sql.len(), |index| body + index + tag.len());
            state.note_significant();
            state.last_char_wordy = true;
            continue;
        }

        if character == ';' {
            if state.paren_depth > 0 {
                pos += 1;
                state.note_significant();
                state.last_char_wordy = false;
                continue;
            }
            if state.compound_depth > 0 {
                if state.pending_end {
                    state.pending_end = false;
                    state.compound_depth -= 1;
                }
                if state.compound_depth > 0 {
                    pos += 1;
                    state.at_body_start = true;
                    state.pending_begin = false;
                    state.last_char_wordy = false;
                    continue;
                }
                // Depth reached zero: this semicolon closes the compound
                // statement.
            }
            push(&mut statements, &sql[start..pos]);
            pos += 1;
            start = pos;
            state = StatementState::default();
            continue;
        }

        if !state.last_char_wordy && (character.is_ascii_alphabetic() || character == '_') {
            let token_len = rest
                .find(|c: char| !is_word_char(c))
                .unwrap_or(rest.len());
            state.note_token(&rest[..token_len], rules);
            pos += token_len;
            continue;
        }

        match character {
            '(' => state.paren_depth += 1,
            ')' => state.paren_depth = state.paren_depth.saturating_sub(1),
            _ => {}
        }
        if !character.is_whitespace() {
            state.note_significant();
        }
        state.last_char_wordy = is_word_char(character);
        pos += character.len_utf8();
    }

    push(&mut statements, &sql[start..]);
    statements
}

/// Matches legacy tracking rows (no `name` column) to local migrations.
///
/// Used when upgrading an old tracking table. Rows are matched by
/// `created_at` truncated to whole seconds (as drizzle-orm does: legacy rows
/// store the journal's millisecond timestamp, folder names only seconds),
/// falling back to `hash` when that is missing or ambiguous.
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
        // drizzle-orm's upgrade drops the last three digits of the stored
        // value (`substring(0, len - 3) + '000'`): legacy rows hold the
        // journal's millisecond `when`, while folder names only carry
        // seconds. Integer division truncates toward zero exactly like that
        // string slice does.
        let truncated = (row.created_at / 1000) * 1000;
        let candidates = by_created_at
            .get(&truncated)
            .or_else(|| by_created_at.get(&row.created_at));
        let migration = match candidates {
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

/// Parses `created_at` from a migration tag.
///
/// `YYYYMMDDHHMMSS_name` gives UTC millis, legacy `0000_name` gives the
/// index, and anything else gives `0`.
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

/// Builds a `Vec<Migration>` from `(tag, sql)` pairs.
///
/// Each pair becomes [`Migration::new`]`(tag, sql)`. Pair it with
/// `include_str!` to embed SQL files.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{Migration, migrations};
///
/// let list: Vec<Migration> = migrations![
///     ("20231220143052_init", "CREATE TABLE users (id INTEGER);"),
///     // ("20231221093015_posts", include_str!("../drizzle/20231221093015_posts/migration.sql")),
/// ];
/// assert_eq!(list[0].tag(), "20231220143052_init");
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
        parse_timestamp_from_tag, split_statements, split_statements_for,
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
            vec!["CREATE TABLE records (id INTEGER);"]
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
            vec!["CREATE TABLE records (id INTEGER);"]
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

        let stmts = split_statements(sql);
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

        let stmts = split_statements(sql);
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

        let stmts = split_statements(sql);
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
            stmts[1].ends_with("END;"),
            "trigger body truncated: {}",
            stmts[1]
        );
    }

    #[test]
    fn breakpoint_files_split_only_on_markers_like_drizzle_orm() {
        // drizzle-orm: `query.split('--> statement-breakpoint')`, each chunk
        // run whole. Semicolons inside a chunk never split it.
        let sql = "CREATE TABLE a (id int);\nCREATE TABLE b (id int);\n--> statement-breakpoint\n\
                   CREATE PROCEDURE p()\nBEGIN\n  DECLARE x INT;\n  BEGIN\n    SET x = 1;\n  END;\nEND;";
        assert_eq!(
            split_statements(sql),
            [
                "CREATE TABLE a (id int);\nCREATE TABLE b (id int);",
                "CREATE PROCEDURE p()\nBEGIN\n  DECLARE x INT;\n  BEGIN\n    SET x = 1;\n  END;\nEND;",
            ]
        );

        // Same line, CRLF, no semicolons, and empty chunks.
        assert_eq!(
            split_statements("CREATE TABLE a (id int);--> statement-breakpoint\nCREATE TABLE b (id int)"),
            ["CREATE TABLE a (id int);", "CREATE TABLE b (id int)"]
        );
        assert_eq!(
            split_statements(
                "CREATE TABLE a (id int);\r\n--> statement-breakpoint\r\n\r\n--> statement-breakpoint\r\nCREATE TABLE b (id int);\r\n"
            ),
            ["CREATE TABLE a (id int);", "CREATE TABLE b (id int);"]
        );

        // Every dialect splits a breakpoint file the same way.
        for dialect in [Dialect::SQLite, Dialect::PostgreSQL, Dialect::MySQL] {
            assert_eq!(split_statements_for(sql, Some(dialect)), split_statements(sql));
        }
    }

    #[test]
    fn mysql_split_handles_backslashes_comments_and_nested_blocks() {
        let mysql = |sql: &str| split_statements_for(sql, Some(Dialect::MySQL));

        assert_eq!(
            mysql("INSERT INTO t VALUES ('a\\';b');\nINSERT INTO t VALUES (\"c\\\";d\");"),
            ["INSERT INTO t VALUES ('a\\';b')", "INSERT INTO t VALUES (\"c\\\";d\")"]
        );
        assert_eq!(
            mysql("# it's a comment; really\nCREATE TABLE a (id int);\nCREATE TABLE b (id int);"),
            ["# it's a comment; really\nCREATE TABLE a (id int)", "CREATE TABLE b (id int)"]
        );
        // `--` needs trailing whitespace to start a MySQL comment.
        assert_eq!(
            mysql("UPDATE t SET a = a--1;\nUPDATE t SET b = 1;-- trailing; comment\n"),
            ["UPDATE t SET a = a--1", "UPDATE t SET b = 1", "-- trailing; comment"]
        );
        assert_eq!(
            mysql("CREATE TABLE `it's` (id int);\nCREATE TABLE `a;b` (id int);"),
            ["CREATE TABLE `it's` (id int)", "CREATE TABLE `a;b` (id int)"]
        );

        let nested = "CREATE PROCEDURE p()\nBEGIN\n  DECLARE x INT;\n  BEGIN\n    SET x = 1;\n  END;\n  \
                      IF x = 1 THEN SELECT 1; END IF;\n  SELECT x;\nEND;\nCREATE TABLE after_proc (id int);";
        let statements = mysql(nested);
        assert_eq!(statements.len(), 2, "{statements:?}");
        assert!(statements[0].ends_with("SELECT x;\nEND"), "{statements:?}");
        assert_eq!(statements[1], "CREATE TABLE after_proc (id int)");

        let handler = "CREATE PROCEDURE p()\nBEGIN\n  DECLARE CONTINUE HANDLER FOR SQLEXCEPTION\n  BEGIN\n    \
                       SET @err = 1;\n  END;\n  INSERT INTO t VALUES (1);\nEND;\nSELECT 2;";
        let statements = mysql(handler);
        assert_eq!(statements.len(), 2, "{statements:?}");
        assert_eq!(statements[1], "SELECT 2");

        let labeled = "CREATE PROCEDURE p()\nouter_block: BEGIN\n  inner_block: BEGIN\n    SELECT 1;\n  \
                       END inner_block;\n  SELECT 2;\nEND outer_block;\nSELECT 3;";
        let statements = mysql(labeled);
        assert_eq!(statements.len(), 2, "{statements:?}");
        assert_eq!(statements[1], "SELECT 3");
    }

    #[test]
    fn postgres_split_handles_escape_strings_rules_and_dollar_identifiers() {
        let postgres = |sql: &str| split_statements_for(sql, Some(Dialect::PostgreSQL));

        assert_eq!(
            postgres("INSERT INTO t VALUES (E'it\\'s; here');\nSELECT 1;"),
            ["INSERT INTO t VALUES (E'it\\'s; here')", "SELECT 1"]
        );
        // A standard string keeps its backslash literal: 'a\' ends there.
        assert_eq!(
            postgres("SELECT 'a\\';\nSELECT 2;"),
            ["SELECT 'a\\'", "SELECT 2"]
        );
        assert_eq!(
            postgres(
                "CREATE RULE r AS ON INSERT TO t DO ALSO (INSERT INTO log VALUES (1); INSERT INTO log VALUES (2));\nSELECT 1;"
            ),
            [
                "CREATE RULE r AS ON INSERT TO t DO ALSO (INSERT INTO log VALUES (1); INSERT INTO log VALUES (2))",
                "SELECT 1"
            ]
        );
        assert_eq!(
            postgres("SELECT a$b$c FROM t; SELECT 'x';"),
            ["SELECT a$b$c FROM t", "SELECT 'x'"]
        );
        assert_eq!(
            postgres("PREPARE p AS SELECT $1; SELECT 2;"),
            ["PREPARE p AS SELECT $1", "SELECT 2"]
        );
        assert_eq!(
            postgres("/* outer /* inner */ still ; comment */ SELECT 1; SELECT 2;"),
            ["/* outer /* inner */ still ; comment */ SELECT 1", "SELECT 2"]
        );
        assert_eq!(
            postgres("CREATE INDEX i ON event (begin);\nCREATE TABLE z (id int);"),
            ["CREATE INDEX i ON event (begin)", "CREATE TABLE z (id int)"]
        );
    }

    #[test]
    fn sqlite_split_handles_brackets_and_backticks() {
        let sqlite = |sql: &str| split_statements_for(sql, Some(Dialect::SQLite));
        assert_eq!(
            sqlite("CREATE TABLE [a;b] (id int);\nCREATE TABLE `it's` (id int);\nSELECT 'x\\';"),
            ["CREATE TABLE [a;b] (id int)", "CREATE TABLE `it's` (id int)", "SELECT 'x\\'"]
        );
    }

    #[test]
    fn migrations_set_resplits_with_its_dialect() {
        let sql = "INSERT INTO t VALUES ('a\\';b');\nINSERT INTO t VALUES ('c');";
        let migration = Migration::new("20240101000000_seed", sql);
        let set = Migrations::new(vec![migration.clone()], Dialect::MySQL);
        assert_eq!(
            set.all()[0].statements(),
            ["INSERT INTO t VALUES ('a\\';b')", "INSERT INTO t VALUES ('c')"]
        );
        // The hash always covers the original file.
        assert_eq!(set.all()[0].hash(), migration.hash());
        assert_eq!(
            Migration::for_dialect("20240101000000_seed", sql, Dialect::MySQL).statements(),
            set.all()[0].statements()
        );
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
    fn mysql_advisory_lock_name_is_stable_and_within_server_limit() {
        let first = Migrations::with_tracking(
            Vec::new(),
            Dialect::MySQL,
            Tracking::new("migrations", None::<String>),
        );
        let different = Migrations::with_tracking(
            Vec::new(),
            Dialect::MySQL,
            Tracking::new("other_migrations", None::<String>),
        );
        assert_eq!(
            first.mysql_advisory_lock_name("app"),
            first.mysql_advisory_lock_name("app")
        );
        assert_ne!(
            first.mysql_advisory_lock_name("app"),
            different.mysql_advisory_lock_name("app")
        );
        assert_ne!(
            first.mysql_advisory_lock_name("app"),
            first.mysql_advisory_lock_name("other_app")
        );
        assert!(first.mysql_advisory_lock_name("app").len() <= 64);
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
        assert!(sql.contains(r#"("applied_at" IS NULL AND "created_at" IS NULL) AS dirty"#));
        // Unlike applied_names_sql, dirty rows are included so integrity
        // checks can report them.
        assert!(!sql.contains("NOT ("));

        let mysql = Migrations::new(Vec::new(), Dialect::MySQL);
        let sql = mysql.applied_records_sql();
        assert!(sql.contains("`hash`"));
        assert!(sql.contains("(`applied_at` IS NULL AND `created_at` IS NULL) AS dirty"));
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
                let marker = "(`applied_at` IS NULL AND `created_at` IS NULL)";
                assert!(applied.contains(&format!("NOT {marker}")), "{applied}");
                assert!(dirty.contains(&format!("AND {marker}")), "{dirty}");
                assert!(dirty.contains("`name` IS NOT NULL"), "{dirty}");
            } else {
                let marker = r#"("applied_at" IS NULL AND "created_at" IS NULL)"#;
                assert!(applied.contains(&format!("NOT {marker}")), "{applied}");
                assert!(dirty.contains(&format!("AND {marker}")), "{dirty}");
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
            started.contains("('abc123', NULL, '20230331141203_test', NULL)"),
            "phase 1 must write created_at and applied_at NULL explicitly: {started}"
        );

        let finished = set.record_migration_finished_sql(&migration);
        assert!(finished.starts_with("UPDATE"));
        assert!(finished.contains("\"applied_at\" = CURRENT_TIMESTAMP"));
        assert!(finished.contains("\"created_at\" = 1680271923000"), "{finished}");
        assert!(
            finished.contains(r#"("applied_at" IS NULL AND "created_at" IS NULL)"#),
            "phase 3 must only clear a still-dirty row: {finished}"
        );

        let cleared = set.clear_migration_started_sql(&migration);
        assert!(cleared.starts_with("DELETE FROM"));
        assert!(cleared.contains(r#"("applied_at" IS NULL AND "created_at" IS NULL)"#));
    }

    /// Runs `sql` against an in-memory SQLite tracking table and returns the
    /// names it yields.
    fn sqlite_names(conn: &rusqlite::Connection, sql: &str) -> Vec<String> {
        let mut statement = conn.prepare(sql).expect("prepare");
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows")
    }

    #[test]
    fn upstream_upgraded_rows_are_applied_but_drizzle_rs_markers_are_dirty() {
        let upgraded = Migration::new("20240101000000_init", "CREATE TABLE a (id INTEGER);");
        let interrupted = Migration::new("20240102000000_next", "CREATE TABLE b (id INTEGER);");
        let set = Migrations::new(vec![upgraded.clone(), interrupted.clone()], Dialect::SQLite);
        let conn = rusqlite::Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(&set.create_table_sql()).expect("tracking table");

        // drizzle-orm's v0 -> v1 upgrade (up-migrations/sqlite.ts) backfills
        // `name` and writes `applied_at = NULL` on every pre-existing row.
        conn.execute_batch(&format!(
            "INSERT INTO \"__drizzle_migrations\" (hash, created_at) VALUES ('{}', 1704067200123);
             UPDATE \"__drizzle_migrations\" SET name = '20240101000000_init', applied_at = NULL;",
            upgraded.hash()
        ))
        .expect("upstream upgrade state");
        // drizzle-rs's two-phase marker for an interrupted run.
        conn.execute_batch(&set.record_migration_started_sql(&interrupted))
            .expect("started marker");

        assert_eq!(
            sqlite_names(&conn, &set.applied_names_sql()),
            vec!["20240101000000_init".to_string()]
        );
        assert_eq!(
            sqlite_names(&conn, &set.dirty_names_sql()),
            vec!["20240102000000_next".to_string()]
        );
        let dirty_flags: Vec<(String, bool)> = conn
            .prepare(&set.applied_records_sql())
            .expect("prepare")
            .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(
            dirty_flags,
            vec![
                ("20240101000000_init".to_string(), false),
                ("20240102000000_next".to_string(), true),
            ]
        );

        // Finishing the marker stamps applied_at and created_at; it then
        // reads as applied, and the upstream row is never touched.
        conn.execute_batch(&set.record_migration_finished_sql(&interrupted))
            .expect("finish");
        assert!(sqlite_names(&conn, &set.dirty_names_sql()).is_empty());
        assert_eq!(sqlite_names(&conn, &set.applied_names_sql()).len(), 2);
        let (created_at, upstream_applied_at): (i64, Option<String>) = conn
            .query_row(
                "SELECT (SELECT created_at FROM \"__drizzle_migrations\" WHERE name = '20240102000000_next'),
                        (SELECT applied_at FROM \"__drizzle_migrations\" WHERE name = '20240101000000_init')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("row");
        assert_eq!(created_at, interrupted.created_at());
        assert_eq!(upstream_applied_at, None);

        // Clearing a marker never deletes the upstream-upgraded row.
        conn.execute_batch(&set.clear_migration_started_sql(&upgraded))
            .expect("clear");
        assert_eq!(sqlite_names(&conn, &set.applied_names_sql()).len(), 2);
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
        assert!(started.contains("NULL, '20230331141203_test', NULL)"), "{started}");
        let finished = mysql.record_migration_finished_sql(&migration);
        assert!(finished.contains("`applied_at` = CURRENT_TIMESTAMP"), "{finished}");
        assert!(finished.contains("`created_at` = 1680271923000"), "{finished}");
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
    fn mysql_interrupted_migration_requires_manual_reconciliation() {
        let set = Migrations::new(Vec::new(), Dialect::MySQL);
        let text = set
            .interrupted_migration_error(&["20230331141203_test"])
            .expect("dirty row must produce an error")
            .to_string();

        assert!(text.contains("reconcile the schema by hand"), "{text}");
        assert!(!text.contains("drizzle migrate --repair"), "{text}");
        assert!(text.contains("UPDATE `__drizzle_migrations` SET"), "{text}");
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
    fn mysql_interrupted_migration_recovery_uses_mysql_identifiers() {
        let set = Migrations::new(Vec::new(), Dialect::MySQL);
        let text = set
            .interrupted_migration_error(&["partial"])
            .expect("dirty row")
            .to_string();
        assert!(
            text.contains("SET `applied_at` = CURRENT_TIMESTAMP WHERE `name` = 'partial'"),
            "{text}"
        );
        assert!(!text.contains("\"applied_at\""), "{text}");
    }

    #[test]
    fn mysql_tracking_table_lookup_is_scoped_to_current_database() {
        let sql = Migrations::new(Vec::new(), Dialect::MySQL).table_exists_sql();
        assert!(sql.contains("table_schema = DATABASE()"), "{sql}");
        assert!(sql.contains("table_name='__drizzle_migrations'"), "{sql}");
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
    fn match_applied_metadata_truncates_legacy_millis_to_seconds() {
        // A drizzle-kit journal `when` keeps milliseconds; the converted
        // folder name (and so the local created_at) only has seconds.
        let migrations = vec![
            super::Migration::new("20230331141203_alpha", "SELECT 1;"),
            super::Migration::new("20230331141204_beta", "SELECT 2;"),
        ];
        let matched = match_applied_migration_metadata(
            &migrations,
            &[
                AppliedMigrationMetadata {
                    id: Some(1),
                    hash: "stale_hash_a".to_string(),
                    created_at: 1_680_271_923_987,
                },
                AppliedMigrationMetadata {
                    id: Some(2),
                    hash: "stale_hash_b".to_string(),
                    created_at: 1_680_271_924_001,
                },
            ],
        )
        .expect("legacy millisecond rows match their second-precision folders");
        assert_eq!(matched[0].name, "20230331141203_alpha");
        assert_eq!(matched[1].name, "20230331141204_beta");
        // The stored value is kept as-is for the backfill's WHERE clause.
        assert_eq!(matched[0].created_at, 1_680_271_923_987);
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
