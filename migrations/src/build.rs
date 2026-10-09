//! Generate migrations from `build.rs`, without the CLI.
//!
//! [`run`] parses your Rust schema files, diffs them against the newest
//! snapshot in the output folder (`./drizzle` by default), and writes a new
//! `<tag>/migration.sql` + `<tag>/snapshot.json` when something changed.
//!
//! # Examples
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
use crate::generate::{DiffOptions, diff_with};
use crate::naming::{PrefixMode, generate_migration_tag_with_mode};
use crate::parser::SchemaParser;
use crate::schema::Snapshot;
pub use drizzle_types::Casing;
use drizzle_types::{ConfigValue, ConfigValueError, Dialect};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Settings for [`run`]: schema files, output folder, dialect, and naming.
///
/// Build it with [`Config::new`] and the builder methods, or load it from a
/// `drizzle.config.toml` with [`Config::from_toml`].
#[derive(Debug, Clone)]
pub struct Config {
    files: Vec<PathBuf>,
    out_dir: PathBuf,
    dialect: Dialect,
    casing: Option<Casing>,
    breakpoints: bool,
    prefix_mode: PrefixMode,
    custom_name: Option<String>,
    ignore_conflicts: bool,
    url: Option<ConfigValue>,
    tracking: Tracking,
    /// Path of the TOML config this was loaded from (if any). Watched by
    /// [`Config::watch`] alongside the schema files.
    config_path: Option<PathBuf>,
    /// Names of env vars referenced by `dbCredentials.url`. Emitted as
    /// `cargo:rerun-if-env-changed=` by [`Config::watch`].
    watched_env_vars: Vec<String>,
    /// Optional last-mile rewrite of the generated statements.
    transform: Option<StatementTransform>,
    sqlite_rebuild_data: Option<SqliteRebuildDataSource>,
    renames: crate::RenameHints,
}

/// Boxed statement-transform callback.
///
/// Wrapped in a newtype so [`Config`] keeps its derived `Debug` and `Clone`
/// (a bare `Box<dyn Fn>` has neither).
#[derive(Clone)]
struct StatementTransform(std::sync::Arc<dyn Fn(Vec<String>) -> Vec<String> + Send + Sync>);

#[derive(Clone, Debug)]
enum SqliteRebuildDataSource {
    Inline(crate::sqlite::SqliteRebuildDataPlanRegistry),
    File(PathBuf),
}

impl std::fmt::Debug for StatementTransform {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<statement transform>")
    }
}

