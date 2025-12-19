//! Configuration for Drizzle CLI
//!
//! Handles loading `drizzle.config.toml` with type-safe credentials.

use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "drizzle.config.toml";

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
    pub const ALL: &'static [&'static str] = &["sqlite", "postgresql", "mysql", "turso", "singlestore"];

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
        "d1-http", "expo", "durable-sqlite", "sqlite-cloud", "aws-data-api", "pglite"
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
            Dialect::Sqlite => &[Self::D1Http, Self::Expo, Self::DurableSqlite, Self::SqliteCloud],
            Dialect::Turso => &[Self::D1Http, Self::SqliteCloud],
            Dialect::Postgresql => &[Self::AwsDataApi, Self::Pglite],
            Dialect::Mysql | Dialect::Singlestore => &[],
        }
    }

    #[inline]
    pub const fn is_valid_for(self, dialect: Dialect) -> bool {
        matches!(
            (self, dialect),
            (Self::D1Http | Self::Expo | Self::DurableSqlite | Self::SqliteCloud, Dialect::Sqlite)
            | (Self::D1Http | Self::SqliteCloud, Dialect::Turso)
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
    Turso { url: Box<str>, auth_token: Option<Box<str>> },

    /// PostgreSQL
    Postgres(PostgresCreds),

    /// MySQL (also used for SingleStore)
    Mysql(MysqlCreds),

    /// Cloudflare D1
    D1 { account_id: Box<str>, database_id: Box<str>, token: Box<str> },

    /// AWS RDS Data API
    AwsDataApi { database: Box<str>, secret_arn: Box<str>, resource_arn: Box<str> },

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
            Self::Host { host, port, user, password, database, .. } => {
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
            Self::Host { host, port, user, password, database, .. } => {
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
// Configuration
// ============================================================================

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub dialect: Dialect,

    #[serde(default = "default_schema")]
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

fn default_schema() -> Schema { Schema::One("src/schema.rs".into()) }
fn default_out() -> PathBuf { PathBuf::from("./drizzle") }
fn yes() -> bool { true }

/// Schema path(s)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Schema {
    One(String),
    Many(Vec<String>),
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
pub enum MigrationPrefix { Index, Timestamp, Supabase, Unix, None }

// ============================================================================
// Raw credentials (serde parsing helper)
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawCreds {
    Url {
        url: String,
        #[serde(default, rename = "authToken")]
        auth_token: Option<String>,
    },
    Host {
        host: String,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        user: Option<String>,
        #[serde(default)]
        password: Option<String>,
        database: String,
        #[serde(default)]
        ssl: Option<SslVal>,
    },
    D1 {
        #[serde(rename = "accountId")]
        account_id: String,
        #[serde(rename = "databaseId")]
        database_id: String,
        token: String,
    },
    Aws {
        database: String,
        #[serde(rename = "secretArn")]
        secret_arn: String,
        #[serde(rename = "resourceArn")]
        resource_arn: String,
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
// Config implementation
// ============================================================================

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

        let mut config: Self = toml::from_str(&content)
            .map_err(|e| Error::Parse(path.into(), e))?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&mut self) -> Result<(), Error> {
        // Check driver compatibility
        if let Some(d) = self.driver {
            if !d.is_valid_for(self.dialect) {
                return Err(Error::InvalidDriver { driver: d, dialect: self.dialect });
            }
        }

        // Validate credentials if present
        if let Some(ref raw) = self.db_credentials {
            self.validate_creds(raw)?;
        }

        Ok(())
    }

    fn validate_creds(&self, raw: &RawCreds) -> Result<(), Error> {
        let err = |msg: &str| Error::InvalidCredentials(msg.into());

        // Driver-specific checks
        match self.driver {
            Some(Driver::D1Http) if !matches!(raw, RawCreds::D1 { .. }) => {
                return Err(err("D1 driver requires accountId, databaseId, and token"));
            }
            Some(Driver::AwsDataApi) if !matches!(raw, RawCreds::Aws { .. }) => {
                return Err(err("AWS Data API requires database, secretArn, and resourceArn"));
            }
            _ => {}
        }

        // Dialect-specific checks
        match (self.dialect, raw) {
            (Dialect::Sqlite, RawCreds::Url { auth_token: Some(_), .. }) => {
                Err(err("SQLite doesn't support authToken (use dialect = \"turso\")"))
            }
            (Dialect::Sqlite, RawCreds::Url { url, .. }) if url.starts_with("libsql://") => {
                Err(err("libsql:// URLs require dialect = \"turso\""))
            }
            (Dialect::Turso, RawCreds::Url { url, .. })
                if !url.starts_with("libsql://") && !url.starts_with("http") => {
                Err(err("Turso URL must start with libsql:// or http(s)://"))
            }
            (Dialect::Postgresql, RawCreds::Url { url, .. })
                if !url.starts_with("postgres") => {
                Err(err("PostgreSQL URL must start with postgres://"))
            }
            (Dialect::Mysql, RawCreds::Url { url, .. })
                if !url.starts_with("mysql://") => {
                Err(err("MySQL URL must start with mysql://"))
            }
            (Dialect::Singlestore, RawCreds::Url { url, .. })
                if !url.starts_with("mysql://") && !url.starts_with("singlestore://") => {
                Err(err("SingleStore URL must start with mysql:// or singlestore://"))
            }
            _ => Ok(()),
        }
    }

    /// Get typed credentials
    pub fn credentials(&self) -> Option<Credentials> {
        let raw = self.db_credentials.as_ref()?;

        let creds = match (self.dialect, self.driver, raw) {
            // D1
            (_, Some(Driver::D1Http), RawCreds::D1 { account_id, database_id, token }) => {
                Credentials::D1 {
                    account_id: account_id.as_str().into(),
                    database_id: database_id.as_str().into(),
                    token: token.as_str().into(),
                }
            }
            // AWS Data API
            (_, Some(Driver::AwsDataApi), RawCreds::Aws { database, secret_arn, resource_arn }) => {
                Credentials::AwsDataApi {
                    database: database.as_str().into(),
                    secret_arn: secret_arn.as_str().into(),
                    resource_arn: resource_arn.as_str().into(),
                }
            }
            // PGlite
            (_, Some(Driver::Pglite), RawCreds::Url { url, .. }) => {
                Credentials::Pglite { path: url.as_str().into() }
            }
            // SQLite Cloud
            (_, Some(Driver::SqliteCloud), RawCreds::Url { url, .. }) => {
                Credentials::SqliteCloud { url: url.as_str().into() }
            }
            // SQLite
            (Dialect::Sqlite, _, RawCreds::Url { url, .. }) => {
                Credentials::Sqlite { path: url.as_str().into() }
            }
            // Turso
            (Dialect::Turso, _, RawCreds::Url { url, auth_token }) => {
                Credentials::Turso {
                    url: url.as_str().into(),
                    auth_token: auth_token.as_deref().map(Into::into),
                }
            }
            // PostgreSQL URL
            (Dialect::Postgresql, _, RawCreds::Url { url, .. }) => {
                Credentials::Postgres(PostgresCreds::Url(url.as_str().into()))
            }
            // PostgreSQL Host
            (Dialect::Postgresql, _, RawCreds::Host { host, port, user, password, database, ssl }) => {
                Credentials::Postgres(PostgresCreds::Host {
                    host: host.as_str().into(),
                    port: port.unwrap_or(5432),
                    user: user.as_deref().map(Into::into),
                    password: password.as_deref().map(Into::into),
                    database: database.as_str().into(),
                    ssl: ssl.as_ref().map(|s| s.enabled()).unwrap_or(false),
                })
            }
            // MySQL/SingleStore URL
            (Dialect::Mysql | Dialect::Singlestore, _, RawCreds::Url { url, .. }) => {
                Credentials::Mysql(MysqlCreds::Url(url.as_str().into()))
            }
            // MySQL/SingleStore Host
            (Dialect::Mysql | Dialect::Singlestore, _, RawCreds::Host { host, port, user, password, database, ssl }) => {
                Credentials::Mysql(MysqlCreds::Host {
                    host: host.as_str().into(),
                    port: port.unwrap_or(3306),
                    user: user.as_deref().map(Into::into),
                    password: password.as_deref().map(Into::into),
                    database: database.as_str().into(),
                    ssl: ssl.as_ref().map(|s| s.enabled()).unwrap_or(false),
                })
            }
            _ => return None,
        };

        Some(creds)
    }

    /// Base dialect for SQL generation
    #[inline]
    pub fn base_dialect(&self) -> drizzle_types::Dialect {
        self.dialect.to_base()
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
        let cfg: Config = toml::from_str(r#"
            dialect = "sqlite"
            [dbCredentials]
            url = "./dev.db"
        "#).unwrap();
        assert!(matches!(cfg.credentials(), Some(Credentials::Sqlite { .. })));
    }

    #[test]
    fn postgres_url() {
        let cfg: Config = toml::from_str(r#"
            dialect = "postgresql"
            [dbCredentials]
            url = "postgres://localhost/db"
        "#).unwrap();
        assert!(matches!(cfg.credentials(), Some(Credentials::Postgres(PostgresCreds::Url(_)))));
    }

    #[test]
    fn postgres_host() {
        let cfg: Config = toml::from_str(r#"
            dialect = "postgresql"
            [dbCredentials]
            host = "localhost"
            database = "mydb"
            port = 5432
        "#).unwrap();
        if let Some(Credentials::Postgres(PostgresCreds::Host { host, port, .. })) = cfg.credentials() {
            assert_eq!(&*host, "localhost");
            assert_eq!(port, 5432);
        } else {
            panic!("expected postgres host creds");
        }
    }

    #[test]
    fn turso() {
        let cfg: Config = toml::from_str(r#"
            dialect = "turso"
            [dbCredentials]
            url = "libsql://db.turso.io"
            authToken = "token"
        "#).unwrap();
        if let Some(Credentials::Turso { url, auth_token }) = cfg.credentials() {
            assert_eq!(&*url, "libsql://db.turso.io");
            assert_eq!(auth_token.as_deref(), Some("token"));
        } else {
            panic!("expected turso creds");
        }
    }

    #[test]
    fn d1() {
        let cfg: Config = toml::from_str(r#"
            dialect = "sqlite"
            driver = "d1-http"
            [dbCredentials]
            accountId = "acc"
            databaseId = "db"
            token = "tok"
        "#).unwrap();
        assert!(matches!(cfg.credentials(), Some(Credentials::D1 { .. })));
    }

    #[test]
    fn defaults() {
        let cfg: Config = toml::from_str(r#"dialect = "sqlite""#).unwrap();
        assert_eq!(cfg.out, PathBuf::from("./drizzle"));
        assert!(cfg.breakpoints);
    }

    #[test]
    fn dialect_base() {
        assert_eq!(Dialect::Sqlite.to_base(), drizzle_types::Dialect::SQLite);
        assert_eq!(Dialect::Turso.to_base(), drizzle_types::Dialect::SQLite);
        assert_eq!(Dialect::Postgresql.to_base(), drizzle_types::Dialect::PostgreSQL);
    }

    #[test]
    fn driver_compat() {
        assert!(Driver::D1Http.is_valid_for(Dialect::Sqlite));
        assert!(!Driver::D1Http.is_valid_for(Dialect::Postgresql));
        assert!(Driver::AwsDataApi.is_valid_for(Dialect::Postgresql));
    }
}
