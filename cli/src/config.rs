//! Configuration for Drizzle CLI
//!
//! Handles loading `drizzle.config.toml` with type-safe credentials.
//! Supports both single-database (legacy) and multi-database configurations.
//!
//! This configuration format is designed to be compatible with drizzle-kit
//! so TypeScript users can use the same config expectations.

pub use drizzle_types::Casing;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "drizzle.config.toml";

/// Casing mode for introspection (pull command)
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

/// Introspection configuration
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct IntrospectConfig {
    /// Casing mode for introspected identifiers
    #[serde(default)]
    pub casing: IntrospectCasing,
}

// ============================================================================
// Entities Filter (matching drizzle-kit)
// ============================================================================

/// Roles filter configuration
///
/// Can be either a boolean (true = include all, false = exclude all)
/// or a detailed configuration with provider/include/exclude lists.
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
    /// Check if roles should be included at all
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Config { .. } => true,
        }
    }

    /// Check if a specific role should be included
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

/// Entities filter configuration
///
/// Controls which database entities are included in push/pull operations.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct EntitiesFilter {
    /// Roles filter (`PostgreSQL` only)
    #[serde(default)]
    pub roles: RolesFilter,
}

// ============================================================================
// Extensions Filter (PostgreSQL only)
// ============================================================================

/// Known `PostgreSQL` extensions that can be filtered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Extension {
    /// `PostGIS` spatial extension
    Postgis,
}

impl Extension {
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
// EnvOr - Environment variable or direct value
// ============================================================================

/// A value that can be either a direct string or an environment variable reference.
///
/// In TOML config, users can write:
/// ```toml
/// url = "postgres://localhost/db"           # Direct value
/// url = { env = "DATABASE_URL" }            # Environment variable
/// ```
#[derive(Debug, Clone)]
pub enum EnvOr {
    /// Direct string value
    Value(String),
    /// Environment variable name to resolve
    Env(String),
}

impl EnvOr {
    /// Resolve the value, looking up environment variable if needed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EnvNotFound`] if this is an [`EnvOr::Env`] pointing to
    /// a variable that is not set in the process environment.
    pub fn resolve(&self) -> Result<String, Error> {
        match self {
            Self::Value(v) => Ok(v.clone()),
            Self::Env(var) => std::env::var(var).map_err(|_| Error::EnvNotFound(var.clone())),
        }
    }

    /// Resolve to an optional value (returns `None` for missing env vars).
    ///
    /// # Errors
    ///
    /// Returns [`Error::EnvInvalid`] if the referenced environment variable
    /// is set but contains non-unicode bytes.
    pub fn resolve_optional(&self) -> Result<Option<String>, Error> {
        match self {
            Self::Value(v) => Ok(Some(v.clone())),
            Self::Env(var) => match std::env::var(var) {
                Ok(v) => Ok(Some(v)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(std::env::VarError::NotUnicode(_)) => Err(Error::EnvInvalid(
                    var.clone(),
                    "contains invalid unicode".into(),
                )),
            },
        }
    }
}

impl JsonSchema for EnvOr {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "EnvOr".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;

        // EnvOr accepts either a plain string or { env: "VAR_NAME" }
        json_schema!({
            "oneOf": [
                generator.subschema_for::<String>(),
                {
                    "type": "object",
                    "properties": {
                        "env": { "type": "string" }
                    },
                    "required": ["env"],
                    "additionalProperties": false
                }
            ]
        })
    }
}

impl<'de> Deserialize<'de> for EnvOr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvOrVisitor;

        impl<'de> Visitor<'de> for EnvOrVisitor {
            type Value = EnvOr;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or { env = \"VAR_NAME\" }")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(EnvOr::Value(value.to_string()))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut env_var: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "env" {
                        env_var = Some(map.next_value()?);
                    } else {
                        return Err(de::Error::unknown_field(&key, &["env"]));
                    }
                }

                env_var
                    .map(EnvOr::Env)
                    .ok_or_else(|| de::Error::missing_field("env"))
            }
        }

        deserializer.deserialize_any(EnvOrVisitor)
    }
}

// ============================================================================
// Dialect
// ============================================================================

/// Database dialect
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    #[default]
    Sqlite,
    #[serde(alias = "postgres")]
    Postgresql,
    Turso,
}

