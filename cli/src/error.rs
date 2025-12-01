//! CLI error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Schema file not found: {0}")]
    SchemaNotFound(String),

    #[error("Invalid dialect: {0}. Expected: sqlite, postgresql, mysql")]
    InvalidDialect(String),

    #[error("No migrations found in {0}")]
    NoMigrations(String),

    #[error("Journal file not found: {0}")]
    JournalNotFound(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}
