//! Loading and validating `drizzle.config.toml`.
//!
//! A config holds either one database (top-level `dialect`, `schema`, ...)
//! or several under `[databases.<name>]`. Field names follow drizzle-kit, so
//! a drizzle-kit config translates directly.

pub use drizzle_types::{Casing, ConfigValue, ConfigValueError};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default config file name, looked up in the current directory.
pub const CONFIG_FILE: &str = "drizzle.config.toml";

/// Casing for Rust identifiers written by `introspect` / `pull`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, JsonSchema)]
pub enum IntrospectCasing {
    /// Convert database names to camelCase
    #[default]
    #[serde(rename = "camel")]
    Camel,
    /// Preserve original database names
    #[serde(rename = "preserve")]
    Preserve,
}

impl IntrospectCasing {
    /// Returns the config spelling (`"camel"` or `"preserve"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Camel => "camel",
            Self::Preserve => "preserve",
        }
    }
}

impl std::fmt::Display for IntrospectCasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for IntrospectCasing {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "camel" | "camelCase" => Ok(Self::Camel),
            "preserve" => Ok(Self::Preserve),
            _ => Err(format!(
                "invalid introspect casing '{s}', expected 'camel' or 'preserve'"
            )),
        }
    }
}

/// The `[introspect]` config section.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct IntrospectConfig {
    /// Casing mode for introspected identifiers
    #[serde(default)]
    pub casing: IntrospectCasing,
}

// ============================================================================
// Entities Filter (matching drizzle-kit)
// ============================================================================

/// The `entities.roles` filter (PostgreSQL).
///
/// Either a boolean (`true` = all user roles, `false` = none) or a table
/// with `provider`, `include`, and `exclude` lists.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RolesFilter {
    /// Simple boolean: true = include all user roles, false = exclude all
    Bool(bool),
    /// Detailed configuration
    Config {
        /// Provider preset (e.g., "supabase", "neon") - excludes provider-specific roles
        #[serde(default)]
        provider: Option<String>,
        /// Explicit list of role names to include
        #[serde(default)]
        include: Option<Vec<String>>,
        /// Explicit list of role names to exclude
        #[serde(default)]
        exclude: Option<Vec<String>>,
    },
}

impl Default for RolesFilter {
    fn default() -> Self {
        Self::Bool(false)
    }
}

impl RolesFilter {
    /// Returns `false` for `roles = false`, `true` otherwise.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Config { .. } => true,
        }
    }

    /// Returns `true` if `role_name` passes this filter.
    #[must_use]
    pub fn should_include(&self, role_name: &str) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Config {
                provider,
                include,
                exclude,
            } => {
                // Check provider exclusions
                if let Some(p) = provider
                    && is_provider_role(p, role_name)
                {
                    return false;
                }
                // Check explicit exclude list
                if let Some(excl) = exclude
                    && excl.iter().any(|e| e == role_name)
                {
                    return false;
                }
                // Check explicit include list (if specified, only include those)
                if let Some(incl) = include {
                    return incl.iter().any(|i| i == role_name);
                }
                true
            }
        }
    }
}

/// Check if a role belongs to a provider's built-in roles
fn is_provider_role(provider: &str, role_name: &str) -> bool {
    match provider {
        "supabase" => matches!(
            role_name,
            "anon"
                | "authenticated"
                | "service_role"
                | "supabase_admin"
                | "supabase_auth_admin"
                | "supabase_storage_admin"
                | "dashboard_user"
                | "supabase_replication_admin"
                | "supabase_read_only_user"
                | "supabase_realtime_admin"
                | "supabase_functions_admin"
                | "postgres"
                | "pgbouncer"
                | "pgsodium_keyholder"
                | "pgsodium_keyiduser"
                | "pgsodium_keymaker"
        ),
        "neon" => matches!(
            role_name,
            "neon_superuser" | "cloud_admin" | "authenticated" | "anonymous"
        ),
        _ => false,
    }
}

/// The `entities` filter: which database entities `push` and `pull` touch.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct EntitiesFilter {
    /// Roles filter (`PostgreSQL` only)
    #[serde(default)]
    pub roles: RolesFilter,
}

// ============================================================================
// Extensions Filter (PostgreSQL only)
// ============================================================================

/// A PostgreSQL extension for `extensionsFilters`; its objects are skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Extension {
    /// `PostGIS` spatial extension
    Postgis,
}

impl Extension {
    /// Returns the config spelling, e.g. `"postgis"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Postgis => "postgis",
        }
    }
}

impl std::fmt::Display for Extension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// Dialect
// ============================================================================

/// The `dialect` config value.
///
/// `turso` is SQLite with libSQL/Turso drivers; `postgres` is accepted as an
/// alias for `postgresql`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    /// `sqlite` (the default).
    #[default]
    Sqlite,
    /// `postgresql` (or `postgres`).
    #[serde(alias = "postgres")]
    Postgresql,
    /// `mysql`.
    Mysql,
    /// `turso`: SQLite via libSQL or Turso.
    Turso,
}

impl Dialect {
    /// All accepted spellings, excluding aliases.
    pub const ALL: &'static [&'static str] = &["sqlite", "postgresql", "mysql", "turso"];

    /// Returns the config spelling, e.g. `"postgresql"`.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgresql => "postgresql",
            Self::Mysql => "mysql",
            Self::Turso => "turso",
        }
    }

    /// Returns the SQL dialect used for generation (`Turso` maps to SQLite).
    #[inline]
    #[must_use]
    pub const fn to_base(self) -> drizzle_types::Dialect {
        match self {
            Self::Sqlite | Self::Turso => drizzle_types::Dialect::SQLite,
            Self::Postgresql => drizzle_types::Dialect::PostgreSQL,
            Self::Mysql => drizzle_types::Dialect::MySQL,
        }
    }
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Dialect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sqlite" => Ok(Self::Sqlite),
            "postgresql" | "postgres" => Ok(Self::Postgresql),
            "mysql" => Ok(Self::Mysql),
            "turso" => Ok(Self::Turso),
            _ => Err(format!(
                "invalid dialect '{}', expected one of: {}",
                s,
                Self::ALL.join(", ")
            )),
        }
    }
}

impl From<Dialect> for drizzle_types::Dialect {
    #[inline]
    fn from(d: Dialect) -> Self {
        d.to_base()
    }
}

// ============================================================================
// Driver
// ============================================================================

/// The `driver` config value: which Rust driver the CLI connects with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Driver {
    /// rusqlite - synchronous `SQLite` driver
    Rusqlite,
    /// libsql - `LibSQL` driver (local embedded)
    Libsql,
    /// turso - Turso cloud driver (remote)
    Turso,
    /// postgres-sync - synchronous `PostgreSQL` driver
    PostgresSync,
    /// tokio-postgres - async `PostgreSQL` driver
    TokioPostgres,
    /// mysql-sync - blocking MySQL driver
    MysqlSync,
    /// mysql-async - Tokio MySQL driver
    MysqlAsync,
    /// d1-http - Cloudflare D1 over the HTTP API
    ///
    /// Targets a remote D1 database via the Cloudflare REST API. Requires
    /// `accountId`, `databaseId`, and `token` in `dbCredentials`. For deploying
    /// to a Worker binding at runtime use the `d1` driver feature on the
    /// drizzle crate itself — this CLI driver is for schema ops (generate /
    /// push / pull / migrate) against a live D1 instance from your dev box.
    D1Http,
    /// durable-sqlite - Cloudflare Durable Objects `SQLite` storage
    ///
    /// DOs run `SQLite` embedded inside the Worker runtime. There's no remote
    /// endpoint to push to from the CLI, so this driver is schema-only:
    /// `generate` produces SQL migrations and a bundled `migrations.js` index
    /// (like drizzle-kit's `bundle: true`) that the Worker imports at build
    /// time to apply migrations inside `DurableObject::new()`.
    DurableSqlite,
    /// aws-data-api - AWS RDS Data API (Aurora Serverless `PostgreSQL`)
    ///
    /// Runs SQL through the AWS RDS Data API instead of a direct TCP
    /// connection. Requires `database`, `secretArn` (AWS Secrets Manager ARN
    /// holding the DB password), and `resourceArn` (Aurora cluster ARN) in
    /// `dbCredentials`. The AWS region comes from the standard SDK chain
    /// (env vars, `~/.aws/config`, EC2/ECS metadata) — drizzle-kit takes no
    /// region field and we match that.
    ///
    /// At the Rust layer this would route through the `aws-sdk-rdsdata` crate,
    /// which isn't yet wired into drizzle-rs — this driver is currently
    /// recognized by the CLI for config parity, but operations return a
    /// pointed `UnsupportedForDriver` error.
    AwsDataApi,
}