impl Dialect {
    pub const ALL: &'static [&'static str] = &["sqlite", "postgresql", "turso"];

    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgresql => "postgresql",
            Self::Turso => "turso",
        }
    }

    #[inline]
    #[must_use]
    pub const fn to_base(self) -> drizzle_types::Dialect {
        match self {
            Self::Sqlite | Self::Turso => drizzle_types::Dialect::SQLite,
            Self::Postgresql => drizzle_types::Dialect::PostgreSQL,
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

/// Database driver for Rust database connections
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
    pub const ALL: &'static [&'static str] = &[
        "rusqlite",
        "libsql",
        "turso",
        "postgres-sync",
        "tokio-postgres",
        "d1-http",
        "durable-sqlite",
        "aws-data-api",
    ];

    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rusqlite => "rusqlite",
            Self::Libsql => "libsql",
            Self::Turso => "turso",
            Self::PostgresSync => "postgres-sync",
            Self::TokioPostgres => "tokio-postgres",
            Self::D1Http => "d1-http",
            Self::DurableSqlite => "durable-sqlite",
            Self::AwsDataApi => "aws-data-api",
        }
    }

    #[must_use]
    pub const fn valid_for(dialect: Dialect) -> &'static [Self] {
        match dialect {
            // D1 and Durable Objects are both SQLite-dialect — they only differ
            // in how you reach the database at runtime, so the generator/parser
            // path is identical to plain rusqlite.
            Dialect::Sqlite => &[Self::Rusqlite, Self::D1Http, Self::DurableSqlite],
            Dialect::Turso => &[Self::Libsql, Self::Turso],
            Dialect::Postgresql => &[Self::PostgresSync, Self::TokioPostgres, Self::AwsDataApi],
        }
    }

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
        )
    }

    /// True for drivers that only make sense as schema/codegen targets — i.e.
    /// drivers where the CLI has no way to connect to a live DB from the dev
    /// machine (e.g. Durable Objects `SQLite` runs inside the Workers runtime).
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

/// Database credentials - validated and typed
#[derive(Debug, Clone)]
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

/// `PostgreSQL` credentials
#[derive(Debug, Clone)]
pub enum PostgresCreds {
    Url(Box<str>),
    Host {
        host: Box<str>,
        port: u16,
        user: Option<Box<str>>,
        password: Option<Box<str>>,
        database: Box<str>,
        ssl: bool,
    },
}

impl PostgresCreds {
    /// Build connection URL
    #[must_use]
    pub fn connection_url(&self) -> String {
        match self {
            Self::Url(url) => url.to_string(),
            Self::Host {
                host,
                port,
                user,
                password,
                database,
                ..
            } => {
                let auth = match (user, password) {
                    (Some(u), Some(p)) => format!("{u}:{p}@"),
                    (Some(u), None) => format!("{u}@"),
                    _ => String::new(),
                };
                format!("postgres://{auth}{host}:{port}/{database}")
            }
        }
    }
}

// ============================================================================
// Schema path(s)
// ============================================================================

/// Schema path(s)
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Schema {
    One(String),
    Many(Vec<String>),
}

impl Default for Schema {
    fn default() -> Self {
        Self::One("src/schema.rs".into())
    }
}

impl Schema {
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::One(s) => std::slice::from_ref(s).iter().map(String::as_str),
            Self::Many(v) => v.iter().map(String::as_str),
        }
    }
}

/// Filter (single or multiple values)
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Filter {
    One(String),
    Many(Vec<String>),
}

impl Filter {
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::One(s) => std::slice::from_ref(s).iter().map(String::as_str),
            Self::Many(v) => v.iter().map(String::as_str),
        }
    }
}

/// Migration options
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MigrationsOpts {
    pub table: Option<String>,
    pub schema: Option<String>,
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

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MigrationPrefix {
    Index,
    Timestamp,
    Supabase,
    Unix,
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
        account_id: EnvOr,
        #[serde(rename = "databaseId")]
        database_id: EnvOr,
        token: EnvOr,
    },
    /// AWS RDS Data API credentials — `{ database, secretArn, resourceArn }`.
    ///
    /// Also listed before `Url`/`Host` so the fully-specified shape wins in the
    /// untagged match. Note that `database` is also a field name on the Host
    /// shape, but the combination with `secretArn` + `resourceArn` uniquely
    /// identifies this variant.
    AwsDataApi {
        database: EnvOr,
        #[serde(rename = "secretArn")]
        secret_arn: EnvOr,
        #[serde(rename = "resourceArn")]
        resource_arn: EnvOr,
    },
    Url {
        url: EnvOr,
        #[serde(default, rename = "authToken")]
        auth_token: Option<EnvOr>,
    },
    Host {
        host: EnvOr,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        user: Option<EnvOr>,
        #[serde(default)]
        password: Option<EnvOr>,
        database: EnvOr,
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
    fn enabled(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Str(s) => !matches!(s.as_str(), "disable" | "false" | "no" | "off"),
        }
    }
}

// ============================================================================
// DatabaseConfig - Per-database configuration
// ============================================================================

