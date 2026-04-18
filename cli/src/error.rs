//! Error types for the CLI

use thiserror::Error;

use crate::config::Error;

/// CLI errors
#[derive(Debug, Error)]
pub enum CliError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] Error),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// No schema files found
    #[error("No schema files found matching: {0}")]
    NoSchemaFiles(String),

    /// Dialect mismatch between snapshots
    #[error("Dialect mismatch between previous and current snapshots")]
    DialectMismatch,

    /// Database connection error
    #[error("Database connection failed: {0}")]
    ConnectionError(String),

    /// Migration execution error
    #[error("Migration failed: {0}")]
    MigrationError(String),

    /// Missing database driver
    #[error("No driver available for {dialect}. Build with '{feature}' feature enabled.")]
    MissingDriver {
        dialect: &'static str,
        feature: &'static str,
    },

    /// Operation not supported for this driver.
    ///
    /// Used for codegen-only drivers (e.g. `durable-sqlite` — no way to reach
    /// a running DO from the CLI) and for remote drivers the CLI hasn't been
    /// wired to yet (e.g. `d1-http`).
    #[error("{operation} is not supported for driver '{driver}'. {hint}")]
    UnsupportedForDriver {
        operation: &'static str,
        driver: &'static str,
        hint: &'static str,
    },

    /// Other errors
    #[error("{0}")]
    Other(String),
}