impl Driver {
    /// All accepted spellings.
    pub const ALL: &'static [&'static str] = &[
        "rusqlite",
        "libsql",
        "turso",
        "postgres-sync",
        "tokio-postgres",
        "mysql-sync",
        "mysql-async",
        "d1-http",
        "durable-sqlite",
        "aws-data-api",
    ];

    /// Returns the config spelling, e.g. `"tokio-postgres"`.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rusqlite => "rusqlite",
            Self::Libsql => "libsql",
            Self::Turso => "turso",
            Self::PostgresSync => "postgres-sync",
            Self::TokioPostgres => "tokio-postgres",
            Self::MysqlSync => "mysql-sync",
            Self::MysqlAsync => "mysql-async",
            Self::D1Http => "d1-http",
            Self::DurableSqlite => "durable-sqlite",
            Self::AwsDataApi => "aws-data-api",
        }
    }

    /// Returns the drivers allowed for `dialect`.
    #[must_use]
    pub const fn valid_for(dialect: Dialect) -> &'static [Self] {
        match dialect {
            // D1 and Durable Objects are both SQLite-dialect — they only differ
            // in how you reach the database at runtime, so the generator/parser
            // path is identical to plain rusqlite.
            Dialect::Sqlite => &[Self::Rusqlite, Self::D1Http, Self::DurableSqlite],
            Dialect::Turso => &[Self::Libsql, Self::Turso],
            Dialect::Postgresql => &[Self::PostgresSync, Self::TokioPostgres, Self::AwsDataApi],
            Dialect::Mysql => &[Self::MysqlSync, Self::MysqlAsync],
        }
    }

    /// Returns `true` if this driver is allowed for `dialect`.
    #[inline]
    #[must_use]
    pub const fn is_valid_for(self, dialect: Dialect) -> bool {
        matches!(
            (self, dialect),
            (
                Self::Rusqlite | Self::D1Http | Self::DurableSqlite,
                Dialect::Sqlite
            ) | (Self::Libsql | Self::Turso, Dialect::Turso)
                | (
                    Self::PostgresSync | Self::TokioPostgres | Self::AwsDataApi,
                    Dialect::Postgresql
                )
                | (Self::MysqlSync | Self::MysqlAsync, Dialect::Mysql)
        )
    }

    /// Returns `true` for drivers the CLI cannot connect through, so only
    /// `generate`-style commands work (currently `durable-sqlite`, which runs
    /// inside the Workers runtime).
    #[inline]
    #[must_use]
    pub const fn is_codegen_only(self) -> bool {
        matches!(self, Self::DurableSqlite)
    }
}

impl std::fmt::Display for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Driver {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rusqlite" => Ok(Self::Rusqlite),
            "libsql" => Ok(Self::Libsql),
            "turso" => Ok(Self::Turso),
            "postgres-sync" => Ok(Self::PostgresSync),
            "tokio-postgres" => Ok(Self::TokioPostgres),
            "mysql-sync" => Ok(Self::MysqlSync),
            "mysql-async" => Ok(Self::MysqlAsync),
            "d1-http" => Ok(Self::D1Http),
            "durable-sqlite" => Ok(Self::DurableSqlite),
            "aws-data-api" => Ok(Self::AwsDataApi),
            _ => Err(format!(
                "invalid driver '{}', expected one of: {}",
                s,
                Self::ALL.join(", ")
            )),
        }
    }
}

impl std::str::FromStr for Extension {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "postgis" => Ok(Self::Postgis),
            _ => Err(format!(
                "invalid extension filter '{s}', expected 'postgis'"
            )),
        }
    }
}

// ============================================================================
// Credentials
// ============================================================================

/// Resolved `dbCredentials`, with env vars read and values validated.
///
/// `Debug` output redacts secrets.
#[derive(Clone)]
pub enum Credentials {
    /// Local `SQLite` file
    Sqlite { path: Box<str> },

    /// Turso/LibSQL
    Turso {
        url: Box<str>,
        auth_token: Option<Box<str>>,
    },

    /// `PostgreSQL`
    Postgres(PostgresCreds),

    /// MySQL
    MySQL(MySQLCreds),

    /// Cloudflare D1 over the HTTP API.
    ///
    /// Used by the CLI to hit the Cloudflare REST endpoint for schema ops
    /// (push/pull/migrate). The drizzle runtime itself uses the worker
    /// `D1Database` binding — not these credentials.
    D1 {
        account_id: Box<str>,
        database_id: Box<str>,
        token: Box<str>,
    },

    /// AWS RDS Data API (Aurora Serverless `PostgreSQL`).
    ///
    /// The region isn't stored here — the AWS SDK pulls it from the standard
    /// credential chain (env vars, `~/.aws/config`, instance metadata). This
    /// matches drizzle-kit's TypeScript config exactly.
    AwsDataApi {
        database: Box<str>,
        secret_arn: Box<str>,
        resource_arn: Box<str>,
    },
}

/// PostgreSQL credentials: a URL or host fields.
#[derive(Clone)]
pub enum PostgresCreds {
    /// `url = "postgres://..."`.
    Url(Box<str>),
    /// `host`, `port`, `user`, `password`, `database`, `ssl`.
    Host {
        host: Box<str>,
        port: u16,
        user: Option<Box<str>>,
        password: Option<Box<str>>,
        database: Box<str>,
        ssl: PostgresSslMode,
    },
}

/// MySQL credentials. Both Rust adapters consume the same resolved shape;
/// [`Driver`] selects the concrete connection effect later.
#[derive(Clone)]
pub enum MySQLCreds {
    /// `url = "mysql://..."`.
    Url(Box<str>),
    /// `host`, `port`, `user`, `password`, `database`, `ssl`.
    Host {
        host: Box<str>,
        port: u16,
        user: Option<Box<str>>,
        password: Option<Box<str>>,
        database: Box<str>,
        ssl: MySQLSslMode,
    },
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite { path } => f.debug_struct("Sqlite").field("path", path).finish(),
            Self::Turso { url, auth_token } => f
                .debug_struct("Turso")
                .field("url", url)
                .field("auth_token", &auth_token.as_ref().map(|_| "[REDACTED]"))
                .finish(),
            Self::Postgres(credentials) => f.debug_tuple("Postgres").field(credentials).finish(),
            Self::MySQL(credentials) => f.debug_tuple("MySQL").field(credentials).finish(),
            Self::D1 {
                account_id,
                database_id,
                token,
            } => f
                .debug_struct("D1")
                .field("account_id", account_id)
                .field("database_id", database_id)
                .field("token", &(!token.is_empty()).then_some("[REDACTED]"))
                .finish(),
            Self::AwsDataApi {
                database,
                secret_arn,
                resource_arn,
            } => f
                .debug_struct("AwsDataApi")
                .field("database", database)
                .field("secret_arn", secret_arn)
                .field("resource_arn", resource_arn)
                .finish(),
        }
    }
}

impl std::fmt::Debug for PostgresCreds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Url(_) => f.debug_tuple("Url").field(&"[REDACTED]").finish(),
            Self::Host {
                host,
                port,
                user,
                password,
                database,
                ssl,
            } => f
                .debug_struct("Host")
                .field("host", host)
                .field("port", port)
                .field("user", user)
                .field("password", &password.as_ref().map(|_| "[REDACTED]"))
                .field("database", database)
                .field("ssl", ssl)
                .finish(),
        }
    }
}

impl std::fmt::Debug for MySQLCreds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Url(_) => f.debug_tuple("Url").field(&"[REDACTED]").finish(),
            Self::Host {
                host,
                port,
                user,
                password,
                database,
                ssl,
            } => f
                .debug_struct("Host")
                .field("host", host)
                .field("port", port)
                .field("user", user)
                .field("password", &password.as_ref().map(|_| "[REDACTED]"))
                .field("database", database)
                .field("ssl", ssl)
                .finish(),
        }
    }
}

/// PostgreSQL TLS policy (`ssl` in `dbCredentials`, default `Disable`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PostgresSslMode {
    #[default]
    Disable,
    Allow,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

/// MySQL TLS policy accepted by URL/host CLI configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MySQLSslMode {
    #[default]
    Disable,
    Required,
    VerifyCa,
    VerifyIdentity,
}

impl MySQLSslMode {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "false" | "0" | "no" | "off" | "disable" | "disabled" => Ok(Self::Disable),
            "true" | "1" | "yes" | "on" | "require" | "required" => Ok(Self::Required),
            "verify-ca" => Ok(Self::VerifyCa),
            "verify-identity" | "verify-full" => Ok(Self::VerifyIdentity),
            _ => Err(format!("invalid MySQL SSL mode `{value}`")),
        }
    }
}

