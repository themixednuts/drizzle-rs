//! Configuration for Drizzle CLI
//!
//! Handles loading `drizzle.config.toml` with type-safe credentials.
//! Supports both single-database (legacy) and multi-database configurations.

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "drizzle.config.toml";

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
    /// Resolve the value, looking up environment variable if needed
    pub fn resolve(&self) -> Result<String, Error> {
        match self {
            Self::Value(v) => Ok(v.clone()),
            Self::Env(var) => std::env::var(var).map_err(|_| Error::EnvNotFound(var.clone())),
        }
    }

    /// Resolve to an optional value (returns None for missing env vars)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    #[default]
    Sqlite,
    #[serde(alias = "postgres")]
    Postgresql,
    Mysql,
    Turso,
    Singlestore,
}

impl Dialect {
    pub const ALL: &'static [&'static str] =
        &["sqlite", "postgresql", "mysql", "turso", "singlestore"];

    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgresql => "postgresql",
            Self::Mysql => "mysql",
            Self::Turso => "turso",
            Self::Singlestore => "singlestore",
        }
    }

    #[inline]
    pub const fn to_base(self) -> drizzle_types::Dialect {
        match self {
            Self::Sqlite | Self::Turso => drizzle_types::Dialect::SQLite,
            Self::Postgresql => drizzle_types::Dialect::PostgreSQL,
            Self::Mysql | Self::Singlestore => drizzle_types::Dialect::MySQL,
        }
    }
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

/// Database driver for special connection types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Driver {
    D1Http,
    Expo,
    DurableSqlite,
    SqliteCloud,
    AwsDataApi,
    Pglite,
}

impl Driver {
    pub const ALL: &'static [&'static str] = &[
        "d1-http",
        "expo",
        "durable-sqlite",
        "sqlite-cloud",
        "aws-data-api",
        "pglite",
    ];

    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::D1Http => "d1-http",
            Self::Expo => "expo",
            Self::DurableSqlite => "durable-sqlite",
            Self::SqliteCloud => "sqlite-cloud",
            Self::AwsDataApi => "aws-data-api",
            Self::Pglite => "pglite",
        }
    }

    pub const fn valid_for(dialect: Dialect) -> &'static [Driver] {
        match dialect {
            Dialect::Sqlite => &[
                Self::D1Http,
                Self::Expo,
                Self::DurableSqlite,
                Self::SqliteCloud,
            ],
            Dialect::Turso => &[Self::D1Http, Self::SqliteCloud],
            Dialect::Postgresql => &[Self::AwsDataApi, Self::Pglite],
            Dialect::Mysql | Dialect::Singlestore => &[],
        }
    }

    #[inline]
    pub const fn is_valid_for(self, dialect: Dialect) -> bool {
        matches!(
            (self, dialect),
            (
                Self::D1Http | Self::Expo | Self::DurableSqlite | Self::SqliteCloud,
                Dialect::Sqlite
            ) | (Self::D1Http | Self::SqliteCloud, Dialect::Turso)
                | (Self::AwsDataApi | Self::Pglite, Dialect::Postgresql)
        )
    }
}

impl std::fmt::Display for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// Credentials
// ============================================================================

/// Database credentials - validated and typed
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Local SQLite file
    Sqlite { path: Box<str> },

    /// Turso/LibSQL
    Turso {
        url: Box<str>,
        auth_token: Option<Box<str>>,
    },

    /// PostgreSQL
    Postgres(PostgresCreds),

    /// MySQL (also used for SingleStore)
    Mysql(MysqlCreds),

    /// Cloudflare D1
    D1 {
        account_id: Box<str>,
        database_id: Box<str>,
        token: Box<str>,
    },

    /// AWS RDS Data API
    AwsDataApi {
        database: Box<str>,
        secret_arn: Box<str>,
        resource_arn: Box<str>,
    },

    /// PGlite
    Pglite { path: Box<str> },

    /// SQLite Cloud
    SqliteCloud { url: Box<str> },
}

/// PostgreSQL credentials
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

/// MySQL credentials
#[derive(Debug, Clone)]
pub enum MysqlCreds {
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

impl MysqlCreds {
    /// Build connection URL
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
                format!("mysql://{auth}{host}:{port}/{database}")
            }
        }
    }
}

// ============================================================================
// Schema path(s)
// ============================================================================