impl Config {
    /// Creates a config for `dialect` with no schema files.
    ///
    /// `out_dir` defaults to `./drizzle`, breakpoints are enabled by default,
    /// and migration tag prefixes default to timestamp mode. Tracking defaults
    /// to the dialect-appropriate `Tracking::SQLITE`, `Tracking::POSTGRES`, or
    /// `Tracking::MYSQL` value.
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
            ignore_conflicts: false,
            url: None,
            tracking: default_tracking(dialect),
            config_path: None,
            watched_env_vars: Vec::new(),
            transform: None,
            sqlite_rebuild_data: None,
            renames: crate::RenameHints::default(),
        }
    }

    /// Loads a config from a `drizzle.config.toml` file.
    ///
    /// Reads `dialect`, `schema` (one path or a list), `out`, `breakpoints`,
    /// `casing`, `dbCredentials.url` (literal string or `{ env = "VAR" }`),
    /// and an optional `[migrations]` section with `table`/`schema` tracking
    /// overrides and a `sqliteRebuildDataPlan` path (relative to the config
    /// file).
    ///
    /// Anything else in the file is ignored: this loader covers only what
    /// the build-time generate/migrate flow needs. The CLI's full loader
    /// handles multi-database configs and filters.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use drizzle_migrations::build::{Config, run};
    ///
    /// let cfg = Config::from_toml("drizzle.config.toml")?;
    /// cfg.watch();
    /// run(&cfg)?;
    /// # Ok::<(), drizzle_migrations::BuildError>(())
    /// ```
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
            if let ConfigValue::Env(ref var) = creds.url {
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
            if let Some(plan) = m.sqlite_rebuild_data_plan {
                let base = path.parent().unwrap_or_else(|| Path::new("."));
                cfg.sqlite_rebuild_data = Some(SqliteRebuildDataSource::File(base.join(plan)));
            }
        }

        Ok(cfg)
    }

    /// Adds a Rust schema file. Call it once per file.
    #[must_use]
    pub fn file(self, path: impl Into<PathBuf>) -> Self {
        let mut this = self;
        this.files.push(path.into());
        this
    }

    /// Sets the migrations folder (default `./drizzle`).
    #[must_use]
    pub fn out(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = out_dir.into();
        self
    }

    /// Records a [`Casing`] preference.
    ///
    /// Currently has no effect on generated migrations: the table macros
    /// always snake_case inferred names, so the diff uses those names. Kept
    /// for API compatibility.
    #[must_use]
    pub const fn casing(mut self, casing: Casing) -> Self {
        self.casing = Some(casing);
        self
    }

    /// Writes `--> statement-breakpoint` between statements when `true`
    /// (the default); joins them with blank lines when `false`.
    #[must_use]
    pub const fn breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }

    /// Sets how the migration folder name is prefixed (default:
    /// [`PrefixMode::Timestamp`]).
    #[must_use]
    pub const fn prefix_mode(mut self, mode: PrefixMode) -> Self {
        self.prefix_mode = mode;
        self
    }

    /// Uses `name` as the tag suffix instead of a random `adjective_hero`
    /// (for example `20240101120000_add_users`).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    /// When migration branches merged into the output folder change the same
    /// objects, diff against the newest snapshot instead of failing
    /// (drizzle-kit's `--ignore-conflicts`). See
    /// [`history::load_merge_base`](crate::history::load_merge_base).
    #[must_use]
    pub const fn ignore_conflicts(mut self, ignore: bool) -> Self {
        self.ignore_conflicts = ignore;
        self
    }

    /// Rewrite the generated statements before they are written to
    /// `migration.sql`.
    ///
    /// This is the supported place for app-level DDL policy — ephemeral
    /// tables, engine-specific pragmas, `IF NOT EXISTS` conventions, dropping
    /// statements for objects the app manages itself. Encoding the policy here
    /// keeps it in version control and re-applies it to every future
    /// migration; hand-editing generated SQL does neither.
    ///
    /// The callback receives the statements in execution order and returns the
    /// list to write. Returning an empty list makes the run report
    /// [`Output::NoChanges`] and write nothing.
    ///
    /// The snapshot is **not** transformed: it records the schema the diff was
    /// computed from, and rewriting it would desynchronize the next diff.
    ///
    /// Cannot be combined with a SQLite rebuild-data plan; [`run`] returns
    /// [`BuildError::SqliteRebuildDataTransformConflict`] if both are set.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use drizzle_migrations::build::{Config, run};
    /// use drizzle_types::Dialect;
    ///
    /// let cfg = Config::new(Dialect::SQLite)
    ///     .file("src/schema.rs")
    ///     .out("./drizzle")
    ///     // Session-scoped scratch tables are created by the app at startup,
    ///     // so migrations must not manage them.
    ///     .transform_statements(|statements| {
    ///         statements
    ///             .into_iter()
    ///             .filter(|sql| !sql.contains("`scratch_"))
    ///             .collect()
    ///     });
    ///
    /// run(&cfg)?;
    /// # Ok::<(), drizzle_migrations::BuildError>(())
    /// ```
    ///
    /// Runtime-generation callers do not need this hook: [`crate::Plan`]
    /// exposes `statements` as a public `Vec<String>`, so they can rewrite the
    /// plan directly before executing or writing it.
    #[must_use]
    pub fn transform_statements(
        mut self,
        transform: impl Fn(Vec<String>) -> Vec<String> + Send + Sync + 'static,
    ) -> Self {
        self.transform = Some(StatementTransform(std::sync::Arc::new(transform)));
        self
    }

    /// Attaches typed data movement to SQLite table rebuilds in this generation.
    ///
    /// The plan is validated against both schema snapshots and its exact
    /// predecessor ID. It does not rewrite generated statements after diffing.
    #[must_use]
    pub fn sqlite_rebuild_data_plan(mut self, plan: crate::sqlite::SqliteRebuildDataPlan) -> Self {
        self.sqlite_rebuild_data = Some(SqliteRebuildDataSource::Inline(
            crate::sqlite::SqliteRebuildDataPlanRegistry::single(plan),
        ));
        self
    }

    /// Attaches a versioned registry of snapshot-bound SQLite rebuild plans.
    #[must_use]
    pub fn sqlite_rebuild_data_plan_registry(
        mut self,
        registry: crate::sqlite::SqliteRebuildDataPlanRegistry,
    ) -> Self {
        self.sqlite_rebuild_data = Some(SqliteRebuildDataSource::Inline(registry));
        self
    }

    /// Loads a checked-in, versioned SQLite rebuild-data plan (JSON) from
    /// `path` when [`run`] is called.
    #[must_use]
    pub fn sqlite_rebuild_data_plan_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.sqlite_rebuild_data = Some(SqliteRebuildDataSource::File(path.into()));
        self
    }

    /// Answers the rename-or-create questions of this generation.
    ///
    /// When a schema, enum, table or column disappears and another of the
    /// same kind appears, [`run`] fails with
    /// [`MigrationError::UnansweredRenames`](crate::MigrationError::UnansweredRenames)
    /// rather than guess, and the error gives the hint for each answer.
    /// Hints that match nothing in a later diff are ignored, so they can stay
    /// in `build.rs` after the migration is generated.
    ///
    /// ```rust,no_run
    /// use drizzle_migrations::RenameHints;
    /// use drizzle_migrations::build::Config;
    /// use drizzle_types::Dialect;
    ///
    /// let cfg = Config::new(Dialect::SQLite)
    ///     .file("src/schema.rs")
    ///     .renames(RenameHints::new().rename_column("users", "name", "full_name"));
    /// ```
    #[must_use]
    pub fn renames(mut self, renames: crate::RenameHints) -> Self {
        self.renames = renames;
        self
    }

    /// Apply the configured statement transform, if any.
    fn apply_transform(&self, statements: Vec<String>) -> Vec<String> {
        match &self.transform {
            Some(StatementTransform(transform)) => transform(statements),
            None => statements,
        }
    }

    /// Paths cargo must watch: the schema files, the TOML config (if loaded
    /// via [`Config::from_toml`]), the migrations output directory, and the
    /// SQLite rebuild-data plan file (if any).
    ///
    /// Split out of [`Config::watch`] so the set is assertable without
    /// capturing the build script's stdout.
    fn watch_targets(&self) -> Vec<PathBuf> {
        let mut targets = self.files.clone();
        if let Some(cfg_path) = &self.config_path {
            targets.push(cfg_path.clone());
        }
        targets.push(self.out_dir.clone());
        if let Some(SqliteRebuildDataSource::File(path)) = &self.sqlite_rebuild_data {
            targets.push(path.clone());
        }
        targets
    }

    /// Prints `cargo:rerun-if-changed=` for schema files, the TOML config (if
    /// loaded via [`Config::from_toml`]), the migrations output directory, and
    /// a SQLite rebuild-data plan file (if one is set), plus
    /// `cargo:rerun-if-env-changed=` for any env vars referenced by
    /// `dbCredentials.url`.
    ///
    /// The output directory is watched because the previous-snapshot chain
    /// under it is a diff input: deleting or reverting a migration folder
    /// changes what [`run`] generates. Without it, cargo sees no watched path
    /// change, skips the script, replays the cached "generated migration"
    /// output, and the migration is silently never regenerated. Cargo scans a
    /// watched directory recursively, and a not-yet-existing one counts as
    /// changed — the first run creates it, so this converges.
    ///
    /// Call this once after construction so cargo reruns `build.rs` whenever
    /// any relevant input changes.
    pub fn watch(&self) {
        for path in self.watch_targets() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        for var in &self.watched_env_vars {
            println!("cargo:rerun-if-env-changed={var}");
        }
    }

    /// Returns the dialect this config targets.
    #[inline]
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Returns the migrations folder.
    #[inline]
    #[must_use]
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    /// Returns the database URL, reading the env var when it was configured
    /// as `{ env = "VAR" }`.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::MissingUrl`] if no URL was configured,
    /// [`BuildError::EnvVarNotSet`] if a referenced env var is unset, or
    /// [`BuildError::EnvVarNotUnicode`] if it is set but contains invalid UTF-8.
    pub fn url(&self) -> Result<String, BuildError> {
        let cred = self.url.as_ref().ok_or(BuildError::MissingUrl)?;
        cred.resolve().map_err(|e| match e {
            ConfigValueError::NotPresent(var) => BuildError::EnvVarNotSet(var),
            ConfigValueError::NotUnicode(var) => BuildError::EnvVarNotUnicode(var),
        })
    }

    /// Returns the migration tracking table and schema.
    ///
    /// Defaults to the dialect-appropriate `Tracking::SQLITE`,
    /// `Tracking::POSTGRES`, or `Tracking::MYSQL`, with overrides applied from
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
        Dialect::MySQL => Tracking::MYSQL,
        Dialect::SQLite => Tracking::SQLITE,
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
    url: ConfigValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMigrations {
    #[serde(default)]
    table: Option<String>,
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    sqlite_rebuild_data_plan: Option<PathBuf>,
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
        /// Path to the written migration folder (under the configured
        /// output folder).
        path: PathBuf,
        /// Number of SQL statements emitted.
        statement_count: usize,
    },
}