impl PostgresSslMode {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "false" | "0" | "no" | "off" | "disable" => Ok(Self::Disable),
            "allow" => Ok(Self::Allow),
            "prefer" => Ok(Self::Prefer),
            "true" | "1" | "yes" | "on" | "require" => Ok(Self::Require),
            "verify-ca" => Ok(Self::VerifyCa),
            "verify-full" => Ok(Self::VerifyFull),
            _ => Err(format!("invalid PostgreSQL SSL mode `{value}`")),
        }
    }
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
pub struct PostgresConnectionConfig {
    /// Connection settings for `tokio-postgres` / `postgres`.
    pub config: tokio_postgres::Config,
    /// TLS policy to apply when connecting.
    pub ssl: PostgresSslMode,
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
impl PostgresCreds {
    /// Builds driver connection settings from these credentials.
    ///
    /// # Errors
    ///
    /// Returns the parser's message when a URL is not a valid PostgreSQL
    /// connection string.
    pub fn connection_config(&self) -> Result<PostgresConnectionConfig, String> {
        match self {
            Self::Url(url) => {
                let config = url
                    .parse::<tokio_postgres::Config>()
                    .map_err(|error| error.to_string())?;
                let ssl = match config.get_ssl_mode() {
                    tokio_postgres::config::SslMode::Disable => PostgresSslMode::Disable,
                    tokio_postgres::config::SslMode::Prefer => PostgresSslMode::Prefer,
                    tokio_postgres::config::SslMode::Require => PostgresSslMode::Require,
                    _ => PostgresSslMode::Require,
                };
                Ok(PostgresConnectionConfig { config, ssl })
            }
            Self::Host {
                host,
                port,
                user,
                password,
                database,
                ssl,
            } => {
                let mut config = tokio_postgres::Config::new();
                config
                    .host(host.as_ref())
                    .port(*port)
                    .dbname(database.as_ref());
                if let Some(user) = user {
                    config.user(user.as_ref());
                }
                if let Some(password) = password {
                    config.password(password.as_bytes());
                }
                config.ssl_mode(match ssl {
                    PostgresSslMode::Disable => tokio_postgres::config::SslMode::Disable,
                    PostgresSslMode::Allow | PostgresSslMode::Prefer => {
                        tokio_postgres::config::SslMode::Prefer
                    }
                    PostgresSslMode::Require
                    | PostgresSslMode::VerifyCa
                    | PostgresSslMode::VerifyFull => tokio_postgres::config::SslMode::Require,
                });
                Ok(PostgresConnectionConfig { config, ssl: *ssl })
            }
        }
    }
}

// ============================================================================
// Schema path(s)
// ============================================================================

/// The `schema` config value: one path/glob or a list (default
/// `src/schema.rs`).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Schema {
    /// A single path or glob.
    One(String),
    /// Several paths or globs.
    Many(Vec<String>),
}

impl Default for Schema {
    fn default() -> Self {
        Self::One("src/schema.rs".into())
    }
}

impl Schema {
    /// Iterates over the listed paths/globs.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::One(s) => std::slice::from_ref(s).iter().map(String::as_str),
            Self::Many(v) => v.iter().map(String::as_str),
        }
    }
}

/// A `tablesFilter` / `schemaFilter` value: one pattern or a list.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Filter {
    /// A single pattern.
    One(String),
    /// Several patterns.
    Many(Vec<String>),
}

impl Filter {
    /// Iterates over the patterns.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::One(s) => std::slice::from_ref(s).iter().map(String::as_str),
            Self::Many(v) => v.iter().map(String::as_str),
        }
    }
}

/// The `[migrations]` config section.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MigrationsOpts {
    /// Tracking table name (default `__drizzle_migrations`).
    pub table: Option<String>,
    /// Tracking schema, PostgreSQL only (default `drizzle`).
    pub schema: Option<String>,
    /// Migration folder name prefix (default `timestamp`).
    pub prefix: Option<MigrationPrefix>,
    /// Emit a `migrations.js` index at the root of the migrations output folder.
    ///
    /// Matches drizzle-kit's `bundle: true` behavior. The file statically
    /// `import`s each `migration.sql` so JS bundlers (Metro for Expo/React
    /// Native, Cloudflare Workers for Durable Objects `SQLite`) can embed the
    /// SQL text at build time. Harmless for Rust-only consumers.
    #[serde(default)]
    pub bundle: Option<bool>,
}

/// The `[migrations] prefix` value: how migration folder names start.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MigrationPrefix {
    /// `0000`, `0001`, ...
    Index,
    /// `YYYYMMDDHHMMSS`.
    Timestamp,
    /// Supabase style (`YYYYMMDDHHMMSS`).
    Supabase,
    /// Unix seconds.
    Unix,
    /// No prefix.
    None,
}

// ============================================================================
// Raw credentials (serde parsing helper)
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum RawCreds {
    /// Cloudflare D1 HTTP API credentials — `{ accountId, databaseId, token }`.
    ///
    /// Listed before the more generic `Url` / `Host` variants so that serde's
    /// untagged matching prefers the fully-specified D1 shape when all three
    /// fields are present. (Untagged enums try variants top-to-bottom and pick
    /// the first that deserializes cleanly.)
    D1 {
        #[serde(rename = "accountId")]
        account_id: ConfigValue,
        #[serde(rename = "databaseId")]
        database_id: ConfigValue,
        token: ConfigValue,
    },
    /// AWS RDS Data API credentials — `{ database, secretArn, resourceArn }`.
    ///
    /// Also listed before `Url`/`Host` so the fully-specified shape wins in the
    /// untagged match. Note that `database` is also a field name on the Host
    /// shape, but the combination with `secretArn` + `resourceArn` uniquely
    /// identifies this variant.
    AwsDataApi {
        database: ConfigValue,
        #[serde(rename = "secretArn")]
        secret_arn: ConfigValue,
        #[serde(rename = "resourceArn")]
        resource_arn: ConfigValue,
    },
    Url {
        url: ConfigValue,
        #[serde(default, rename = "authToken")]
        auth_token: Option<ConfigValue>,
    },
    Host {
        host: ConfigValue,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        user: Option<ConfigValue>,
        #[serde(default)]
        password: Option<ConfigValue>,
        database: ConfigValue,
        #[serde(default)]
        ssl: Option<SslVal>,
    },
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum SslVal {
    Bool(bool),
    Str(String),
}

impl SslVal {
    fn postgres_mode(&self) -> Result<PostgresSslMode, String> {
        match self {
            Self::Bool(false) => Ok(PostgresSslMode::Disable),
            Self::Bool(true) => Ok(PostgresSslMode::Require),
            Self::Str(value) => PostgresSslMode::parse(value),
        }
    }

    fn mysql_mode(&self) -> Result<MySQLSslMode, String> {
        match self {
            Self::Bool(false) => Ok(MySQLSslMode::Disable),
            Self::Bool(true) => Ok(MySQLSslMode::Required),
            Self::Str(value) => MySQLSslMode::parse(value),
        }
    }
}

// ============================================================================
// DatabaseConfig - Per-database configuration
// ============================================================================

/// Settings for one database: the top level of a single-database config,
/// or one `[databases.<name>]` table.
///
/// Field names match drizzle-kit (`dbCredentials`, `tablesFilter`, ...).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
    /// Database dialect (required)
    pub dialect: Dialect,

    /// Path(s) to schema file(s) - supports glob patterns
    #[serde(default)]
    pub schema: Schema,

    /// Output directory for migrations (default: "./drizzle")
    #[serde(default = "default_out")]
    pub out: PathBuf,

    /// Whether to use SQL breakpoints in migrations (default: true)
    #[serde(default = "yes")]
    pub breakpoints: bool,

    /// Database driver for Rust connections
    #[serde(default)]
    pub driver: Option<Driver>,

    /// Database credentials
    #[serde(default)]
    db_credentials: Option<RawCreds>,

    /// Table name filter (glob patterns supported)
    #[serde(default)]
    pub tables_filter: Option<Filter>,

    /// Schema name filter (`PostgreSQL` only)
    #[serde(default)]
    pub schema_filter: Option<Filter>,

    /// Extensions filter (`PostgreSQL` only, e.g., `["postgis"]`)
    #[serde(default)]
    pub extensions_filters: Option<Vec<Extension>>,

    /// Entities filter (roles, etc.)
    #[serde(default)]
    pub entities: Option<EntitiesFilter>,

    /// Casing mode for generated code
    #[serde(default)]
    pub casing: Option<Casing>,

    /// Introspection configuration
    #[serde(default)]
    pub introspect: Option<IntrospectConfig>,

    /// Verbose output
    #[serde(default)]
    pub verbose: bool,

    /// Migration table configuration
    #[serde(default)]
    pub migrations: Option<MigrationsOpts>,
}

fn default_out() -> PathBuf {
    PathBuf::from("./drizzle")
}

const fn yes() -> bool {
    true
}

impl DatabaseConfig {
    fn normalize_paths(&mut self, base_dir: &Path) {
        // Resolve `out` relative to the config file directory for predictable behavior,
        // especially when `--config` points at a file outside the current working directory.
        if self.out.is_relative() {
            self.out = base_dir.join(&self.out);
        }

        // Normalize schema patterns:
        // - Resolve relative patterns relative to config dir
        // - Use forward slashes to avoid glob escaping issues on Windows
        let base = base_dir.to_string_lossy().replace('\\', "/");
        let base = base.trim_end_matches('/').to_string();

        let normalize_one = |p: &str| -> String {
            let p_trim = p.trim();
            let is_abs = Path::new(p_trim).is_absolute() || p_trim.starts_with("\\\\");
            let joined = if is_abs || base.is_empty() || base == "." {
                p_trim.to_string()
            } else {
                format!("{base}/{p_trim}")
            };
            joined.replace('\\', "/")
        };

        match &mut self.schema {
            Schema::One(p) => *p = normalize_one(p),
            Schema::Many(v) => {
                for p in v.iter_mut() {
                    *p = normalize_one(p);
                }
            }
        }
    }

    fn validate(&self, name: &str) -> Result<(), Error> {
        // Check driver compatibility
        if let Some(d) = self.driver
            && !d.is_valid_for(self.dialect)
        {
            return Err(Error::InvalidDriver {
                driver: d,
                dialect: self.dialect,
            });
        }

        // Validate credentials if present
        if let Some(ref raw) = self.db_credentials {
            self.validate_creds(raw, name)?;
        }

        // PostgreSQL-only settings
        if self.dialect != Dialect::Postgresql {
            if self.schema_filter.is_some() {
                return Err(Error::InvalidConfig(
                    "schemaFilter is only supported for dialect = \"postgresql\"".into(),
                ));
            }
            if self.extensions_filters.is_some() {
                return Err(Error::InvalidConfig(
                    "extensionsFilters is only supported for dialect = \"postgresql\"".into(),
                ));
            }
            if self.entities.is_some() {
                return Err(Error::InvalidConfig(
                    "entities filter is only supported for dialect = \"postgresql\"".into(),
                ));
            }
        }

        Ok(())
    }

