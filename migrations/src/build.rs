//! Build-time migration generation helpers.
//!
//! This module is intended for `build.rs` flows where users do not want to use
//! the CLI. It parses Rust schema files, computes diffs against the latest
//! snapshot in `./drizzle`, and writes a new migration folder when needed.
//!
//! # Recommended flow
//!
//! ```rust,no_run
//! use drizzle_migrations::build::{Config, Output, run};
//! use drizzle_types::Dialect;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cfg = Config::new(Dialect::SQLite)
//!         .file("src/schema.rs")
//!         .out("./drizzle");
//!
//!     // Tell Cargo to rerun build.rs when schema files change.
//!     cfg.watch();
//!
//!     match run(&cfg)? {
//!         Output::NoChanges => {}
//!         Output::Generated { tag, path, .. } => {
//!             println!("cargo:warning=generated migration {tag} at {}", path.display());
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::config::Tracking;
use crate::generate::diff;
use crate::parser::SchemaParser;
use crate::schema::Snapshot;
use crate::snapshot_builder::parse_result_to_snapshot;
use crate::words::{PrefixMode, generate_migration_tag_with_mode};
pub use drizzle_types::Casing;
use drizzle_types::{Dialect, EnvOr, EnvOrError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Build-time migration generation configuration.
#[derive(Debug, Clone)]
pub struct Config {
    files: Vec<PathBuf>,
    out_dir: PathBuf,
    dialect: Dialect,
    casing: Option<Casing>,
    breakpoints: bool,
    prefix_mode: PrefixMode,
    custom_name: Option<String>,
    url: Option<EnvOr>,
    tracking: Tracking,
    /// Path of the TOML config this was loaded from (if any). Watched by
    /// [`Config::watch`] alongside the schema files.
    config_path: Option<PathBuf>,
    /// Names of env vars referenced by `dbCredentials.url`. Emitted as
    /// `cargo:rerun-if-env-changed=` by [`Config::watch`].
    watched_env_vars: Vec<String>,
}

impl Config {
    /// Create a new configuration.
    ///
    /// `out_dir` defaults to `./drizzle`, breakpoints are enabled by default,
    /// and migration tag prefixes default to timestamp mode. Tracking defaults
    /// to the dialect-appropriate `Tracking::SQLITE` / `Tracking::POSTGRES`.
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        Self {
            files: Vec::new(),
            out_dir: PathBuf::from("./drizzle"),
            dialect,
            casing: None,
            breakpoints: true,
            prefix_mode: PrefixMode::Timestamp,
            custom_name: None,
            url: None,
            tracking: default_tracking(dialect),
            config_path: None,
            watched_env_vars: Vec::new(),
        }
    }

    /// Load configuration from a `drizzle.config.toml` file.
    ///
    /// Reads `dialect`, `schema` (one path or a list), `out`, `dbCredentials.url`
    /// (literal string or `{ env = "VAR" }`), and an optional `[migrations]`
    /// section with `table` / `schema` overrides for the tracking table.
    ///
    /// Anything else in the file is ignored — this loader covers only what
    /// the build-time generate/migrate flow needs. The CLI's full loader
    /// handles multi-database configs, filters, and casing-from-TOML.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::ConfigNotFound`] if the file is missing,
    /// [`BuildError::Io`] for other read failures, or [`BuildError::Toml`]
    /// if it fails to parse.
    pub fn from_toml(path: impl AsRef<Path>) -> Result<Self, BuildError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                BuildError::ConfigNotFound(path.to_path_buf())
            } else {
                BuildError::Io(source)
            }
        })?;
        let raw: RawConfig = toml::from_str(&content).map_err(|source| BuildError::Toml {
            path: path.to_path_buf(),
            source,
        })?;

        let dialect = raw.dialect;
        let mut cfg = Self::new(dialect);
        cfg.config_path = Some(path.to_path_buf());

        if let Some(out) = raw.out {
            cfg.out_dir = out;
        }
        cfg.files = match raw.schema {
            Some(SchemaPaths::One(s)) => vec![PathBuf::from(s)],
            Some(SchemaPaths::Many(v)) => v.into_iter().map(PathBuf::from).collect(),
            None => Vec::new(),
        };
        if let Some(b) = raw.breakpoints {
            cfg.breakpoints = b;
        }
        if let Some(c) = raw.casing {
            cfg.casing = Some(c);
        }
        if let Some(creds) = raw.db_credentials {
            if let EnvOr::Env(ref var) = creds.url {
                cfg.watched_env_vars.push(var.clone());
            }
            cfg.url = Some(creds.url);
        }
        if let Some(m) = raw.migrations {
            if let Some(t) = m.table {
                cfg.tracking = cfg.tracking.table(t);
            }
            if let Some(s) = m.schema {
                cfg.tracking = cfg.tracking.schema(s);
            }
        }

        Ok(cfg)
    }

    /// Add one Rust source file to the build input set.
    #[must_use]
    pub fn file(self, path: impl Into<PathBuf>) -> Self {
        let mut this = self;
        this.files.push(path.into());
        this
    }

    /// Set the output migrations directory.
    #[must_use]
    pub fn out(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = out_dir.into();
        self
    }

    /// Set the inferred naming casing strategy.
    #[must_use]
    pub const fn casing(mut self, casing: Casing) -> Self {
        self.casing = Some(casing);
        self
    }

    /// Enable or disable statement breakpoints in written SQL.
    #[must_use]
    pub const fn breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }

    /// Set migration tag prefix mode.
    #[must_use]
    pub const fn prefix_mode(mut self, mode: PrefixMode) -> Self {
        self.prefix_mode = mode;
        self
    }

    /// Set a custom suffix for the generated migration tag.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    /// Emit `cargo:rerun-if-changed=` for schema files (and the TOML config,
    /// if loaded via [`Config::from_toml`]), and `cargo:rerun-if-env-changed=`
    /// for any env vars referenced by `dbCredentials.url`.
    ///
    /// Call this once after construction so cargo reruns `build.rs` whenever
    /// any relevant input changes.
    pub fn watch(&self) {
        for path in &self.files {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        if let Some(cfg_path) = &self.config_path {
            println!("cargo:rerun-if-changed={}", cfg_path.display());
        }
        for var in &self.watched_env_vars {
            println!("cargo:rerun-if-env-changed={var}");
        }
    }

    /// Dialect this config targets.
    #[inline]
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Migrations output directory (where generated `migration.sql` /
    /// `snapshot.json` folders are written).
    #[inline]
    #[must_use]
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    /// Resolved database URL, reading from the environment if configured as
    /// `{ env = "VAR" }`.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::MissingUrl`] if no URL was configured,
    /// [`BuildError::EnvVarNotSet`] if a referenced env var is unset, or
    /// [`BuildError::EnvVarNotUnicode`] if it is set but contains invalid UTF-8.
    pub fn url(&self) -> Result<String, BuildError> {
        let cred = self.url.as_ref().ok_or(BuildError::MissingUrl)?;
        cred.resolve().map_err(|e| match e {
            EnvOrError::NotPresent(var) => BuildError::EnvVarNotSet(var),
            EnvOrError::NotUnicode(var) => BuildError::EnvVarNotUnicode(var),
        })
    }

    /// Migration tracking table/schema for this config.
    ///
    /// Defaults to the dialect-appropriate `Tracking::SQLITE` /
    /// `Tracking::POSTGRES`, with overrides applied from
    /// `[migrations] table = ...` / `schema = ...` in TOML if present.
    #[inline]
    #[must_use]
    pub fn tracking(&self) -> Tracking {
        self.tracking.clone()
    }
}

#[inline]
fn default_tracking(dialect: Dialect) -> Tracking {
    match dialect {
        Dialect::PostgreSQL => Tracking::POSTGRES,
        _ => Tracking::SQLITE,
    }
}

// ============================================================================
// drizzle.config.toml — minimal shape for build.rs
// ============================================================================

/// Raw TOML shape — see [`Config::from_toml`] for the user-facing docs.
///
/// This deliberately ignores fields the build-time flow doesn't need
/// (multi-DB, filters, driver, etc.); the CLI's loader covers those.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConfig {
    dialect: Dialect,
    #[serde(default)]
    schema: Option<SchemaPaths>,
    #[serde(default)]
    out: Option<PathBuf>,
    #[serde(default)]
    breakpoints: Option<bool>,
    #[serde(default)]
    casing: Option<Casing>,
    #[serde(default)]
    db_credentials: Option<RawCreds>,
    #[serde(default)]
    migrations: Option<RawMigrations>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SchemaPaths {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct RawCreds {
    url: EnvOr,
}

#[derive(Debug, Deserialize)]
struct RawMigrations {
    #[serde(default)]
    table: Option<String>,
    #[serde(default)]
    schema: Option<String>,
}

/// Result of a build-time migration generation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// No schema changes were detected.
    NoChanges,
    /// A new migration folder was written.
    Generated {
        /// Generated migration tag (folder name).
        tag: String,
        /// Absolute/relative path to the written migration directory.
        path: PathBuf,
        /// Number of SQL statements emitted.
        statement_count: usize,
    },
}

impl Output {
    #[must_use]
    pub const fn is_generated(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }
}

/// Errors that can occur while generating migrations in `build.rs`.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unsupported dialect for build generation: {0:?}")]
    UnsupportedDialect(Dialect),

    #[error("no schema files configured")]
    MissingSchemaFiles,

    #[error("failed to read schema file `{path:?}`: {source}")]
    ReadSchema {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse or write migration metadata: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to generate migration diff: {0}")]
    Migration(#[from] crate::writer::MigrationError),

    #[error("config file not found: {}", .0.display())]
    ConfigNotFound(PathBuf),

    #[error("failed to parse config `{}`: {source}", path.display())]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("no database URL configured (set `dbCredentials.url` in TOML)")]
    MissingUrl,

    #[error("env var `{0}` not set")]
    EnvVarNotSet(String),

    #[error("env var `{0}` contains invalid unicode")]
    EnvVarNotUnicode(String),
}

/// Generate and write a migration folder when schema changes are detected.
///
/// This is the high-level API that handles:
/// - diffing against the latest local snapshot
/// - tag generation
/// - writing `migration.sql` and `snapshot.json` in `./drizzle/<tag>/`
///
/// # Example
///
/// ```rust,no_run
/// use drizzle_migrations::build::{Config, Output, run};
/// use drizzle_types::Dialect;
///
/// let cfg = Config::new(Dialect::SQLite)
///     .file("src/schema.rs")
///     .out("./drizzle");
///
/// let outcome = run(&cfg)?;
/// if let Output::Generated { tag, .. } = outcome {
///     println!("generated {tag}");
/// }
/// # Ok::<(), drizzle_migrations::BuildError>(())
/// ```
///
/// # Errors
///
/// Returns a [`BuildError`] if the config has no schema files, the dialect is
/// unsupported, schema parsing fails, snapshot/migration generation fails, or
/// any filesystem operation (read/write) errors while materializing the
/// migration folder.
pub fn run(config: &Config) -> Result<Output, BuildError> {
    if config.files.is_empty() {
        return Err(BuildError::MissingSchemaFiles);
    }

    if !matches!(config.dialect, Dialect::SQLite | Dialect::PostgreSQL) {
        return Err(BuildError::UnsupportedDialect(config.dialect));
    }

    let parse_result = parse_files(&config.files)?;
    if parse_result.tables.is_empty() && parse_result.indexes.is_empty() {
        return Ok(Output::NoChanges);
    }

    let current_snapshot = parse_result_to_snapshot(&parse_result, config.dialect, config.casing);
    let previous_snapshot = load_previous_snapshot(&config.out_dir, config.dialect)?;
    let generated = diff(&previous_snapshot, &current_snapshot)?;

    if generated.is_empty() {
        return Ok(Output::NoChanges);
    }

    for warning in &generated.warnings {
        println!("cargo:warning={warning}");
    }

    std::fs::create_dir_all(&config.out_dir)?;
    let next_idx = next_migration_index(&config.out_dir)?;
    let tag = generate_migration_tag_with_mode(
        config.prefix_mode,
        next_idx,
        config.custom_name.as_deref(),
    );

    let migration_dir = config.out_dir.join(&tag);
    std::fs::create_dir_all(&migration_dir)?;

    let sql = if config.breakpoints {
        generated.to_sql()
    } else {
        generated.statements.join("\n\n")
    };
    std::fs::write(migration_dir.join("migration.sql"), sql)?;

    generated
        .snapshot
        .save(&migration_dir.join("snapshot.json"))?;

    Ok(Output::Generated {
        tag,
        path: migration_dir,
        statement_count: generated.statements.len(),
    })
}

fn parse_files(files: &[PathBuf]) -> Result<crate::parser::ParseResult, BuildError> {
    let mut combined = String::new();
    for path in files {
        let code = std::fs::read_to_string(path).map_err(|source| BuildError::ReadSchema {
            path: path.clone(),
            source,
        })?;
        combined.push_str(&code);
        combined.push('\n');
    }
    Ok(SchemaParser::parse(&combined))
}

fn load_previous_snapshot(out_dir: &Path, dialect: Dialect) -> Result<Snapshot, BuildError> {
    let v3_entries = collect_v3_migration_dirs(out_dir)?;
    if let Some((_, migration_dir)) = v3_entries.last() {
        let snapshot_path = migration_dir.join("snapshot.json");
        if snapshot_path.exists() {
            return Snapshot::load(&snapshot_path, dialect).map_err(BuildError::from);
        }
    }

    Ok(Snapshot::empty(dialect))
}

fn next_migration_index(out_dir: &Path) -> Result<u32, BuildError> {
    let entries = collect_v3_migration_dirs(out_dir)?;
    let mut max_index: Option<u32> = None;

    for (tag, _) in &entries {
        let Some(prefix) = tag.split('_').next() else {
            continue;
        };

        if prefix.len() > 10 || !prefix.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        if let Ok(idx) = prefix.parse::<u32>() {
            max_index = Some(max_index.map_or(idx, |curr| curr.max(idx)));
        }
    }

    Ok(max_index.map_or_else(
        || u32::try_from(entries.len()).unwrap_or(u32::MAX),
        |idx| idx.saturating_add(1),
    ))
}

fn collect_v3_migration_dirs(out_dir: &Path) -> Result<Vec<(String, PathBuf)>, BuildError> {
    if !out_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        if tag == "meta" {
            continue;
        }

        let path = entry.path();
        if path.join("migration.sql").exists() {
            entries.push((tag, path));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_creates_then_stabilizes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");

        std::fs::write(
            &schema_path,
            r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}
"#,
        )
        .expect("write schema");

        let cfg = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir);

        let first = run(&cfg).expect("first generation should succeed");
        assert!(matches!(first, Output::Generated { .. }));
        assert!(
            !out_dir.join("meta").join("_journal.json").exists(),
            "v3 generation should not create legacy journal metadata"
        );

        let second = run(&cfg).expect("second generation should succeed");
        assert_eq!(second, Output::NoChanges);
    }

    #[test]
    fn run_accepts_multiple_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let users_path = dir.path().join("users.rs");
        let posts_path = dir.path().join("posts.rs");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");

        std::fs::write(
            &users_path,
            r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub name: String,
}
"#,
        )
        .expect("write users schema");

        std::fs::write(
            &posts_path,
            r#"
#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
    #[column(references = Users::id)]
    pub author_id: i64,
}
"#,
        )
        .expect("write posts schema");

        std::fs::write(
            &schema_path,
            r#"
#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
}
"#,
        )
        .expect("write root schema");

        let cfg = Config::new(Dialect::SQLite)
            .file(&users_path)
            .file(&posts_path)
            .file(&schema_path)
            .out(&out_dir);

        let outcome = run(&cfg).expect("generation should succeed");
        let Output::Generated { path, .. } = outcome else {
            panic!("expected a migration to be generated");
        };

        let migration_sql_path = path.join("migration.sql");
        assert!(migration_sql_path.exists(), "migration.sql should exist");
        assert!(
            path.join("snapshot.json").exists(),
            "snapshot.json should exist"
        );

        let migration_sql =
            std::fs::read_to_string(&migration_sql_path).expect("read generated migration.sql");
        let mut statements: Vec<_> = migration_sql
            .split("\n--> statement-breakpoint\n")
            .map(str::to_string)
            .collect();
        statements.sort();

        let mut expected = vec![
            "CREATE TABLE `posts` (\n\t`id` INTEGER PRIMARY KEY,\n\t`author_id` INTEGER NOT NULL,\n\tCONSTRAINT `posts_author_id_users_id_fk` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`)\n);".to_string(),
            "CREATE TABLE `users` (\n\t`id` INTEGER PRIMARY KEY,\n\t`name` TEXT NOT NULL\n);".to_string(),
        ];
        expected.sort();

        assert_eq!(statements, expected, "unexpected generated migration SQL");
    }

    #[test]
    fn from_toml_loads_minimal_sqlite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("drizzle.config.toml");
        std::fs::write(
            &cfg_path,
            r#"
dialect = "sqlite"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
"#,
        )
        .expect("write config");

        let cfg = Config::from_toml(&cfg_path).expect("load toml");

        assert_eq!(cfg.dialect(), Dialect::SQLite);
        assert_eq!(cfg.out_dir(), Path::new("./drizzle"));
        assert_eq!(cfg.url().expect("resolve url"), "./dev.db");
        assert_eq!(cfg.tracking(), Tracking::SQLITE);
    }

    #[test]
    fn from_toml_handles_env_url_and_multiple_schemas() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("drizzle.config.toml");
        std::fs::write(
            &cfg_path,
            r#"
dialect = "postgresql"
schema = ["src/users.rs", "src/posts.rs"]

[dbCredentials]
url = { env = "DRIZZLE_BUILD_TEST_URL" }

[migrations]
table = "my_migrations"
schema = "drizzle_meta"
"#,
        )
        .expect("write config");

        let cfg = Config::from_toml(&cfg_path).expect("load toml");
        assert_eq!(cfg.dialect(), Dialect::PostgreSQL);

        let tracking = cfg.tracking();
        assert_eq!(tracking.table, "my_migrations");
        assert_eq!(tracking.schema.as_deref(), Some("drizzle_meta"));

        // SAFETY: single-test scope, no other env consumers race here.
        unsafe { std::env::set_var("DRIZZLE_BUILD_TEST_URL", "postgres://x") };
        assert_eq!(cfg.url().expect("resolve env"), "postgres://x");
        unsafe { std::env::remove_var("DRIZZLE_BUILD_TEST_URL") };

        let err = cfg.url().expect_err("missing env var should error");
        assert!(
            matches!(err, BuildError::EnvVarNotSet(ref v) if v == "DRIZZLE_BUILD_TEST_URL"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn from_toml_missing_url_errors_lazily() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("drizzle.config.toml");
        std::fs::write(
            &cfg_path,
            r#"
dialect = "sqlite"
schema = "src/schema.rs"
"#,
        )
        .expect("write config");

        // Missing dbCredentials is fine — only fails when url() is called.
        let cfg = Config::from_toml(&cfg_path).expect("load toml");
        assert!(matches!(cfg.url(), Err(BuildError::MissingUrl)));
    }
}