/// Configuration for a single database
///
/// This structure matches drizzle-kit's config format for compatibility.
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
            (Dialect::Postgresql, RawCreds::Host { .. } | RawCreds::Url { .. }) => {}
            (_, RawCreds::Host { .. }) => {
                return Err(err(
                    "host-based dbCredentials are only supported for dialect = \"postgresql\"",
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
                Dialect::Sqlite,
                RawCreds::Url {
                    url: EnvOr::Value(url),
                    ..
                },
            ) if url.starts_with("libsql://") => Err(err(
                "libsql:// URLs require dialect = \"turso\" (for local SQLite files, use ./path.db)",
            )),
            (
                Dialect::Sqlite,
                RawCreds::Url {
                    url: EnvOr::Value(url),
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
                    url: EnvOr::Value(url),
                    ..
                },
            ) if !url.starts_with("libsql://") && !url.starts_with("http") => {
                Err(err("Turso URL must start with libsql:// or http(s)://"))
            }
            (
                Dialect::Postgresql,
                RawCreds::Url {
                    url: EnvOr::Value(url),
                    ..
                },
            ) if !url.starts_with("postgres") => {
                Err(err("PostgreSQL URL must start with postgres://"))
            }
            _ => Ok(()),
        }
    }

    /// Get typed credentials, resolving any environment variable references.
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

        // Helper to resolve an optional EnvOr
        let resolve_opt = |opt: &Option<EnvOr>| -> Result<Option<Box<str>>, Error> {
            opt.as_ref()
                .map_or(Ok(None), |e| e.resolve().map(|s| Some(s.into_boxed_str())))
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
                ssl: ssl.as_ref().is_some_and(SslVal::enabled),
            }),
            _ => return Ok(None),
        };

        Ok(Some(creds))
    }

    /// Migrations output directory
    #[inline]
    #[must_use]
    pub fn migrations_dir(&self) -> &Path {
        &self.out
    }

    /// Meta directory (for journal)
    #[inline]
    #[must_use]
    pub fn meta_dir(&self) -> PathBuf {
        self.out.join("meta")
    }

    /// Journal file path
    #[inline]
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Schema paths display string
    #[must_use]
    pub fn schema_display(&self) -> String {
        match &self.schema {
            Schema::One(s) => s.clone(),
            Schema::Many(v) => v.join(", "),
        }
    }

    /// Resolve schema files (with glob support).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if a glob pattern is invalid, if expanding a glob
    /// fails, or if the resolved pattern matches zero files.
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, Error> {
        let mut files = Vec::new();

        for pattern in self.schema.iter() {
            let pat = pattern.trim();

            // If it's not a glob pattern, treat it as a direct path (better Windows behavior).
            let is_glob = pat.contains('*') || pat.contains('?') || pat.contains('[');
            if !is_glob {
                let p = PathBuf::from(pat);
                if p.exists() {
                    files.push(p);
                    continue;
                }
            }

            // Glob patterns: normalize separators to avoid `\` being treated as an escape.
            let pat_norm = pat.replace('\\', "/");
            match glob::glob(&pat_norm) {
                Ok(paths) => {
                    let matched: Vec<_> = paths.filter_map(Result::ok).collect();
                    if matched.is_empty() && !is_glob {
                        let p = PathBuf::from(&pat_norm);
                        if p.exists() {
                            files.push(p);
                        }
                    } else {
                        files.extend(matched);
                    }
                }
                Err(e) => return Err(Error::Glob(pat.into(), e)),
            }
        }

        // Keep only real files (glob can return directories).
        files.retain(|p| p.is_file());
        files.sort();
        files.dedup();

        if files.is_empty() {
            return Err(Error::NoSchemaFiles(self.schema_display()));
        }

        Ok(files)
    }

    /// Get effective casing mode (default: camelCase)
    #[inline]
    #[must_use]
    pub fn effective_casing(&self) -> Casing {
        self.casing.unwrap_or_default()
    }

    /// Get effective introspect casing mode (default: camel)
    #[inline]
    #[must_use]
    pub fn effective_introspect_casing(&self) -> IntrospectCasing {
        self.introspect
            .as_ref()
            .map(|i| i.casing)
            .unwrap_or_default()
    }

    /// Get entities filter (default: empty)
    #[inline]
    #[must_use]
    pub fn effective_entities(&self) -> EntitiesFilter {
        self.entities.clone().unwrap_or_default()
    }

    /// Check if a role should be included based on entities filter
    #[must_use]
    pub fn should_include_role(&self, role_name: &str) -> bool {
        self.entities
            .as_ref()
            .is_some_and(|e| e.roles.should_include(role_name))
    }

    /// Check if roles are enabled in entities filter
    #[must_use]
    pub fn roles_enabled(&self) -> bool {
        self.entities.as_ref().is_some_and(|e| e.roles.is_enabled())
    }

    /// Get extensions filters (`PostgreSQL` only)
    #[must_use]
    pub fn extensions(&self) -> &[Extension] {
        self.extensions_filters.as_deref().unwrap_or(&[])
    }

    /// Check if an extension is in the filter list
    #[must_use]
    pub fn has_extension(&self, ext: Extension) -> bool {
        self.extensions_filters
            .as_ref()
            .is_some_and(|v| v.contains(&ext))
    }

    /// Get migration table name (default: __`drizzle_migrations`)
    #[must_use]
    pub fn migrations_table(&self) -> &str {
        self.migrations
            .as_ref()
            .and_then(|m| m.table.as_deref())
            .unwrap_or("__drizzle_migrations")
    }

    /// Get migration schema (`PostgreSQL` only, default: drizzle)
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