    fn validate_creds(&self, raw: &RawCreds, _name: &str) -> Result<(), Error> {
        let err = |msg: &str| Error::InvalidCredentials(msg.into());

        // Enforce dialect/shape pairing. Without this, serde can parse a "host" form for
        // any dialect, and later `credentials()` would silently return None.
        match (self.dialect, raw) {
            (
                Dialect::Postgresql | Dialect::Mysql,
                RawCreds::Host { .. } | RawCreds::Url { .. },
            ) => {}
            (_, RawCreds::Host { .. }) => {
                return Err(err(
                    "host-based dbCredentials are only supported for dialect = \"postgresql\" or \"mysql\"",
                ));
            }
            _ => {}
        }

        // D1-specific shape requires dialect=sqlite AND driver=d1-http. Paired
        // together so users can't accidentally point a rusqlite driver at D1
        // credentials (or vice versa).
        if let RawCreds::D1 { .. } = raw {
            if self.dialect != Dialect::Sqlite {
                return Err(err(
                    "D1 dbCredentials (accountId/databaseId/token) require dialect = \"sqlite\"",
                ));
            }
            if self.driver != Some(Driver::D1Http) {
                return Err(err(
                    "D1 dbCredentials (accountId/databaseId/token) require driver = \"d1-http\"",
                ));
            }
        }

        // Conversely, if the user picked driver = d1-http but didn't supply the
        // D1 shape, flag it early — otherwise `credentials()` would silently
        // return None and the CLI would fail much later with a confusing error.
        if self.driver == Some(Driver::D1Http) && !matches!(raw, RawCreds::D1 { .. }) {
            return Err(err(
                "driver = \"d1-http\" requires dbCredentials with accountId, databaseId, and token",
            ));
        }

        // AWS Data API shape requires dialect=postgresql AND driver=aws-data-api.
        // Matches drizzle-kit's shape exactly: { database, secretArn, resourceArn }.
        // Region isn't part of the config — the AWS SDK resolves it from env.
        if let RawCreds::AwsDataApi { .. } = raw {
            if self.dialect != Dialect::Postgresql {
                return Err(err(
                    "AWS Data API dbCredentials (database/secretArn/resourceArn) require dialect = \"postgresql\"",
                ));
            }
            if self.driver != Some(Driver::AwsDataApi) {
                return Err(err(
                    "AWS Data API dbCredentials (database/secretArn/resourceArn) require driver = \"aws-data-api\"",
                ));
            }
        }

        // And the inverse — driver=aws-data-api must use the AwsDataApi shape.
        if self.driver == Some(Driver::AwsDataApi) && !matches!(raw, RawCreds::AwsDataApi { .. }) {
            return Err(err(
                "driver = \"aws-data-api\" requires dbCredentials with database, secretArn, and resourceArn",
            ));
        }

        // Dialect-specific checks (only for direct values, not env var references)
        match (self.dialect, raw) {
            (
                Dialect::Sqlite,
                RawCreds::Url {
                    auth_token: Some(_),
                    ..
                },
            ) => Err(err(
                "SQLite doesn't support authToken (use dialect = \"turso\")",
            )),
            (
                Dialect::Mysql,
                RawCreds::Url {
                    auth_token: Some(_),
                    ..
                },
            ) => Err(err("MySQL doesn't support authToken")),
            (
                Dialect::Sqlite,
                RawCreds::Url {
                    url: ConfigValue::Inline(url),
                    ..
                },
            ) if url.starts_with("libsql://") => Err(err(
                "libsql:// URLs require dialect = \"turso\" (for local SQLite files, use ./path.db)",
            )),
            (
                Dialect::Sqlite,
                RawCreds::Url {
                    url: ConfigValue::Inline(url),
                    ..
                },
            ) if url.starts_with("http://")
                || url.starts_with("https://")
                || url.starts_with("postgres://")
                || url.starts_with("postgresql://") =>
            {
                Err(err(
                    "SQLite dbCredentials.url must be a local file path (not an http(s)/postgres URL)",
                ))
            }
            (
                Dialect::Turso,
                RawCreds::Url {
                    url: ConfigValue::Inline(url),
                    ..
                },
            ) if !url.starts_with("libsql://") && !url.starts_with("http") => {
                Err(err("Turso URL must start with libsql:// or http(s)://"))
            }
            (
                Dialect::Postgresql,
                RawCreds::Url {
                    url: ConfigValue::Inline(url),
                    ..
                },
            ) if !url.starts_with("postgres") => {
                Err(err("PostgreSQL URL must start with postgres://"))
            }
            (Dialect::Mysql, RawCreds::Host { ssl: Some(ssl), .. }) => ssl
                .mysql_mode()
                .map(|_| ())
                .map_err(Error::InvalidCredentials),
            (
                Dialect::Mysql,
                RawCreds::Url {
                    url: ConfigValue::Inline(url),
                    ..
                },
            ) if !url.starts_with("mysql://") => Err(err("MySQL URL must start with mysql://")),
            _ => Ok(()),
        }
    }

    /// Returns typed credentials, reading any `{ env = "VAR" }` values, or
    /// `None` when `dbCredentials` is absent.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if a referenced environment variable is missing or
    /// invalid, or if the credentials block does not match the configured
    /// dialect.
    pub fn credentials(&self) -> Result<Option<Credentials>, Error> {
        let Some(raw) = self.db_credentials.as_ref() else {
            return Ok(None);
        };

        // Helper to resolve an optional ConfigValue
        let resolve_opt = |opt: &Option<ConfigValue>| -> Result<Option<Box<str>>, Error> {
            match opt.as_ref() {
                None => Ok(None),
                Some(e) => Ok(Some(e.resolve()?.into_boxed_str())),
            }
        };

        let creds = match (self.dialect, raw) {
            // Cloudflare D1 HTTP — only valid with dialect=sqlite (enforced by
            // validate_creds). Keeps the driver field out of this arm since
            // validate_creds already guaranteed driver = d1-http.
            (
                Dialect::Sqlite,
                RawCreds::D1 {
                    account_id,
                    database_id,
                    token,
                },
            ) => Credentials::D1 {
                account_id: account_id.resolve()?.into_boxed_str(),
                database_id: database_id.resolve()?.into_boxed_str(),
                token: token.resolve()?.into_boxed_str(),
            },
            // AWS RDS Data API — only valid with dialect=postgresql (enforced
            // by validate_creds).
            (
                Dialect::Postgresql,
                RawCreds::AwsDataApi {
                    database,
                    secret_arn,
                    resource_arn,
                },
            ) => Credentials::AwsDataApi {
                database: database.resolve()?.into_boxed_str(),
                secret_arn: secret_arn.resolve()?.into_boxed_str(),
                resource_arn: resource_arn.resolve()?.into_boxed_str(),
            },
            // SQLite
            (Dialect::Sqlite, RawCreds::Url { url, .. }) => Credentials::Sqlite {
                path: url.resolve()?.into_boxed_str(),
            },
            // Turso
            (Dialect::Turso, RawCreds::Url { url, auth_token }) => Credentials::Turso {
                url: url.resolve()?.into_boxed_str(),
                auth_token: resolve_opt(auth_token)?,
            },
            // PostgreSQL URL
            (Dialect::Postgresql, RawCreds::Url { url, .. }) => {
                Credentials::Postgres(PostgresCreds::Url(url.resolve()?.into_boxed_str()))
            }
            // MySQL URL
            (
                Dialect::Mysql,
                RawCreds::Url {
                    url,
                    auth_token: None,
                },
            ) => Credentials::MySQL(MySQLCreds::Url(url.resolve()?.into_boxed_str())),
            // PostgreSQL Host
            (
                Dialect::Postgresql,
                RawCreds::Host {
                    host,
                    port,
                    user,
                    password,
                    database,
                    ssl,
                },
            ) => Credentials::Postgres(PostgresCreds::Host {
                host: host.resolve()?.into_boxed_str(),
                port: port.unwrap_or(5432),
                user: resolve_opt(user)?,
                password: resolve_opt(password)?,
                database: database.resolve()?.into_boxed_str(),
                ssl: ssl
                    .as_ref()
                    .map_or(Ok(PostgresSslMode::Disable), SslVal::postgres_mode)
                    .map_err(Error::InvalidCredentials)?,
            }),
            // MySQL Host
            (
                Dialect::Mysql,
                RawCreds::Host {
                    host,
                    port,
                    user,
                    password,
                    database,
                    ssl,
                },
            ) => Credentials::MySQL(MySQLCreds::Host {
                host: host.resolve()?.into_boxed_str(),
                port: port.unwrap_or(3306),
                user: resolve_opt(user)?,
                password: resolve_opt(password)?,
                database: database.resolve()?.into_boxed_str(),
                ssl: ssl
                    .as_ref()
                    .map_or(Ok(MySQLSslMode::Disable), SslVal::mysql_mode)
                    .map_err(Error::InvalidCredentials)?,
            }),
            _ => return Ok(None),
        };

        Ok(Some(creds))
    }

