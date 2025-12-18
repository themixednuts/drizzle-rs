//! TOML configuration parsing for drizzle CLI
//!
//! This module handles loading and validating the `drizzle.config.toml` configuration file.
//! The config structure matches the drizzle-kit config format.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Default configuration file name
pub const DEFAULT_CONFIG_FILE: &str = "drizzle.config.toml";

/// Root configuration structure matching `drizzle.config.ts` from drizzle-kit
///
/// # Example: SQLite
/// ```toml
/// dialect = "sqlite"
/// schema = "src/schema.rs"
/// out = "./drizzle"
///
/// [dbCredentials]
/// url = "./dev.db"
/// ```
///
/// # Example: PostgreSQL
/// ```toml
/// dialect = "postgresql"
/// schema = ["src/schema/*.rs"]
/// out = "./drizzle"
///
/// [dbCredentials]
/// host = "localhost"
/// port = 5432
/// user = "postgres"
/// password = "password"
/// database = "mydb"
/// ```
///
/// # Example: Turso
/// ```toml
/// dialect = "turso"
/// schema = "src/schema.rs"
/// out = "./drizzle"
///
/// [dbCredentials]
/// url = "libsql://your-db.turso.io"
/// authToken = "your-token"
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrizzleConfig {
    /// Database dialect: "sqlite", "postgresql", "mysql", "turso"
    pub dialect: Dialect,

    /// Path or array of paths to schema files (glob patterns supported)
    #[serde(default)]
    pub schema: SchemaPath,

    /// Output directory for migrations (default: "./drizzle")
    #[serde(default = "default_out")]
    pub out: PathBuf,

    /// Whether to use SQL breakpoints in generated migrations
    #[serde(default = "default_breakpoints")]
    pub breakpoints: bool,

    /// Optional driver for specific database implementations
    #[serde(default)]
    pub driver: Option<Driver>,

    /// Database credentials
    #[serde(default)]
    pub db_credentials: Option<DbCredentials>,

    /// Table filter for push/introspect commands
    #[serde(default)]
    pub tables_filter: Option<StringOrArray>,

    /// Schema filter for PostgreSQL
    #[serde(default)]
    pub schema_filter: Option<StringOrArray>,

    /// Verbose output
    #[serde(default)]
    pub verbose: bool,

    /// Strict mode for push command
    #[serde(default)]
    pub strict: bool,

    /// Casing mode for generated code
    #[serde(default)]
    pub casing: Option<Casing>,

    /// Migration table configuration
    #[serde(default)]
    pub migrations: Option<MigrationsConfig>,
}

/// Database dialect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    Sqlite,
    #[serde(alias = "postgres")]
    Postgresql,
    Mysql,
    Turso,
    Singlestore,
}

impl From<Dialect> for drizzle_types::Dialect {
    fn from(d: Dialect) -> Self {
        match d {
            Dialect::Sqlite | Dialect::Turso => drizzle_types::Dialect::SQLite,
            Dialect::Postgresql => drizzle_types::Dialect::PostgreSQL,
            Dialect::Mysql => drizzle_types::Dialect::MySQL,
            Dialect::Singlestore => drizzle_types::Dialect::MySQL, // SingleStore is MySQL-compatible
        }
    }
}

/// Optional driver specification
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Driver {
    D1Http,
    Expo,
    DurableSqlite,
    SqliteCloud,
    AwsDataApi,
    Pglite,
}

/// Schema path - can be a single string or array of strings
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SchemaPath {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for SchemaPath {
    fn default() -> Self {
        SchemaPath::Single("src/schema.rs".to_string())
    }
}

impl SchemaPath {
    /// Get all schema paths as a vector
    pub fn paths(&self) -> Vec<String> {
        match self {
            SchemaPath::Single(s) => vec![s.clone()],
            SchemaPath::Multiple(v) => v.clone(),
        }
    }
}

/// String or array of strings
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrArray {
    Single(String),
    Multiple(Vec<String>),
}

/// Database credentials - dialect-specific
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DbCredentials {
    /// URL-based connection (SQLite file path or connection string)
    Url {
        url: String,
        #[serde(default, rename = "authToken")]
        auth_token: Option<String>,
    },
    /// Host-based connection (PostgreSQL, MySQL)
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
        ssl: Option<SslConfig>,
    },
    /// D1 HTTP credentials
    D1 {
        #[serde(rename = "accountId")]
        account_id: String,
        #[serde(rename = "databaseId")]
        database_id: String,
        token: String,
    },
    /// AWS Data API credentials
    AwsDataApi {
        database: String,
        #[serde(rename = "secretArn")]
        secret_arn: String,
        #[serde(rename = "resourceArn")]
        resource_arn: String,
    },
}

/// SSL configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SslConfig {
    Boolean(bool),
    Mode(String),
}

/// Casing mode
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Casing {
    CamelCase,
    SnakeCase,
}

/// Migration table configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MigrationsConfig {
    /// Custom table name for migrations
    #[serde(default)]
    pub table: Option<String>,

    /// Schema for migrations table (PostgreSQL only)
    #[serde(default)]
    pub schema: Option<String>,

    /// Migration naming prefix
    #[serde(default)]
    pub prefix: Option<MigrationPrefix>,
}

/// Migration file naming prefix
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationPrefix {
    Index,
    Timestamp,
    Supabase,
    Unix,
    None,
}

fn default_out() -> PathBuf {
    PathBuf::from("./drizzle")
}