/// Main configuration structure
///
/// Supports both single-database (legacy) and multi-database configurations:
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

/// Default database name for single-database configs
pub const DEFAULT_DB: &str = "default";

impl Config {
    /// Load from default config file.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the default config file cannot be read, if it
    /// fails to parse as JSON, or if validation of the parsed config fails.
    pub fn load() -> Result<Self, Error> {
        Self::load_from(Path::new(CONFIG_FILE))
    }

    /// Load from specific path.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if `path` does not exist, [`Error::Io`] for
    /// other read errors, and [`Error`] variants for JSON-parse or validation
    /// failures.
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

    /// Load from string content
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

    /// Check if this is a single-database config
    #[must_use]
    pub const fn is_single_database(&self) -> bool {
        self.is_single
    }

    /// Get all database names
    pub fn database_names(&self) -> impl Iterator<Item = &str> {
        self.databases.keys().map(String::as_str)
    }

    /// Get a specific database config by name.
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

    /// Get the default database (for single-db mode or when only one db exists).
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

    /// Get dialect (for single-db mode backwards compat)
    #[must_use]
    pub fn dialect(&self) -> Dialect {
        self.default_database()
            .map(|d| d.dialect)
            .unwrap_or_default()
    }

    /// Get credentials (for single-db mode backwards compat).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if resolving the default database or its credentials
    /// fails (see [`Self::default_database`] and [`DatabaseConfig::credentials`]).
    pub fn credentials(&self) -> Result<Option<Credentials>, Error> {
        self.default_database()?.credentials()
    }

    /// Get migrations directory (for single-db mode backwards compat)
    #[must_use]
    pub fn migrations_dir(&self) -> &Path {
        self.default_database()
            .map_or_else(|_| Path::new("./drizzle"), |d| d.migrations_dir())
    }

    /// Get journal path (for single-db mode backwards compat)
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.default_database().map_or_else(
            |_| PathBuf::from("./drizzle/meta/_journal.json"),
            DatabaseConfig::journal_path,
        )
    }

    /// Get schema display (for single-db mode backwards compat)
    #[must_use]
    pub fn schema_display(&self) -> String {
        self.default_database()
            .map_or_else(|_| "src/schema.rs".into(), DatabaseConfig::schema_display)
    }

    /// Get schema files (for single-db mode backwards compat).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if resolving the default database fails or if
    /// resolving its schema files fails (see [`DatabaseConfig::schema_files`]).
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, Error> {
        self.default_database()?.schema_files()
    }

    /// Base dialect for SQL generation (for single-db mode backwards compat)
    #[must_use]
    pub fn base_dialect(&self) -> drizzle_types::Dialect {
        self.dialect().to_base()
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config not found: {}", .0.display())]
    NotFound(PathBuf),

    #[error("failed to read {}: {}", .0.display(), .1)]
    Io(PathBuf, #[source] std::io::Error),

    #[error("failed to parse {}: {}", .0.display(), .1)]
    Parse(PathBuf, #[source] toml::de::Error),

    #[error("driver '{driver}' invalid for {dialect} dialect")]
    InvalidDriver { driver: Driver, dialect: Dialect },

    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("invalid glob '{0}': {1}")]
    Glob(String, #[source] glob::PatternError),

    #[error("no schema files found: {0}")]
    NoSchemaFiles(String),

    #[error("environment variable '{0}' not found")]
    EnvNotFound(String),

    #[error("environment variable '{0}' invalid: {1}")]
    EnvInvalid(String, String),

    #[error("no databases configured")]
    NoDatabases,

    #[error("database '{0}' not found")]
    DatabaseNotFound(String),

    #[error("multiple databases configured, use --db to specify: {}", .0.join(", "))]
    DatabaseRequired(Vec<String>),
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
            "invalid credentials: host-based dbCredentials are only supported for dialect = \"postgresql\""
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
}