    /// Returns the migrations folder (`out`).
    #[inline]
    #[must_use]
    pub fn migrations_dir(&self) -> &Path {
        &self.out
    }

    /// Returns `out/meta`, where legacy drizzle-kit layouts keep the journal.
    #[inline]
    #[must_use]
    pub fn meta_dir(&self) -> PathBuf {
        self.out.join("meta")
    }

    /// Returns the legacy `out/meta/_journal.json` path.
    #[inline]
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Returns the `schema` paths joined for display.
    #[must_use]
    pub fn schema_display(&self) -> String {
        match &self.schema {
            Schema::One(s) => s.clone(),
            Schema::Many(v) => v.join(", "),
        }
    }

    /// Resolves `schema` paths and globs to files.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if a listed path is missing, if a glob pattern is
    /// invalid, fails to expand or matches no file (see
    /// [`resolve_schema_patterns`]), or if nothing is listed at all.
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, Error> {
        let files = resolve_schema_patterns(self.schema.iter())?;
        if files.is_empty() {
            return Err(Error::NoSchemaFiles(self.schema_display()));
        }
        Ok(files)
    }

    /// Returns `casing`, or `camelCase` when unset.
    #[inline]
    #[must_use]
    pub fn effective_casing(&self) -> Casing {
        self.casing.unwrap_or_default()
    }

    /// Returns `introspect.casing`, or `camel` when unset.
    #[inline]
    #[must_use]
    pub fn effective_introspect_casing(&self) -> IntrospectCasing {
        self.introspect
            .as_ref()
            .map(|i| i.casing)
            .unwrap_or_default()
    }

    /// Returns the `entities` filter, or the default when unset.
    #[inline]
    #[must_use]
    pub fn effective_entities(&self) -> EntitiesFilter {
        self.entities.clone().unwrap_or_default()
    }

    /// Returns `true` if `role_name` passes the `entities.roles` filter
    /// (`false` when no `entities` filter is set).
    #[must_use]
    pub fn should_include_role(&self, role_name: &str) -> bool {
        self.entities
            .as_ref()
            .is_some_and(|e| e.roles.should_include(role_name))
    }

    /// Returns `true` if the `entities.roles` filter is enabled.
    #[must_use]
    pub fn roles_enabled(&self) -> bool {
        self.entities.as_ref().is_some_and(|e| e.roles.is_enabled())
    }

    /// Returns the `extensionsFilters` list (PostgreSQL only; empty when
    /// unset).
    #[must_use]
    pub fn extensions(&self) -> &[Extension] {
        self.extensions_filters.as_deref().unwrap_or(&[])
    }

    /// Returns `true` if `ext` is in `extensionsFilters`.
    #[must_use]
    pub fn has_extension(&self, ext: Extension) -> bool {
        self.extensions_filters
            .as_ref()
            .is_some_and(|v| v.contains(&ext))
    }

    /// Returns the migrations tracking table (default `__drizzle_migrations`).
    #[must_use]
    pub fn migrations_table(&self) -> &str {
        self.migrations
            .as_ref()
            .and_then(|m| m.table.as_deref())
            .unwrap_or("__drizzle_migrations")
    }

    /// Returns the migrations tracking schema (PostgreSQL only, default
    /// `drizzle`).
    #[must_use]
    pub fn migrations_schema(&self) -> &str {
        self.migrations
            .as_ref()
            .and_then(|m| m.schema.as_deref())
            .unwrap_or("drizzle")
    }

    /// Should a bundled `migrations.js` index be emitted alongside `migration.sql`?
    ///
    /// Resolution order:
    /// 1. Explicit `[migrations] bundle = true/false` in the config wins.
    /// 2. Otherwise, auto-enable for `driver = "durable-sqlite"` since Durable
    ///    Objects need the JS index to import migrations at Worker build time
    ///    (there's no other way to ship SQL into a DO).
    /// 3. Otherwise, default to `false`.
    #[must_use]
    pub fn bundle_enabled(&self) -> bool {
        if let Some(explicit) = self.migrations.as_ref().and_then(|m| m.bundle) {
            return explicit;
        }
        matches!(self.driver, Some(Driver::DurableSqlite))
    }
}

// ============================================================================
// Main Configuration - Wrapper for single/multi-database modes
// ============================================================================

/// Internal format for multi-database config
#[derive(Debug, Clone, Deserialize)]
struct MultiDbConfig {
    databases: HashMap<String, DatabaseConfig>,
}

/// A loaded `drizzle.config.toml`.
///
/// Holds one database or several named ones:
///
/// Single database:
/// ```toml
/// dialect = "sqlite"
/// [dbCredentials]
/// url = "./dev.db"
/// ```
///
/// Multiple databases:
/// ```toml
/// [databases.dev]
/// dialect = "sqlite"
/// [databases.dev.dbCredentials]
/// url = "./dev.db"
///
/// [databases.prod]
/// dialect = "postgresql"
/// [databases.prod.dbCredentials]
/// url = { env = "DATABASE_URL" }
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Named database configurations
    databases: HashMap<String, DatabaseConfig>,
    /// Whether this is a single-database config (for backwards compat)
    is_single: bool,
}

/// Name given to the database of a single-database config.
pub const DEFAULT_DB: &str = "default";

impl Config {
    /// Loads [`CONFIG_FILE`] from the current directory.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the file cannot be read, is not valid TOML for
    /// the config, or fails validation.
    pub fn load() -> Result<Self, Error> {
        Self::load_from(Path::new(CONFIG_FILE))
    }

    /// Loads a config file from `path`.
    ///
    /// Relative paths inside the file (`schema`, `out`, ...) are resolved
    /// against the file's folder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use drizzle_cli::{Config, Dialect};
    ///
    /// let dir = std::env::temp_dir().join("drizzle-cli-doc-load-from");
    /// std::fs::create_dir_all(&dir)?;
    /// let path = dir.join("drizzle.config.toml");
    /// std::fs::write(&path, r#"
    /// dialect = "sqlite"
    /// schema = "src/schema.rs"
    /// out = "./drizzle"
    ///
    /// [dbCredentials]
    /// url = "./dev.db"
    /// "#)?;
    ///
    /// let config = Config::load_from(&path)?;
    /// assert!(config.is_single_database());
    /// assert_eq!(config.dialect(), Dialect::Sqlite);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if `path` does not exist, [`Error::Io`] for
    /// other read errors, [`Error::Parse`] for invalid TOML, and other
    /// [`Error`] variants when validation fails (for example a driver that
    /// does not match the dialect).
    pub fn load_from(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotFound(path.into())
            } else {
                Error::Io(path.into(), e)
            }
        })?;

        Self::load_from_str(&content, path)
    }

    /// Parses config text; `path` is used for errors and relative paths.
    fn load_from_str(content: &str, path: &Path) -> Result<Self, Error> {
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        // Try multi-database format first
        if let Ok(multi) = toml::from_str::<MultiDbConfig>(content)
            && !multi.databases.is_empty()
        {
            let mut config = Self {
                databases: multi.databases,
                is_single: false,
            };
            for db in config.databases.values_mut() {
                db.normalize_paths(base_dir);
            }
            config.validate()?;
            return Ok(config);
        }

        // Fall back to single-database format
        let db_config: DatabaseConfig =
            toml::from_str(content).map_err(|e| Error::Parse(path.into(), e))?;

        let mut databases = HashMap::new();
        databases.insert(DEFAULT_DB.to_string(), db_config);

        let mut config = Self {
            databases,
            is_single: true,
        };
        for db in config.databases.values_mut() {
            db.normalize_paths(base_dir);
        }
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), Error> {
        for (name, db) in &self.databases {
            db.validate(name)?;
        }
        Ok(())
    }

    /// Returns `true` for a single-database config (no `[databases.*]`).
    #[must_use]
    pub const fn is_single_database(&self) -> bool {
        self.is_single
    }

    /// Returns the configured database names (`"default"` for a
    /// single-database config).
    pub fn database_names(&self) -> impl Iterator<Item = &str> {
        self.databases.keys().map(String::as_str)
    }

    /// Returns the database config named `name`.
    ///
    /// If name is `None`, returns the default/only database.
    /// For single-db configs, any name or `None` returns the single database.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoDatabases`] if the config has no databases,
    /// [`Error::DatabaseNotFound`] if `name` does not match any configured
    /// database, or [`Error::DatabaseRequired`] if multiple databases exist
    /// and no name was supplied.
    pub fn database(&self, name: Option<&str>) -> Result<&DatabaseConfig, Error> {
        name.map_or_else(
            || {
                // Get default
                if self.is_single {
                    self.databases.get(DEFAULT_DB).ok_or(Error::NoDatabases)
                } else if self.databases.len() == 1 {
                    self.databases.values().next().ok_or(Error::NoDatabases)
                } else {
                    Err(Error::DatabaseRequired(
                        self.databases.keys().cloned().collect(),
                    ))
                }
            },
            |name| {
                if self.is_single {
                    // For single-db config, accept any name
                    self.databases.get(DEFAULT_DB).ok_or(Error::NoDatabases)
                } else {
                    self.databases
                        .get(name)
                        .ok_or_else(|| Error::DatabaseNotFound(name.to_string()))
                }
            },
        )
    }

    /// Returns the only database, or an error when there are several.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::database`] invoked with `None`.
    pub fn default_database(&self) -> Result<&DatabaseConfig, Error> {
        self.database(None)
    }

    // ========================================================================
    // Backwards compatibility - delegate to default database
    // ========================================================================

    /// Returns the default database's dialect (single-database shortcut).
    #[must_use]
    pub fn dialect(&self) -> Dialect {
        self.default_database()
            .map(|d| d.dialect)
            .unwrap_or_default()
    }

    /// Returns the default database's credentials (single-database shortcut).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if resolving the default database or its credentials
    /// fails (see [`Self::default_database`] and [`DatabaseConfig::credentials`]).
    pub fn credentials(&self) -> Result<Option<Credentials>, Error> {
        self.default_database()?.credentials()
    }

    /// Returns the default database's migrations folder (single-database
    /// shortcut).
    #[must_use]
    pub fn migrations_dir(&self) -> &Path {
        self.default_database()
            .map_or_else(|_| Path::new("./drizzle"), |d| d.migrations_dir())
    }

    /// Returns the default database's legacy `meta/_journal.json` path
    /// (single-database shortcut).
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.default_database().map_or_else(
            |_| PathBuf::from("./drizzle/meta/_journal.json"),
            DatabaseConfig::journal_path,
        )
    }

    /// Returns the default database's schema paths for display
    /// (single-database shortcut).
    #[must_use]
    pub fn schema_display(&self) -> String {
        self.default_database()
            .map_or_else(|_| "src/schema.rs".into(), DatabaseConfig::schema_display)
    }

    /// Returns the default database's resolved schema files
    /// (single-database shortcut).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if resolving the default database fails or if
    /// resolving its schema files fails (see [`DatabaseConfig::schema_files`]).
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, Error> {
        self.default_database()?.schema_files()
    }

    /// Returns the default database's base SQL dialect (Turso maps to
    /// SQLite).
    #[must_use]
    pub fn base_dialect(&self) -> drizzle_types::Dialect {
        self.dialect().to_base()
    }
}