fn default_breakpoints() -> bool {
    true
}

impl DrizzleConfig {
    /// Load configuration from the default `drizzle.config.toml` file
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from(&PathBuf::from(DEFAULT_CONFIG_FILE))
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(path.to_path_buf(), e.to_string()))?;

        let config: DrizzleConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(path.to_path_buf(), e.to_string()))?;

        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate credentials match dialect
        if let Some(ref creds) = self.db_credentials {
            match (&self.dialect, creds) {
                (Dialect::Postgresql, DbCredentials::Url { .. })
                | (Dialect::Postgresql, DbCredentials::Host { .. })
                | (Dialect::Postgresql, DbCredentials::AwsDataApi { .. }) => {}
                (Dialect::Sqlite, DbCredentials::Url { .. })
                | (Dialect::Sqlite, DbCredentials::D1 { .. }) => {}
                (Dialect::Turso, DbCredentials::Url { .. }) => {}
                (Dialect::Mysql, DbCredentials::Url { .. })
                | (Dialect::Mysql, DbCredentials::Host { .. }) => {}
                (Dialect::Singlestore, DbCredentials::Url { .. })
                | (Dialect::Singlestore, DbCredentials::Host { .. }) => {}
                (dialect, _) => {
                    return Err(ConfigError::InvalidCredentials(format!(
                        "Invalid credentials for dialect {:?}",
                        dialect
                    )));
                }
            }
        }
        Ok(())
    }

    /// Get the drizzle_types::Dialect for this config
    pub fn drizzle_dialect(&self) -> drizzle_types::Dialect {
        self.dialect.into()
    }

    /// Get the migrations output directory
    pub fn migrations_dir(&self) -> PathBuf {
        self.out.clone()
    }

    /// Get the meta directory path
    pub fn meta_dir(&self) -> PathBuf {
        self.out.join("meta")
    }

    /// Get the journal file path
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Get a display string for the schema pattern(s)
    pub fn schema_pattern_display(&self) -> String {
        self.schema.paths().join(", ")
    }

    /// Resolve schema file paths (supports globs)
    pub fn schema_files(&self) -> Result<Vec<PathBuf>, ConfigError> {
        let mut all_paths = Vec::new();

        for pattern in self.schema.paths() {
            let paths: Vec<PathBuf> = glob::glob(&pattern)
                .map_err(|e| ConfigError::GlobError(pattern.clone(), e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            if paths.is_empty() {
                // Try as literal path
                let path = PathBuf::from(&pattern);
                if path.exists() {
                    all_paths.push(path);
                }
            } else {
                all_paths.extend(paths);
            }
        }

        if all_paths.is_empty() {
            return Err(ConfigError::NoSchemaFiles(format!(
                "{:?}",
                self.schema.paths()
            )));
        }

        Ok(all_paths)
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {}", .0.display())]
    NotFound(PathBuf),

    #[error("Failed to read configuration file {path}: {msg}", path = .0.display(), msg = .1)]
    IoError(PathBuf, String),

    #[error("Failed to parse configuration file {path}: {msg}", path = .0.display(), msg = .1)]
    ParseError(PathBuf, String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Invalid glob pattern '{0}': {1}")]
    GlobError(String, String),

    #[error("No schema files found matching pattern: {0}")]
    NoSchemaFiles(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sqlite_config() {
        let toml = r#"
dialect = "sqlite"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
"#;

        let config: DrizzleConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.dialect, Dialect::Sqlite);
        assert!(matches!(
            config.db_credentials,
            Some(DbCredentials::Url { .. })
        ));
    }

    #[test]
    fn test_parse_postgresql_url_config() {
        let toml = r#"
dialect = "postgresql"
schema = ["src/schema/*.rs"]
out = "./migrations"

[dbCredentials]
url = "postgres://user:pass@localhost:5432/mydb"
"#;

        let config: DrizzleConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.dialect, Dialect::Postgresql);
    }

    #[test]
    fn test_parse_postgresql_host_config() {
        let toml = r#"
dialect = "postgresql"
schema = "src/schema.rs"

[dbCredentials]
host = "localhost"
port = 5432
user = "postgres"
password = "secret"
database = "mydb"
"#;

        let config: DrizzleConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.dialect, Dialect::Postgresql);
        if let Some(DbCredentials::Host {
            host,
            port,
            database,
            ..
        }) = config.db_credentials
        {
            assert_eq!(host, "localhost");
            assert_eq!(port, Some(5432));
            assert_eq!(database, "mydb");
        } else {
            panic!("Expected Host credentials");
        }
    }

    #[test]
    fn test_parse_turso_config() {
        let toml = r#"
dialect = "turso"
schema = "src/schema.rs"

[dbCredentials]
url = "libsql://my-db.turso.io"
authToken = "my-token"
"#;

        let config: DrizzleConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.dialect, Dialect::Turso);
    }

    #[test]
    fn test_default_values() {
        let toml = r#"
dialect = "sqlite"

[dbCredentials]
url = "./dev.db"
"#;

        let config: DrizzleConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.out, PathBuf::from("./drizzle"));
        assert!(config.breakpoints);
    }

    #[test]
    fn test_schema_array() {
        let toml = r#"
dialect = "sqlite"
schema = ["src/tables/*.rs", "src/views/*.rs"]
"#;

        let config: DrizzleConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.schema.paths().len(), 2);
    }
}