impl Output {
    /// Returns `true` for [`Output::Generated`].
    #[must_use]
    pub const fn is_generated(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }
}

/// Errors from [`run`] and [`Config`] loading.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// No schema files were added with [`Config::file`] or `schema` in TOML.
    #[error("no schema files configured")]
    MissingSchemaFiles,

    /// A SQLite rebuild-data plan and a statement transform were both set.
    #[error(
        "SQLite rebuild-data plans cannot be combined with statement transforms; typed plan validation must remain the final migration authority"
    )]
    SqliteRebuildDataTransformConflict,

    /// A schema file could not be read.
    #[error("failed to read schema file `{path:?}`: {source}")]
    ReadSchema {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The schema source has parse errors.
    #[error("schema source failed to parse:\n{0}")]
    SchemaParse(String),

    /// Reading or writing the migrations folder failed.
    #[error("failed to parse or write migration metadata: {0}")]
    Io(#[from] std::io::Error),

    /// Diffing or writing the migration failed.
    #[error("failed to generate migration diff: {0}")]
    Migration(#[from] crate::writer::MigrationError),

    /// Migration branches merged into the output folder conflict, or their
    /// snapshots could not be read.
    #[error(transparent)]
    History(#[from] crate::history::HistoryError),

    /// The TOML config file does not exist.
    #[error("config file not found: {}", .0.display())]
    ConfigNotFound(PathBuf),

    /// The TOML config file is invalid.
    #[error("failed to parse config `{}`: {source}", path.display())]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// The SQLite rebuild-data plan file could not be read.
    #[error("failed to read SQLite rebuild-data plan `{}`: {source}", path.display())]
    ReadSqliteRebuildDataPlan {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The SQLite rebuild-data plan file is not valid JSON for the plan.
    #[error("failed to parse SQLite rebuild-data plan `{}`: {source}", path.display())]
    ParseSqliteRebuildDataPlan {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// [`Config::url`] was called but no URL is configured.
    #[error("no database URL configured (set `dbCredentials.url` in TOML)")]
    MissingUrl,

    /// The env var named by `dbCredentials.url` is not set.
    #[error("env var `{0}` not set")]
    EnvVarNotSet(String),

    /// The env var named by `dbCredentials.url` is not valid UTF-8.
    #[error("env var `{0}` contains invalid unicode")]
    EnvVarNotUnicode(String),
}

fn load_sqlite_rebuild_data_plan(
    source: &SqliteRebuildDataSource,
) -> Result<crate::sqlite::SqliteRebuildDataPlanRegistry, BuildError> {
    match source {
        SqliteRebuildDataSource::Inline(plan) => Ok(plan.clone()),
        SqliteRebuildDataSource::File(path) => {
            let bytes =
                std::fs::read(path).map_err(|source| BuildError::ReadSqliteRebuildDataPlan {
                    path: path.clone(),
                    source,
                })?;
            serde_json::from_slice(&bytes).map_err(|source| {
                BuildError::ParseSqliteRebuildDataPlan {
                    path: path.clone(),
                    source,
                }
            })
        }
    }
}

/// Writes a new migration folder if the schema changed since the last snapshot.
///
/// Steps: parse the schema files, diff against the newest `snapshot.json`
/// in the output folder (or an empty schema; after a branch merge left
/// several open heads, against their merged state — see
/// [`history::load_merge_base`](crate::history::load_merge_base)), apply any
/// [`transform_statements`](Config::transform_statements) hook, then write
/// `<out>/<tag>/migration.sql` and `snapshot.json`. Parse and diff warnings
/// are printed as `cargo:warning=` lines. Returns [`Output::NoChanges`] when
/// the schema has no tables or indexes, or nothing changed.
///
/// # Examples
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
/// Returns a [`BuildError`] if the config has no schema files, both a SQLite
/// rebuild-data plan and a statement transform are set, the schema has parse
/// errors, merged migration branches conflict, diffing fails, the generated
/// tag already exists, or a file read/write fails.
pub fn run(config: &Config) -> Result<Output, BuildError> {
    if config.files.is_empty() {
        return Err(BuildError::MissingSchemaFiles);
    }

    if config.sqlite_rebuild_data.is_some() && config.transform.is_some() {
        return Err(BuildError::SqliteRebuildDataTransformConflict);
    }

    let parse_result = parse_files(&config.files)?;
    for warning in &parse_result.warnings {
        println!("cargo:warning=schema parse: {warning}");
    }
    // Entities are emitted best-effort even when parsing hit hard errors;
    // diffing a half-understood schema produces destructive DDL, so fail
    // loudly instead of quietly reporting "no changes".
    if !parse_result.errors.is_empty() {
        return Err(BuildError::SchemaParse(parse_result.errors.join("\n")));
    }
    if parse_result.tables.is_empty() && parse_result.indexes.is_empty() {
        return Ok(Output::NoChanges);
    }

    let current_snapshot =
        Snapshot::from_parse_result(&parse_result, config.dialect, config.casing);
    let merge_base =
        crate::history::load_merge_base(&config.out_dir, config.dialect, config.ignore_conflicts)?;
    let previous_snapshot = merge_base.snapshot;
    let sqlite_rebuild_data = config
        .sqlite_rebuild_data
        .as_ref()
        .map(load_sqlite_rebuild_data_plan)
        .transpose()?;
    let options = DiffOptions::new().with_renames(config.renames.clone());
    let options = match sqlite_rebuild_data {
        Some(registry) => options.sqlite_rebuild_data_registry(registry),
        None => options,
    };
    let mut generated = diff_with(&previous_snapshot, &current_snapshot, &options)?;
    // After a branch merge the new snapshot follows every open head.
    if let Some(prev_ids) = merge_base.prev_ids {
        generated.snapshot.set_prev_ids(prev_ids);
    }

    // App-level statement policy runs before the emptiness check, so a
    // transform that filters everything out reports NoChanges instead of
    // writing an empty migration.
    generated.statements = config.apply_transform(generated.statements);

    if generated.is_empty() {
        return Ok(Output::NoChanges);
    }

    for warning in &generated.warnings {
        println!("cargo:warning={warning}");
    }

    let next_idx = next_migration_index(&config.out_dir)?;
    let tag = generate_migration_tag_with_mode(
        config.prefix_mode,
        next_idx,
        config.custom_name.as_deref(),
    );

    let sql = if config.breakpoints {
        generated.to_sql()
    } else {
        generated.statements.join("\n\n")
    };

    // Stage-and-rename so a crash never leaves a torn folder, and an existing
    // tag is refused instead of silently overwritten.
    let migration_dir =
        crate::writer::publish_migration_directory(&config.out_dir, &tag, |staging| {
            std::fs::write(staging.join("migration.sql"), &sql)
                .map_err(|error| crate::writer::MigrationError::IoError(error.to_string()))?;
            generated
                .snapshot
                .save(&staging.join("snapshot.json"))
                .map_err(|error| crate::writer::MigrationError::SnapshotError(error.to_string()))?;
            Ok(())
        })?;

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

fn next_migration_index(out_dir: &Path) -> Result<u32, BuildError> {
    let entries = collect_v3_migration_dirs(out_dir)?;
    let mut max_index: Option<u32> = None;

    for (tag, _) in &entries {
        let Some(prefix) = tag.split('_').next() else {
            continue;
        };

        // Index prefixes are short (`0000`); longer digit runs are timestamp
        // (14), unix (10), or millisecond (13) prefixes, not indexes.
        if prefix.len() > 5 || !prefix.chars().all(|c| c.is_ascii_digit()) {
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
    use crate::sqlite::{
        SQLITE_REBUILD_DATA_PLAN_VERSION, SqliteColumnCopy, SqliteCopyExpression,
        SqliteDataValidation, SqliteIntegerMapping, SqliteRebuildDataPlan,
        SqliteRebuildDataPlanRegistry, SqliteTableRebuildPlan,
    };

    #[test]
    fn run_creates_then_stabilizes_for_every_dialect() {
        for (dialect, table_attribute) in [
            (Dialect::SQLite, "SQLiteTable"),
            (Dialect::PostgreSQL, "PostgresTable"),
            (Dialect::MySQL, "MySQLTable"),
        ] {
            let dir = tempfile::tempdir().expect("tempdir");
            let schema_path = dir.path().join("schema.rs");
            let out_dir = dir.path().join("drizzle");

            std::fs::write(
                &schema_path,
                format!(
                    r#"
#[{table_attribute}]
pub struct Users {{
    #[column(primary)]
    pub id: i64,
}}
"#
                ),
            )
            .expect("write schema");

            let cfg = Config::new(dialect).file(&schema_path).out(&out_dir);

            let first = run(&cfg).expect("first generation should succeed");
            assert!(
                matches!(first, Output::Generated { .. }),
                "{dialect:?} must generate the initial migration"
            );
            assert!(
                !out_dir.join("meta").join("_journal.json").exists(),
                "v3 generation should not create legacy journal metadata"
            );

            let second = run(&cfg).expect("second generation should succeed");
            assert_eq!(second, Output::NoChanges, "{dialect:?} must stabilize");
        }
    }

    #[test]
    fn generate_after_a_branch_merge_diffs_against_both_branches() {
        for (dialect, table_attribute) in [
            (Dialect::SQLite, "SQLiteTable"),
            (Dialect::PostgreSQL, "PostgresTable"),
            (Dialect::MySQL, "MySQLTable"),
        ] {
            let dir = tempfile::tempdir().expect("tempdir");
            let schema_path = dir.path().join("schema.rs");
            let main_out = dir.path().join("main");
            let other_out = dir.path().join("other");
            let schema = |tables: &[&str]| {
                let body: String = tables
                    .iter()
                    .map(|table| {
                        format!(
                            "#[{table_attribute}]\npub struct {table} {{\n    #[column(primary)]\n    pub id: i64,\n}}\n"
                        )
                    })
                    .collect();
                std::fs::write(&schema_path, body).expect("write schema");
            };
            let generate = |out: &Path, name: &str| {
                run(&Config::new(dialect).file(&schema_path).out(out).name(name)).expect("generate")
            };
            let copy_folders = |from: &Path, to: &Path| {
                for entry in std::fs::read_dir(from).expect("read") {
                    let entry = entry.expect("entry");
                    let target = to.join(entry.file_name());
                    std::fs::create_dir_all(&target).expect("mkdir");
                    for file in std::fs::read_dir(entry.path()).expect("read") {
                        let file = file.expect("file");
                        std::fs::copy(file.path(), target.join(file.file_name())).expect("copy");
                    }
                }
            };

            schema(&["Users"]);
            assert!(generate(&main_out, "0_base").is_generated());
            std::fs::create_dir_all(&other_out).expect("mkdir");
            copy_folders(&main_out, &other_out);

            // Two branches each add a table, then git merges the folders.
            schema(&["Users", "Alpha"]);
            assert!(generate(&main_out, "1_alpha").is_generated());
            schema(&["Users", "Beta"]);
            assert!(generate(&other_out, "2_beta").is_generated());
            copy_folders(&other_out, &main_out);

            // The merged schema is what both branches produce: no changes,
            // instead of re-emitting the other branch's CREATE TABLE.
            schema(&["Users", "Alpha", "Beta"]);
            assert_eq!(
                generate(&main_out, "3_noop"),
                Output::NoChanges,
                "{dialect:?}"
            );

            schema(&["Users", "Alpha", "Beta", "Gamma"]);
            let Output::Generated { path, .. } = generate(&main_out, "4_gamma") else {
                panic!("{dialect:?}: adding a table must generate");
            };
            let sql = std::fs::read_to_string(path.join("migration.sql")).expect("sql");
            assert!(sql.to_lowercase().contains("gamma"), "{dialect:?}: {sql}");
            assert!(!sql.to_lowercase().contains("alpha"), "{dialect:?}: {sql}");
            assert!(!sql.to_lowercase().contains("beta"), "{dialect:?}: {sql}");

            // The new snapshot follows both heads, closing the fork.
            let merged = Snapshot::load(&path.join("snapshot.json"), dialect).expect("snapshot");
            assert_eq!(merged.prev_ids().len(), 2, "{dialect:?}");
            let base = crate::history::load_merge_base(&main_out, dialect, false).expect("base");
            assert_eq!(base.prev_ids, None, "{dialect:?}: one head again");
            assert_eq!(base.snapshot.id(), merged.id());
        }
    }

    #[test]
    fn mysql_build_uses_legacy_v5_snapshot_as_the_diff_baseline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");
        let legacy_dir = out_dir.join("0000_legacy");
        std::fs::create_dir_all(&legacy_dir).expect("create legacy migration directory");
        std::fs::write(legacy_dir.join("migration.sql"), "-- legacy migration\n")
            .expect("write legacy migration");
        std::fs::write(
            legacy_dir.join("snapshot.json"),
            r#"{
  "version": "5",
  "dialect": "mysql",
  "id": "11111111-1111-1111-1111-111111111111",
  "prevId": "00000000-0000-0000-0000-000000000000",
  "tables": {
    "users": {
      "name": "users",
      "columns": {
        "id": {
          "name": "id",
          "type": "BIGINT",
          "primaryKey": true,
          "notNull": true
        }
      },
      "indexes": {},
      "foreignKeys": {},
      "compositePrimaryKeys": {},
      "uniqueConstraints": {},
      "checkConstraints": {}
    }
  },
  "views": {}
}"#,
        )
        .expect("write legacy snapshot");
        std::fs::write(
            &schema_path,
            r#"
#[MySQLTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}
"#,
        )
        .expect("write schema");

        let config = Config::new(Dialect::MySQL).file(&schema_path).out(&out_dir);
        assert_eq!(
            run(&config).expect("legacy v5 baseline must load and diff"),
            Output::NoChanges
        );
    }

    #[test]
    fn run_applies_snapshot_bound_typed_sqlite_rebuild_data() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");
        std::fs::write(
            &schema_path,
            r#"
#[SQLiteTable]
pub struct Assets {
    #[column(primary)]
    pub id: i64,
    pub digest: Option<String>,
    pub relation: i64,
    pub metadata: String,
}
"#,
        )
        .expect("write predecessor schema");
        let base = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir)
            .prefix_mode(PrefixMode::Index);
        let Output::Generated { path, .. } = run(&base).expect("generate predecessor") else {
            panic!("expected predecessor migration");
        };
        let predecessor = Snapshot::load(&path.join("snapshot.json"), Dialect::SQLite)
            .expect("load predecessor snapshot");

        std::fs::write(
            &schema_path,
            r#"
#[SQLiteTable(STRICT)]
pub struct Assets {
    #[column(primary)]
    pub id: i64,
    #[column(blob)]
    pub digest: Option<Vec<u8>>,
    pub relation: i64,
    pub metadata: String,
}
"#,
        )
        .expect("write current schema");
        let error = run(&base).expect_err("affinity change without a rebuild plan must fail");
        let BuildError::Migration(crate::writer::MigrationError::ConfigError(message)) = error
        else {
            panic!("unexpected error: {error:?}");
        };
        assert!(
            message.contains("storage affinity without a rebuild-data plan")
                && message.contains("assets.digest"),
            "{message}"
        );
        let plan = SqliteRebuildDataPlan {
            predecessor_snapshot_id: uuid::Uuid::parse_str(predecessor.id())
                .expect("generated predecessor ID is a UUID"),
            tables: vec![SqliteTableRebuildPlan {
                table: "assets".to_string(),
                columns: vec![
                    SqliteColumnCopy {
                        target: "digest".to_string(),
                        expression: SqliteCopyExpression::HexTextToBlob {
                            source: "digest".to_string(),
                            bytes: 32,
                        },
                    },
                    SqliteColumnCopy {
                        target: "relation".to_string(),
                        expression: SqliteCopyExpression::IntegerMap {
                            source: "relation".to_string(),
                            cases: vec![
                                SqliteIntegerMapping { from: 1, to: 0 },
                                SqliteIntegerMapping { from: 2, to: 1 },
                                SqliteIntegerMapping { from: 4, to: 2 },
                            ],
                        },
                    },
                ],
                validations: vec![SqliteDataValidation::JsonValid {
                    column: "metadata".to_string(),
                }],
            }],
        };
        let registry = SqliteRebuildDataPlanRegistry {
            version: SQLITE_REBUILD_DATA_PLAN_VERSION,
            plans: vec![plan],
        };
        let plan_path = dir.path().join("rebuild-data.json");
        std::fs::write(
            &plan_path,
            serde_json::to_vec_pretty(&registry).expect("serialize rebuild-data registry"),
        )
        .expect("write rebuild-data plan");
        let configured = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir)
            .prefix_mode(PrefixMode::Index)
            .sqlite_rebuild_data_plan_file(&plan_path);
        let Output::Generated { tag, path, .. } = run(&configured).expect("generate typed rebuild")
        else {
            panic!("expected typed rebuild migration");
        };
        let sql = std::fs::read_to_string(path.join("migration.sql"))
            .expect("read generated rebuild migration");
        assert!(
            sql.contains("coalesce(length(unhex(`digest`)), -1) <> 32"),
            "{sql}"
        );
        assert!(sql.contains("unhex(`digest`)"), "{sql}");
        assert!(
            sql.contains("CASE `relation` WHEN 1 THEN 0 WHEN 2 THEN 1 WHEN 4 THEN 2 ELSE NULL END"),
            "{sql}"
        );
        assert!(!sql.contains("IF EXISTS"), "{sql}");
        assert!(!sql.contains("IF NOT EXISTS"), "{sql}");

        let connection = rusqlite::Connection::open_in_memory().expect("open SQLite");
        connection
            .execute_batch(
                "CREATE TABLE assets (id INTEGER PRIMARY KEY, digest TEXT, relation INTEGER NOT NULL, metadata TEXT NOT NULL);\
                 INSERT INTO assets VALUES (1, '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f', 1, '[]');\
                 INSERT INTO assets VALUES (2, NULL, 4, '{}');",
            )
            .expect("seed valid predecessor rows");
        apply_generated_sql(&connection, &sql).expect("apply generated rebuild");
        let rows = connection
            .prepare("SELECT id, typeof(digest), length(digest), relation, metadata FROM assets ORDER BY id")
            .expect("prepare migrated read")
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .expect("query migrated rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("decode migrated rows");
        assert_eq!(
            rows,
            vec![
                (1, "blob".to_string(), Some(32), 0, "[]".to_string()),
                (2, "null".to_string(), None, 2, "{}".to_string()),
            ]
        );
        let guard_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_temp_master WHERE name LIKE '__drizzle_rebuild_guard_%'",
                [],
                |row| row.get(0),
            )
            .expect("query temp guards");
        assert_eq!(guard_count, 0, "successful migration must drop its guard");

        let migration = crate::Migration::new(&tag, &sql);
        let migrations = crate::Migrations::new(vec![migration], Dialect::SQLite);
        assert_eq!(migrations.pending::<String>(&[]).count(), 1);
        assert_eq!(migrations.pending(&[tag]).count(), 0);
        assert!(
            apply_generated_sql(&connection, &sql).is_err(),
            "forced replay must fail loudly"
        );

        assert_invalid_rebuild_row(
            &sql,
            "'zz0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f'",
            "1",
            "'[]'",
        );
        assert_invalid_rebuild_row(&sql, "NULL", "3", "'[]'");
        assert_invalid_rebuild_row(&sql, "NULL", "1", "'{' ");

        assert_eq!(
            run(&configured).expect("reopen generation with historical registry"),
            Output::NoChanges
        );
    }

    fn apply_generated_sql(connection: &rusqlite::Connection, sql: &str) -> rusqlite::Result<()> {
        for statement in sql.split("\n--> statement-breakpoint\n") {
            connection.execute_batch(statement)?;
        }
        Ok(())
    }

    fn assert_invalid_rebuild_row(sql: &str, digest: &str, relation: &str, metadata: &str) {
        let connection = rusqlite::Connection::open_in_memory().expect("open SQLite");
        connection
            .execute_batch(&format!(
                "CREATE TABLE assets (id INTEGER PRIMARY KEY, digest TEXT, relation INTEGER NOT NULL, metadata TEXT NOT NULL);\
                 INSERT INTO assets VALUES (1, {digest}, {relation}, {metadata});"
            ))
            .expect("seed invalid predecessor row");
        assert!(
            apply_generated_sql(&connection, sql).is_err(),
            "invalid predecessor row unexpectedly migrated: digest={digest}, relation={relation}, metadata={metadata}"
        );
        let table_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'assets'",
                [],
                |row| row.get(0),
            )
            .expect("predecessor table remains");
        assert!(!table_sql.contains("STRICT"), "copy began before preflight");
    }

    #[test]
    fn run_fails_loudly_on_parse_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");

        // Unbalanced brace: syn cannot parse the file. Before the errors
        // channel was wired up this quietly produced `NoChanges`.
        std::fs::write(
            &schema_path,
            r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
"#,
        )
        .expect("write schema");

        let cfg = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir);

        let error = run(&cfg).expect_err("parse failure must not be silent");
        assert!(
            matches!(error, BuildError::SchemaParse(_)),
            "expected SchemaParse, got {error:?}"
        );
        assert!(
            !out_dir.exists(),
            "no migration output should be written on parse failure"
        );
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
            "CREATE TABLE `posts` (\n\t`id` INTEGER PRIMARY KEY,\n\t`author_id` INTEGER NOT NULL,\n\tCONSTRAINT `fk_posts_author_id_users_id_fk` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`)\n);".to_string(),
            "CREATE TABLE `users` (\n\t`id` INTEGER PRIMARY KEY,\n\t`name` TEXT NOT NULL\n);".to_string(),
        ];
        expected.sort();

        assert_eq!(statements, expected, "unexpected generated migration SQL");
    }

    /// Write a one-table schema plus its root schema struct, returning
    /// `(schema_file, out_dir)`.
    fn two_table_schema(dir: &Path) -> (PathBuf, PathBuf) {
        let schema_path = dir.join("schema.rs");
        std::fs::write(
            &schema_path,
            r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub name: String,
}

#[SQLiteTable]
pub struct ScratchCache {
    #[column(primary)]
    pub id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub scratch_cache: ScratchCache,
}
"#,
        )
        .expect("write schema");
        (schema_path, dir.join("drizzle"))
    }

    #[test]
    fn transform_statements_rewrites_generated_sql() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (schema_path, out_dir) = two_table_schema(dir.path());

        let cfg = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir)
            // App-level policy: scratch tables are provisioned at runtime, so
            // migrations must not own them.
            .transform_statements(|statements| {
                statements
                    .into_iter()
                    .filter(|sql| !sql.contains("scratch_cache"))
                    .collect()
            });

        let Output::Generated {
            path,
            statement_count,
            ..
        } = run(&cfg).expect("generation should succeed")
        else {
            panic!("expected a migration to be generated");
        };

        assert_eq!(statement_count, 1, "transform dropped one statement");
        let sql = std::fs::read_to_string(path.join("migration.sql")).expect("read migration.sql");
        assert!(sql.contains("`users`"), "{sql}");
        assert!(
            !sql.contains("scratch_cache"),
            "filtered statement leaked into migration.sql: {sql}"
        );
        assert!(
            path.join("snapshot.json").exists(),
            "the snapshot still records the untransformed schema"
        );
    }

    #[test]
    fn transform_statements_can_append_statements() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (schema_path, out_dir) = two_table_schema(dir.path());

        let cfg = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir)
            .transform_statements(|mut statements| {
                statements.push("CREATE INDEX `users_name_idx` ON `users` (`name`);".to_string());
                statements
            });

        let Output::Generated {
            path,
            statement_count,
            ..
        } = run(&cfg).expect("generation should succeed")
        else {
            panic!("expected a migration to be generated");
        };

        assert_eq!(statement_count, 3);
        let sql = std::fs::read_to_string(path.join("migration.sql")).expect("read migration.sql");
        assert!(sql.contains("users_name_idx"), "{sql}");
    }

    #[test]
    fn transform_statements_emptying_the_plan_reports_no_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (schema_path, out_dir) = two_table_schema(dir.path());

        let cfg = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir)
            .transform_statements(|_| Vec::new());

        assert_eq!(run(&cfg).expect("run"), Output::NoChanges);
        assert!(
            !out_dir.exists(),
            "an emptied plan must not write a migration folder"
        );
    }

    #[test]
    fn config_without_transform_is_unchanged() {
        let cfg = Config::new(Dialect::SQLite);
        assert_eq!(
            cfg.apply_transform(vec!["SELECT 1".to_string()]),
            vec!["SELECT 1".to_string()]
        );
        // The boxed callback must not break Config's derived Debug/Clone.
        let cloned = cfg.clone().transform_statements(|s| s);
        assert!(format!("{cloned:?}").contains("statement transform"));
    }

    #[test]
    fn typed_rebuild_plan_rejects_statement_transform() {
        let plan = SqliteRebuildDataPlan {
            predecessor_snapshot_id: uuid::Uuid::new_v4(),
            tables: Vec::new(),
        };
        let cfg = Config::new(Dialect::SQLite)
            .file("schema.rs")
            .sqlite_rebuild_data_plan(plan)
            .transform_statements(|statements| statements);

        assert!(matches!(
            run(&cfg),
            Err(BuildError::SqliteRebuildDataTransformConflict)
        ));
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

[migrations]
sqliteRebuildDataPlan = "plans/rebuild-data.json"

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
        assert!(
            cfg.watch_targets()
                .contains(&dir.path().join("plans/rebuild-data.json")),
            "checked-in rebuild plan must be resolved relative to and watched with its config"
        );
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

    #[test]
    fn watch_targets_include_out_dir_and_schema_files() {
        let cfg = Config::new(Dialect::SQLite)
            .file("src/schema.rs")
            .file("src/posts.rs")
            .out("./drizzle");

        let targets = cfg.watch_targets();

        assert!(
            targets.contains(&PathBuf::from("src/schema.rs"))
                && targets.contains(&PathBuf::from("src/posts.rs")),
            "schema files must be watched: {targets:?}"
        );
        // run() diffs against the snapshot chain under out_dir, so deleting a
        // migration folder has to retrigger build.rs; without this watch cargo
        // replays the cached script output and never regenerates it.
        assert!(
            targets.contains(&PathBuf::from("./drizzle")),
            "out_dir must be watched: {targets:?}"
        );
    }

    #[test]
    fn watch_targets_include_the_toml_config_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("drizzle.config.toml");
        std::fs::write(
            &cfg_path,
            r#"
dialect = "sqlite"
schema = "src/schema.rs"
out = "./migrations-out"
"#,
        )
        .expect("write config");

        let cfg = Config::from_toml(&cfg_path).expect("load toml");
        let targets = cfg.watch_targets();

        assert!(
            targets.contains(&cfg_path),
            "config path must be watched: {targets:?}"
        );
        assert!(
            targets.contains(&PathBuf::from("src/schema.rs")),
            "schema files must be watched: {targets:?}"
        );
        assert!(
            targets.contains(&PathBuf::from("./migrations-out")),
            "out_dir must be watched: {targets:?}"
        );
    }
}