/// Resolves schema paths and glob patterns to the files they name.
///
/// Every listed path must exist, and every glob pattern must be valid, read
/// cleanly and match at least one file. Reading fewer schema files than the
/// config lists would make `push` and `generate` treat the missing tables as
/// removed.
///
/// # Errors
///
/// Returns [`Error::SchemaPathNotFound`] for a path that does not exist,
/// [`Error::Glob`] for an invalid pattern, [`Error::GlobRead`] when expanding
/// a pattern fails, and [`Error::SchemaPatternMatchedNothing`] for a pattern
/// that matches no file.
pub fn resolve_schema_patterns<'a>(
    patterns: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();

    for pattern in patterns {
        let pat = pattern.trim();
        let is_glob = pat.contains('*') || pat.contains('?') || pat.contains('[');

        if !is_glob {
            // A direct path; accept either separator on Windows.
            let path = PathBuf::from(pat);
            let normalized = PathBuf::from(pat.replace('\\', "/"));
            if path.is_file() {
                files.push(path);
            } else if normalized.is_file() {
                files.push(normalized);
            } else {
                return Err(Error::SchemaPathNotFound(pat.into()));
            }
            continue;
        }

        // Normalize separators so `\` is not read as an escape.
        let pat_norm = pat.replace('\\', "/");
        let paths = glob::glob(&pat_norm).map_err(|e| Error::Glob(pat.into(), e))?;
        let mut matched = false;
        for entry in paths {
            let path = entry.map_err(|e| Error::GlobRead(pat.into(), e))?;
            // Glob can return directories.
            if path.is_file() {
                files.push(path);
                matched = true;
            }
        }
        if !matched {
            return Err(Error::SchemaPatternMatchedNothing(pat.into()));
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

// ============================================================================
// Errors
// ============================================================================

/// Errors from loading or using a config file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The config file does not exist.
    #[error("config not found: {}", .0.display())]
    NotFound(PathBuf),

    /// The config file could not be read.
    #[error("failed to read {}: {}", .0.display(), .1)]
    Io(PathBuf, #[source] std::io::Error),

    /// The config file is not valid TOML for the config shape.
    #[error("failed to parse {}: {}", .0.display(), .1)]
    Parse(PathBuf, #[source] toml::de::Error),

    /// `driver` does not belong to `dialect`.
    #[error("driver '{driver}' invalid for {dialect} dialect")]
    InvalidDriver {
        /// The configured driver.
        driver: Driver,
        /// The configured dialect.
        dialect: Dialect,
    },

    /// `dbCredentials` is malformed or does not fit the dialect/driver.
    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),

    /// A setting is not allowed, e.g. a PostgreSQL-only filter on SQLite.
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// A `schema` glob pattern is invalid.
    #[error("invalid glob '{0}': {1}")]
    Glob(String, #[source] glob::PatternError),

    /// Expanding a `schema` glob failed.
    #[error("failed to read a path matched by glob '{0}': {1}")]
    GlobRead(String, #[source] glob::GlobError),

    /// A listed `schema` path does not exist.
    #[error("schema path '{0}' does not exist")]
    SchemaPathNotFound(String),

    /// A `schema` glob matched no files.
    #[error("schema pattern '{0}' matches no file")]
    SchemaPatternMatchedNothing(String),

    /// No schema files are configured.
    #[error("no schema files found: {0}")]
    NoSchemaFiles(String),

    /// An `{ env = "VAR" }` value names an unset variable.
    #[error("environment variable '{0}' not found")]
    EnvNotFound(String),

    /// An `{ env = "VAR" }` value is not valid UTF-8.
    #[error("environment variable '{0}' invalid: {1}")]
    EnvInvalid(String, String),

    /// The config has no databases.
    #[error("no databases configured")]
    NoDatabases,

    /// No database has the requested name.
    #[error("database '{0}' not found")]
    DatabaseNotFound(String),

    /// Several databases are configured and none was chosen with `--db`.
    #[error("multiple databases configured, use --db to specify: {}", .0.join(", "))]
    DatabaseRequired(Vec<String>),
}

impl From<ConfigValueError> for Error {
    fn from(err: ConfigValueError) -> Self {
        match err {
            ConfigValueError::NotPresent(var) => Self::EnvNotFound(var),
            ConfigValueError::NotUnicode(var) => {
                Self::EnvInvalid(var, "contains invalid unicode".into())
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn sqlite() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "sqlite"
            [dbCredentials]
            url = "./dev.db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        assert!(cfg.is_single_database());
        assert!(matches!(
            cfg.credentials().unwrap(),
            Some(Credentials::Sqlite { .. })
        ));
    }

    #[test]
    fn postgres_url() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        assert!(matches!(
            cfg.credentials().unwrap(),
            Some(Credentials::Postgres(PostgresCreds::Url(_)))
        ));
    }

    #[test]
    fn mysql_url_credentials_and_driver_parse() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "mysql"
            driver = "mysql-async"
            [dbCredentials]
            url = "mysql://user:secret@localhost:3306/app"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db = cfg.default_database().unwrap();
        assert_eq!(db.dialect, Dialect::Mysql);
        assert_eq!(db.driver, Some(Driver::MysqlAsync));
        assert!(matches!(
            db.credentials().unwrap(),
            Some(Credentials::MySQL(MySQLCreds::Url(_)))
        ));
    }

    #[test]
    fn mysql_host_credentials_use_mysql_defaults() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "mysql"
            driver = "mysql-sync"
            [dbCredentials]
            host = "db.example.com"
            user = "app"
            password = "secret"
            database = "app_db"
            ssl = "verify-identity"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        match cfg.default_database().unwrap().credentials().unwrap() {
            Some(Credentials::MySQL(MySQLCreds::Host {
                port,
                ssl,
                database,
                ..
            })) => {
                assert_eq!(port, 3306);
                assert_eq!(ssl, MySQLSslMode::VerifyIdentity);
                assert_eq!(database.as_ref(), "app_db");
            }
            other => panic!("unexpected credentials: {other:?}"),
        }
    }

    #[test]
    fn mysql_host_credentials_reject_unrepresentable_preferred_tls() {
        let error = Config::load_from_str(
            r#"
            dialect = "mysql"
            [dbCredentials]
            host = "db.example.com"
            database = "app_db"
            ssl = "preferred"
        "#,
            Path::new("test.toml"),
        )
        .expect_err("host credentials cannot express opportunistic TLS");

        assert!(
            error.to_string().contains("invalid MySQL SSL mode"),
            "{error}"
        );
    }

    #[test]
    fn mysql_rejects_cross_dialect_driver_and_url() {
        let driver_error = Config::load_from_str(
            r#"
            dialect = "mysql"
            driver = "postgres-sync"
            [dbCredentials]
            url = "mysql://localhost/app"
        "#,
            Path::new("test.toml"),
        )
        .expect_err("postgres driver must not target mysql");
        assert!(driver_error.to_string().contains("invalid for mysql"));

        let url_error = Config::load_from_str(
            r#"
            dialect = "mysql"
            [dbCredentials]
            url = "postgres://localhost/app"
        "#,
            Path::new("test.toml"),
        )
        .expect_err("postgres URL must not target mysql");
        assert!(url_error.to_string().contains("must start with mysql://"));
    }

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    #[test]
    fn postgres_host_credentials_are_structured_without_url_encoding() {
        let credentials = PostgresCreds::Host {
            host: "db.example.com".into(),
            port: 5432,
            user: Some("user@tenant".into()),
            password: Some("p:a/ss% word".into()),
            database: "app/db name".into(),
            ssl: PostgresSslMode::VerifyFull,
        };

        let connection = credentials.connection_config().expect("build config");
        assert_eq!(connection.config.get_user(), Some("user@tenant"));
        assert_eq!(connection.config.get_password(), Some(&b"p:a/ss% word"[..]));
        assert_eq!(connection.config.get_dbname(), Some("app/db name"));
        assert_eq!(
            connection.config.get_ssl_mode(),
            tokio_postgres::config::SslMode::Require
        );
        assert_eq!(connection.ssl, PostgresSslMode::VerifyFull);
    }

    #[test]
    fn multi_database() {
        let cfg = Config::load_from_str(
            r#"
            [databases.dev]
            dialect = "sqlite"
            out = "./drizzle/sqlite"
            [databases.dev.dbCredentials]
            url = "./dev.db"

            [databases.prod]
            dialect = "postgresql"
            out = "./drizzle/postgres"
            [databases.prod.dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        assert!(!cfg.is_single_database());
        let names: Vec<_> = cfg.database_names().collect();
        assert!(names.contains(&"dev"));
        assert!(names.contains(&"prod"));

        let dev = cfg.database(Some("dev")).unwrap();
        assert_eq!(dev.dialect, Dialect::Sqlite);

        let prod = cfg.database(Some("prod")).unwrap();
        assert_eq!(prod.dialect, Dialect::Postgresql);
    }

    #[test]
    fn multi_database_requires_selection() {
        let cfg = Config::load_from_str(
            r#"
            [databases.a]
            dialect = "sqlite"
            [databases.b]
            dialect = "postgresql"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        // Should error when no db specified with multiple dbs
        assert!(cfg.database(None).is_err());
    }

    #[test]
    fn env_var_syntax() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [dbCredentials]
            url = { env = "DATABASE_URL" }
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        assert!(cfg.is_single_database());
    }

    #[test]
    fn casing_options() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            casing = "snake_case"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db = cfg.default_database().unwrap();
        assert_eq!(db.effective_casing(), Casing::SnakeCase);

        // Test default (camelCase)
        let cfg2 = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db2 = cfg2.default_database().unwrap();
        assert_eq!(db2.effective_casing(), Casing::CamelCase);
    }

    #[test]
    fn introspect_casing() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [introspect]
            casing = "preserve"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db = cfg.default_database().unwrap();
        assert_eq!(db.effective_introspect_casing(), IntrospectCasing::Preserve);
    }

    #[test]
    fn entities_roles_filter() {
        // Test boolean roles filter
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [entities]
            roles = true
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db = cfg.default_database().unwrap();
        assert!(db.roles_enabled());
        assert!(db.should_include_role("my_role"));

        // Test roles filter with provider
        let cfg2 = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [entities.roles]
            provider = "supabase"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db2 = cfg2.default_database().unwrap();
        assert!(db2.roles_enabled());
        assert!(!db2.should_include_role("anon")); // Supabase built-in
        assert!(db2.should_include_role("my_custom_role"));
    }

    #[test]
    fn extensions_filter() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            extensionsFilters = ["postgis"]
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db = cfg.default_database().unwrap();
        assert!(db.has_extension(Extension::Postgis));
    }

    #[test]
    fn rejects_postgres_only_filters_for_sqlite() {
        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            schemaFilter = ["public"]
            [dbCredentials]
            url = "./dev.db"
        "#,
            Path::new("test.toml"),
        )
        .expect_err("sqlite should reject schemaFilter");
        assert_eq!(
            err.to_string(),
            "invalid config: schemaFilter is only supported for dialect = \"postgresql\""
        );

        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            extensionsFilters = ["postgis"]
            [dbCredentials]
            url = "./dev.db"
        "#,
            Path::new("test.toml"),
        )
        .expect_err("sqlite should reject extensionsFilters");
        assert_eq!(
            err.to_string(),
            "invalid config: extensionsFilters is only supported for dialect = \"postgresql\""
        );
    }

    #[test]
    fn rejects_entities_filter_for_turso() {
        let err = Config::load_from_str(
            r#"
            dialect = "turso"
            [entities]
            roles = true
            [dbCredentials]
            url = "libsql://example.turso.io"
        "#,
            Path::new("test.toml"),
        )
        .expect_err("turso should reject entities filter");
        assert_eq!(
            err.to_string(),
            "invalid config: entities filter is only supported for dialect = \"postgresql\""
        );
    }

    #[test]
    fn migrations_config() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [migrations]
            table = "custom_migrations"
            schema = "custom_schema"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db = cfg.default_database().unwrap();
        assert_eq!(db.migrations_table(), "custom_migrations");
        assert_eq!(db.migrations_schema(), "custom_schema");

        // Test defaults
        let cfg2 = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        let db2 = cfg2.default_database().unwrap();
        assert_eq!(db2.migrations_table(), "__drizzle_migrations");
        assert_eq!(db2.migrations_schema(), "drizzle");
    }

    #[test]
    fn resolves_paths_relative_to_config_dir() {
        let tmp = TempDir::new().unwrap();
        let cfg_dir = tmp.path().join("cfg");
        fs::create_dir_all(&cfg_dir).unwrap();

        // Create schema file next to config file.
        let schema_path = cfg_dir.join("schema.rs");
        fs::write(&schema_path, "#[allow(dead_code)]\npub struct X;").unwrap();

        let cfg_path = cfg_dir.join("drizzle.config.toml");
        let cfg = Config::load_from_str(
            r#"
            dialect = "sqlite"
            schema = "schema.rs"
            out = "./drizzle"
            [dbCredentials]
            url = "./dev.db"
        "#,
            &cfg_path,
        )
        .unwrap();

        let db = cfg.default_database().unwrap();
        assert_eq!(db.migrations_dir(), cfg_dir.join("./drizzle").as_path());

        let files = db.schema_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], schema_path);
    }

    #[test]
    fn rejects_host_credentials_for_sqlite() {
        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            [dbCredentials]
            host = "localhost"
            database = "db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "invalid credentials: host-based dbCredentials are only supported for dialect = \"postgresql\" or \"mysql\""
        );
    }

    // ========================================================================
    // Cloudflare: D1 HTTP and Durable Objects SQLite
    // ========================================================================

    #[test]
    fn d1_http_credentials_parse() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "d1-http"
            [dbCredentials]
            accountId = "acc_abc"
            databaseId = "db_xyz"
            token = "tok_123"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        let db = cfg.default_database().unwrap();
        assert_eq!(db.driver, Some(Driver::D1Http));
        match db.credentials().unwrap() {
            Some(Credentials::D1 {
                account_id,
                database_id,
                token,
            }) => {
                assert_eq!(&*account_id, "acc_abc");
                assert_eq!(&*database_id, "db_xyz");
                assert_eq!(&*token, "tok_123");
            }
            other => panic!("expected Credentials::D1, got {other:?}"),
        }
    }

    #[test]
    fn d1_http_credentials_resolve_from_env() {
        // Unique env var names per-test so parallel tests don't collide.
        unsafe {
            std::env::set_var("TEST_D1_ACCT", "env_acct");
            std::env::set_var("TEST_D1_DB", "env_db");
            std::env::set_var("TEST_D1_TOKEN", "env_token");
        }
        let cfg = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "d1-http"
            [dbCredentials]
            accountId = { env = "TEST_D1_ACCT" }
            databaseId = { env = "TEST_D1_DB" }
            token = { env = "TEST_D1_TOKEN" }
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        match cfg.default_database().unwrap().credentials().unwrap() {
            Some(Credentials::D1 {
                account_id,
                database_id,
                token,
            }) => {
                assert_eq!(&*account_id, "env_acct");
                assert_eq!(&*database_id, "env_db");
                assert_eq!(&*token, "env_token");
            }
            other => panic!("expected Credentials::D1, got {other:?}"),
        }
    }

    #[test]
    fn d1_credentials_require_sqlite_dialect() {
        let err = Config::load_from_str(
            r#"
            dialect = "postgresql"
            [dbCredentials]
            accountId = "acc"
            databaseId = "db"
            token = "tok"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("D1 dbCredentials"),
            "expected D1-specific error, got: {err}"
        );
    }

    #[test]
    fn d1_credentials_require_d1_http_driver() {
        // Same SQLite dialect, but driver is rusqlite — should be rejected.
        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "rusqlite"
            [dbCredentials]
            accountId = "acc"
            databaseId = "db"
            token = "tok"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("driver = \"d1-http\""),
            "expected d1-http driver error, got: {err}"
        );
    }

    #[test]
    fn d1_http_driver_requires_d1_credentials() {
        // Driver is d1-http but creds are URL-shaped — should be rejected.
        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "d1-http"
            [dbCredentials]
            url = "./dev.db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("accountId, databaseId, and token"),
            "expected d1-http creds-shape error, got: {err}"
        );
    }

    #[test]
    fn durable_sqlite_no_credentials_ok() {
        // Durable Objects don't need credentials — migrations are applied inside
        // the Worker runtime. Loading without dbCredentials should succeed.
        let cfg = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "durable-sqlite"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        let db = cfg.default_database().unwrap();
        assert_eq!(db.driver, Some(Driver::DurableSqlite));
        assert!(db.credentials().unwrap().is_none());
        // Bundle should auto-enable so migrations.js gets emitted for the Worker.
        assert!(
            db.bundle_enabled(),
            "durable-sqlite should auto-enable bundle"
        );
    }

    #[test]
    fn durable_sqlite_explicit_bundle_false_respected() {
        // Explicit opt-out must override the durable-sqlite auto-enable.
        let cfg = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "durable-sqlite"
            [migrations]
            bundle = false
        "#,
            Path::new("test.toml"),
        )
        .unwrap();
        assert!(!cfg.default_database().unwrap().bundle_enabled());
    }

    #[test]
    fn durable_sqlite_rejects_non_sqlite_dialect() {
        let err = Config::load_from_str(
            r#"
            dialect = "postgresql"
            driver = "durable-sqlite"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("invalid for postgresql"),
            "expected dialect/driver mismatch error, got: {err}"
        );
    }

    #[test]
    fn driver_valid_for_sqlite_includes_cloudflare() {
        let drivers = Driver::valid_for(Dialect::Sqlite);
        assert!(drivers.contains(&Driver::Rusqlite));
        assert!(drivers.contains(&Driver::D1Http));
        assert!(drivers.contains(&Driver::DurableSqlite));
        // D1/DO must not leak into other dialects.
        for drv in [Driver::D1Http, Driver::DurableSqlite] {
            assert!(!drv.is_valid_for(Dialect::Postgresql));
            assert!(!drv.is_valid_for(Dialect::Turso));
        }
    }

    #[test]
    fn driver_is_codegen_only_flag() {
        assert!(Driver::DurableSqlite.is_codegen_only());
        assert!(!Driver::D1Http.is_codegen_only());
        assert!(!Driver::Rusqlite.is_codegen_only());
        assert!(!Driver::AwsDataApi.is_codegen_only());
    }

    // ========================================================================
    // AWS RDS Data API (Aurora Serverless PostgreSQL)
    // ========================================================================

    #[test]
    fn aws_data_api_credentials_parse() {
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            driver = "aws-data-api"
            [dbCredentials]
            database = "mydb"
            secretArn = "arn:aws:secretsmanager:us-east-1:123:secret:db-xyz"
            resourceArn = "arn:aws:rds:us-east-1:123:cluster:my-aurora"
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        let db = cfg.default_database().unwrap();
        assert_eq!(db.driver, Some(Driver::AwsDataApi));
        match db.credentials().unwrap() {
            Some(Credentials::AwsDataApi {
                database,
                secret_arn,
                resource_arn,
            }) => {
                assert_eq!(&*database, "mydb");
                assert!(secret_arn.starts_with("arn:aws:secretsmanager"));
                assert!(resource_arn.starts_with("arn:aws:rds"));
            }
            other => panic!("expected Credentials::AwsDataApi, got {other:?}"),
        }
    }

    #[test]
    fn aws_data_api_credentials_resolve_from_env() {
        unsafe {
            std::env::set_var("TEST_AWS_DB", "envdb");
            std::env::set_var("TEST_AWS_SECRET", "arn:env:secret");
            std::env::set_var("TEST_AWS_RESOURCE", "arn:env:resource");
        }
        let cfg = Config::load_from_str(
            r#"
            dialect = "postgresql"
            driver = "aws-data-api"
            [dbCredentials]
            database = { env = "TEST_AWS_DB" }
            secretArn = { env = "TEST_AWS_SECRET" }
            resourceArn = { env = "TEST_AWS_RESOURCE" }
        "#,
            Path::new("test.toml"),
        )
        .unwrap();

        match cfg.default_database().unwrap().credentials().unwrap() {
            Some(Credentials::AwsDataApi {
                database,
                secret_arn,
                resource_arn,
            }) => {
                assert_eq!(&*database, "envdb");
                assert_eq!(&*secret_arn, "arn:env:secret");
                assert_eq!(&*resource_arn, "arn:env:resource");
            }
            other => panic!("expected Credentials::AwsDataApi, got {other:?}"),
        }
    }

    #[test]
    fn aws_data_api_requires_postgres_dialect() {
        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            [dbCredentials]
            database = "mydb"
            secretArn = "arn:aws:secretsmanager:..."
            resourceArn = "arn:aws:rds:..."
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("AWS Data API dbCredentials"),
            "expected AWS-specific error, got: {err}"
        );
    }

    #[test]
    fn aws_data_api_requires_aws_data_api_driver() {
        // Same postgresql dialect, but driver is tokio-postgres — should be rejected.
        let err = Config::load_from_str(
            r#"
            dialect = "postgresql"
            driver = "tokio-postgres"
            [dbCredentials]
            database = "mydb"
            secretArn = "arn:aws:secretsmanager:..."
            resourceArn = "arn:aws:rds:..."
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("driver = \"aws-data-api\""),
            "expected aws-data-api driver error, got: {err}"
        );
    }

    #[test]
    fn aws_data_api_driver_requires_aws_credentials() {
        // driver = aws-data-api but creds are URL-shaped — should be rejected.
        let err = Config::load_from_str(
            r#"
            dialect = "postgresql"
            driver = "aws-data-api"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("database, secretArn, and resourceArn"),
            "expected aws-data-api creds-shape error, got: {err}"
        );
    }

    #[test]
    fn aws_data_api_rejected_for_non_postgres_dialect() {
        let err = Config::load_from_str(
            r#"
            dialect = "sqlite"
            driver = "aws-data-api"
        "#,
            Path::new("test.toml"),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("invalid for sqlite"),
            "expected dialect/driver mismatch error, got: {err}"
        );
    }

    #[test]
    fn driver_valid_for_postgres_includes_aws_data_api() {
        let drivers = Driver::valid_for(Dialect::Postgresql);
        assert!(drivers.contains(&Driver::PostgresSync));
        assert!(drivers.contains(&Driver::TokioPostgres));
        assert!(drivers.contains(&Driver::AwsDataApi));
        // Must not leak into other dialects.
        assert!(!Driver::AwsDataApi.is_valid_for(Dialect::Sqlite));
        assert!(!Driver::AwsDataApi.is_valid_for(Dialect::Turso));
    }

    #[test]
    fn credentials_debug_redacts_secrets() {
        let postgres = Credentials::Postgres(PostgresCreds::Host {
            host: "localhost".into(),
            port: 5432,
            user: Some("alice".into()),
            password: Some("super-secret-password".into()),
            database: "app".into(),
            ssl: PostgresSslMode::Require,
        });
        let turso = Credentials::Turso {
            url: "libsql://example.turso.io".into(),
            auth_token: Some("super-secret-token".into()),
        };
        let d1 = Credentials::D1 {
            account_id: "account".into(),
            database_id: "database".into(),
            token: "super-secret-d1-token".into(),
        };
        let postgres_url = Credentials::Postgres(PostgresCreds::Url(
            "postgres://alice:super-secret-url-password@localhost/app".into(),
        ));

        let rendered = format!("{postgres:?} {turso:?} {d1:?} {postgres_url:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("super-secret"));
    }

    #[cfg(windows)]
    #[test]
    fn schema_files_accept_backslash_paths() {
        let tmp = TempDir::new().unwrap();
        let cfg_dir = tmp.path().join("cfg");
        fs::create_dir_all(&cfg_dir).unwrap();

        let schema_path = cfg_dir.join("src").join("schema.rs");
        fs::create_dir_all(schema_path.parent().unwrap()).unwrap();
        fs::write(&schema_path, "#[allow(dead_code)]\npub struct X;").unwrap();

        // Write schema path with backslashes (common on Windows).
        let schema_str = schema_path.to_string_lossy().replace('/', "\\");
        // TOML basic strings treat backslash as an escape; double-escape to embed a Windows path.
        let schema_toml = schema_str.replace('\\', "\\\\");
        let cfg_path = cfg_dir.join("drizzle.config.toml");
        let cfg = Config::load_from_str(
            &format!(
                r#"
                dialect = "sqlite"
                schema = "{}"
            "#,
                schema_toml
            ),
            &cfg_path,
        )
        .unwrap();

        let db = cfg.default_database().unwrap();
        let files = db.schema_files().unwrap();
        assert_eq!(files, vec![schema_path]);
    }

    /// Reading fewer schema files than listed would make push and generate
    /// treat the missing tables as dropped, so every listed path must exist
    /// and every pattern must match.
    #[test]
    fn schema_resolution_rejects_any_missing_path_or_empty_pattern() {
        let tmp = TempDir::new().unwrap();
        let present = tmp.path().join("present.rs");
        fs::write(&present, "pub struct Present;").unwrap();
        let present = present.to_string_lossy().replace('\\', "/");
        let missing = tmp
            .path()
            .join("missing.rs")
            .to_string_lossy()
            .replace('\\', "/");
        let empty_glob = format!(
            "{}/nothing/*.rs",
            tmp.path().to_string_lossy().replace('\\', "/")
        );

        assert_eq!(
            resolve_schema_patterns([present.as_str()]).unwrap().len(),
            1
        );
        assert!(matches!(
            resolve_schema_patterns([present.as_str(), missing.as_str()]),
            Err(Error::SchemaPathNotFound(path)) if path == missing
        ));
        assert!(matches!(
            resolve_schema_patterns([present.as_str(), empty_glob.as_str()]),
            Err(Error::SchemaPatternMatchedNothing(pattern)) if pattern == empty_glob
        ));
        assert!(matches!(
            resolve_schema_patterns(["src/[.rs"]),
            Err(Error::Glob(..))
        ));
    }
}
