//! Error types for the CLI

use thiserror::Error;

use crate::config::ConfigError;

/// CLI errors
#[derive(Debug, Error)]
pub enum CliError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// No schema files found
    #[error("No schema files found matching: {0}")]
    NoSchemaFiles(String),

    /// Dialect mismatch between snapshots
    #[error("Dialect mismatch between previous and current snapshots")]
    DialectMismatch,

    /// Other errors
    #[error("{0}")]
    Other(String),
}
