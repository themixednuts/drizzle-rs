//! Configuration types for drizzle.toml

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Database dialect
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    #[default]
    Sqlite,
    Postgresql,
    Mysql,
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dialect::Sqlite => write!(f, "sqlite"),
            Dialect::Postgresql => write!(f, "postgresql"),
            Dialect::Mysql => write!(f, "mysql"),
        }
    }
}

impl Dialect {
    /// Parse a dialect from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sqlite" | "turso" | "libsql" => Some(Dialect::Sqlite),
            "postgresql" | "postgres" | "pg" => Some(Dialect::Postgresql),
            "mysql" => Some(Dialect::Mysql),
            _ => None,
        }
    }
}

/// Database connection configuration
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,
    /// Auth token for Turso/LibSQL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

/// Migration-specific configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MigrationsConfig {
    /// Table name for tracking applied migrations
    #[serde(default = "default_migrations_table")]
    pub table: String,
    /// Schema name (PostgreSQL only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

fn default_migrations_table() -> String {
    "__drizzle_migrations".to_string()
}

impl Default for MigrationsConfig {
    fn default() -> Self {
        Self {
            table: default_migrations_table(),
            schema: None,
        }
    }
}

/// Main configuration struct for drizzle.toml
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DrizzleConfig {
    /// Database dialect (sqlite, postgresql, mysql)
    pub dialect: Dialect,
    /// Output directory for migrations
    #[serde(default = "default_out")]
    pub out: PathBuf,
    /// Enable SQL statement breakpoints
    #[serde(default = "default_breakpoints")]
    pub breakpoints: bool,
    /// Database connection configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,
    /// Migration configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrations: Option<MigrationsConfig>,
}

fn default_out() -> PathBuf {
    PathBuf::from("./drizzle")
}

fn default_breakpoints() -> bool {
    true
}

impl Default for DrizzleConfig {
    fn default() -> Self {
        Self {
            dialect: Dialect::default(),
            out: default_out(),
            breakpoints: default_breakpoints(),
            database: None,
            migrations: None,
        }
    }
}

impl DrizzleConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        Self::parse(&contents)
    }

    /// Parse configuration from a TOML string
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        toml::from_str(s).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Get the migrations directory path
    pub fn migrations_dir(&self) -> PathBuf {
        self.out.join("migrations")
    }

    /// Get the meta directory path
    pub fn meta_dir(&self) -> PathBuf {
        self.migrations_dir().join("meta")
    }

    /// Get the journal file path
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Get the migrations tracking table name
    pub fn migrations_table(&self) -> &str {
        self.migrations
            .as_ref()
            .map(|m| m.table.as_str())
            .unwrap_or("__drizzle_migrations")
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml = r#"
dialect = "sqlite"
out = "./drizzle"
breakpoints = true

[database]
url = "sqlite:./dev.db"

[migrations]
table = "__drizzle_migrations"
"#;

        let config = DrizzleConfig::parse(toml).unwrap();
        assert_eq!(config.dialect, Dialect::Sqlite);
        assert_eq!(config.out, PathBuf::from("./drizzle"));
        assert!(config.breakpoints);
        assert_eq!(config.database.unwrap().url, "sqlite:./dev.db");
    }

    #[test]
    fn test_default_config() {
        let config = DrizzleConfig::default();
        assert_eq!(config.dialect, Dialect::Sqlite);
        assert_eq!(config.out, PathBuf::from("./drizzle"));
        assert!(config.breakpoints);
    }
}