/// Schema path(s)
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct MigrationsOpts {
    pub table: Option<String>,
    pub schema: Option<String>,
    pub prefix: Option<MigrationPrefix>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawCreds {
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
    D1 {
        #[serde(rename = "accountId")]
        account_id: EnvOr,
        #[serde(rename = "databaseId")]
        database_id: EnvOr,
        token: EnvOr,
    },
    Aws {
        database: EnvOr,
        #[serde(rename = "secretArn")]
        secret_arn: EnvOr,
        #[serde(rename = "resourceArn")]
        resource_arn: EnvOr,
    },
}

#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
    pub dialect: Dialect,

    #[serde(default)]
    pub schema: Schema,

    #[serde(default = "default_out")]
    pub out: PathBuf,

    #[serde(default = "yes")]
    pub breakpoints: bool,

    #[serde(default)]
    pub driver: Option<Driver>,

    #[serde(default)]
    db_credentials: Option<RawCreds>,

    #[serde(default)]
    pub tables_filter: Option<Filter>,

    #[serde(default)]
    pub schema_filter: Option<Filter>,

    #[serde(default)]
    pub verbose: bool,

    #[serde(default)]
    pub strict: bool,

    #[serde(default)]
    pub migrations: Option<MigrationsOpts>,
}

fn default_out() -> PathBuf {
    PathBuf::from("./drizzle")
}

fn yes() -> bool {
    true
}

impl DatabaseConfig {
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

        Ok(())
    }

    fn validate_creds(&self, raw: &RawCreds, _name: &str) -> Result<(), Error> {
        let err = |msg: &str| Error::InvalidCredentials(msg.into());

        // Driver-specific checks
        match self.driver {
            Some(Driver::D1Http) if !matches!(raw, RawCreds::D1 { .. }) => {
                return Err(err("D1 driver requires accountId, databaseId, and token"));
            }
            Some(Driver::AwsDataApi) if !matches!(raw, RawCreds::Aws { .. }) => {
                return Err(err(
                    "AWS Data API requires database, secretArn, and resourceArn",
                ));
            }
            _ => {}
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
            ) if url.starts_with("libsql://") => {
                Err(err("libsql:// URLs require dialect = \"turso\""))
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
            (
                Dialect::Mysql,
                RawCreds::Url {
                    url: EnvOr::Value(url),
                    ..
                },
            ) if !url.starts_with("mysql://") => Err(err("MySQL URL must start with mysql://")),
            (
                Dialect::Singlestore,
                RawCreds::Url {
                    url: EnvOr::Value(url),
                    ..
                },
            ) if !url.starts_with("mysql://") && !url.starts_with("singlestore://") => Err(err(
                "SingleStore URL must start with mysql:// or singlestore://",
            )),
            _ => Ok(()),
        }
    }

    /// Get typed credentials, resolving any environment variable references
    pub fn credentials(&self) -> Result<Option<Credentials>, Error> {
        let raw = match self.db_credentials.as_ref() {
            Some(r) => r,
            None => return Ok(None),
        };

        // Helper to resolve an optional EnvOr
        let resolve_opt = |opt: &Option<EnvOr>| -> Result<Option<Box<str>>, Error> {
            match opt {
                Some(e) => e.resolve().map(|s| Some(s.into_boxed_str())),
                None => Ok(None),
            }
        };

        let creds = match (self.dialect, self.driver, raw) {
            // D1
            (
                _,
                Some(Driver::D1Http),
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
            // AWS Data API
            (
                _,
                Some(Driver::AwsDataApi),
                RawCreds::Aws {
                    database,
                    secret_arn,
                    resource_arn,
                },
            ) => Credentials::AwsDataApi {
                database: database.resolve()?.into_boxed_str(),
                secret_arn: secret_arn.resolve()?.into_boxed_str(),
                resource_arn: resource_arn.resolve()?.into_boxed_str(),
            },
            // PGlite
            (_, Some(Driver::Pglite), RawCreds::Url { url, .. }) => Credentials::Pglite {
                path: url.resolve()?.into_boxed_str(),
            },
            // SQLite Cloud
            (_, Some(Driver::SqliteCloud), RawCreds::Url { url, .. }) => Credentials::SqliteCloud {
                url: url.resolve()?.into_boxed_str(),
            },
            // SQLite
            (Dialect::Sqlite, _, RawCreds::Url { url, .. }) => Credentials::Sqlite {
                path: url.resolve()?.into_boxed_str(),
            },
            // Turso
            (Dialect::Turso, _, RawCreds::Url { url, auth_token }) => Credentials::Turso {
                url: url.resolve()?.into_boxed_str(),
                auth_token: resolve_opt(auth_token)?,
            },
            // PostgreSQL URL
            (Dialect::Postgresql, _, RawCreds::Url { url, .. }) => {
                Credentials::Postgres(PostgresCreds::Url(url.resolve()?.into_boxed_str()))
            }
            // PostgreSQL Host
            (
                Dialect::Postgresql,
                _,
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
                ssl: ssl.as_ref().map(|s| s.enabled()).unwrap_or(false),
            }),
            // MySQL/SingleStore URL
            (Dialect::Mysql | Dialect::Singlestore, _, RawCreds::Url { url, .. }) => {
                Credentials::Mysql(MysqlCreds::Url(url.resolve()?.into_boxed_str()))
            }
            // MySQL/SingleStore Host
            (
                Dialect::Mysql | Dialect::Singlestore,
                _,
                RawCreds::Host {
                    host,
                    port,
                    user,
                    password,
                    database,
                    ssl,
                },
            ) => Credentials::Mysql(MysqlCreds::Host {
                host: host.resolve()?.into_boxed_str(),
                port: port.unwrap_or(3306),
                user: resolve_opt(user)?,
                password: resolve_opt(password)?,
                database: database.resolve()?.into_boxed_str(),
                ssl: ssl.as_ref().map(|s| s.enabled()).unwrap_or(false),
            }),
            _ => return Ok(None),
        };

        Ok(Some(creds))
    }

    /// Migrations output directory
    #[inline]
    pub fn migrations_dir(&self) -> &Path {
        &self.out
    }

    /// Meta directory (for journal)
    #[inline]
    pub fn meta_dir(&self) -> PathBuf {
        self.out.join("meta")
    }

    /// Journal file path
    #[inline]
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Schema paths display string
    pub fn schema_display(&self) -> String {
        match &self.schema {
            Schema::One(s) => s.clone(),
            Schema::Many(v) => v.join(", "),
        }
    }

    /// Resolve schema files (with glob support)
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, Error> {
        let mut files = Vec::new();

        for pattern in self.schema.iter() {
            match glob::glob(pattern) {
                Ok(paths) => {
                    let matched: Vec<_> = paths.filter_map(Result::ok).collect();
                    if matched.is_empty() {
                        let p = PathBuf::from(pattern);
                        if p.exists() {
                            files.push(p);
                        }
                    } else {
                        files.extend(matched);
                    }
                }
                Err(e) => return Err(Error::Glob(pattern.into(), e)),
            }
        }

        if files.is_empty() {
            return Err(Error::NoSchemaFiles(self.schema_display()));
        }

        Ok(files)
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
    /// Load from default config file
    pub fn load() -> Result<Self, Error> {
        Self::load_from(Path::new(CONFIG_FILE))
    }

    /// Load from specific path
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
        // Try multi-database format first
        if let Ok(multi) = toml::from_str::<MultiDbConfig>(content)
            && !multi.databases.is_empty()
        {
            let mut config = Self {
                databases: multi.databases,
                is_single: false,
            };
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
        config.validate()?;
        Ok(config)
    }

    fn validate(&mut self) -> Result<(), Error> {
        for (name, db) in &self.databases {
            db.validate(name)?;
        }
        Ok(())
    }

    /// Check if this is a single-database config
    pub fn is_single_database(&self) -> bool {
        self.is_single
    }

    /// Get all database names
    pub fn database_names(&self) -> impl Iterator<Item = &str> {
        self.databases.keys().map(String::as_str)
    }

    /// Get a specific database config by name
    ///
    /// If name is None, returns the default/only database.
    /// For single-db configs, any name or None returns the single database.
    pub fn database(&self, name: Option<&str>) -> Result<&DatabaseConfig, Error> {
        match name {
            None => {
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
            }
            Some(name) => {
                if self.is_single {
                    // For single-db config, accept any name
                    self.databases.get(DEFAULT_DB).ok_or(Error::NoDatabases)
                } else {
                    self.databases
                        .get(name)
                        .ok_or_else(|| Error::DatabaseNotFound(name.to_string()))
                }
            }
        }
    }

    /// Get the default database (for single-db mode or when only one db exists)
    pub fn default_database(&self) -> Result<&DatabaseConfig, Error> {
        self.database(None)
    }

    // ========================================================================
    // Backwards compatibility - delegate to default database
    // ========================================================================

    /// Get dialect (for single-db mode backwards compat)
    pub fn dialect(&self) -> Dialect {
        self.default_database().map(|d| d.dialect).unwrap_or_default()
    }

    /// Get credentials (for single-db mode backwards compat)
    pub fn credentials(&self) -> Result<Option<Credentials>, Error> {
        self.default_database()?.credentials()
    }

    /// Get migrations directory (for single-db mode backwards compat)
    pub fn migrations_dir(&self) -> &Path {
        self.default_database()
            .map(|d| d.migrations_dir())
            .unwrap_or(Path::new("./drizzle"))
    }

    /// Get journal path (for single-db mode backwards compat)
    pub fn journal_path(&self) -> PathBuf {
        self.default_database()
            .map(|d| d.journal_path())
            .unwrap_or_else(|_| PathBuf::from("./drizzle/meta/_journal.json"))
    }

    /// Get schema display (for single-db mode backwards compat)
    pub fn schema_display(&self) -> String {
        self.default_database()
            .map(|d| d.schema_display())
            .unwrap_or_else(|_| "src/schema.rs".into())
    }

    /// Get schema files (for single-db mode backwards compat)
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, Error> {
        self.default_database()?.schema_files()
    }

    /// Base dialect for SQL generation (for single-db mode backwards compat)
    pub fn base_dialect(&self) -> drizzle_types::Dialect {
        self.dialect().to_base()
    }
}

// Re-export as DrizzleConfig for compatibility
pub type DrizzleConfig = Config;

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

pub type ConfigError = Error;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
