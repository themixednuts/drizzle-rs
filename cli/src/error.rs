//! [`CliError`], the error type every command returns.

use thiserror::Error;

use crate::config::Error;

/// An error from a CLI command. Its `Display` text is what the user sees.
#[derive(Debug, Error)]
pub enum CliError {
    /// Loading or reading the config failed.
    #[error("Configuration error: {0}")]
    Config(#[from] Error),

    /// A file or folder operation failed.
    #[error("I/O error: {0}")]
    IoError(String),

    /// No schema files matched the configured paths.
    #[error("No schema files found matching: {0}")]
    NoSchemaFiles(String),

    /// Schema source failed to parse or used attributes the macros reject.
    #[error("Schema parse failed:\n  {0}")]
    SchemaParse(String),

    /// The previous and current snapshots use different dialects.
    #[error("Dialect mismatch between previous and current snapshots")]
    DialectMismatch,

    /// Connecting to the database failed.
    #[error("Database connection failed: {0}")]
    ConnectionError(String),

    /// Planning, verifying, or applying migrations failed.
    #[error("Migration failed: {0}")]
    MigrationError(String),

    /// A command that needs a live database found no credentials.
    #[error(
        "No database credentials configured: `{0}` needs a database connection \
         (add a [dbCredentials] section to drizzle.config.toml)"
    )]
    MissingCredentials(&'static str),

    /// The user declined to continue, so nothing was applied.
    #[error("{0}")]
    Aborted(String),

    /// The CLI was built without the driver feature this database needs.
    #[error("No driver available for {dialect}. Build with '{feature}' feature enabled.")]
    MissingDriver {
        /// Database kind, for the message.
        dialect: &'static str,
        /// Cargo feature to enable.
        feature: &'static str,
    },

    /// Operation not supported for this driver.
    ///
    /// Used for drivers the CLI cannot drive for this operation, such as
    /// `aws-data-api` (not wired up yet) and MySQL migration repair.
    #[error("{operation} is not supported for driver '{driver}'. {hint}")]
    UnsupportedForDriver {
        /// What was attempted.
        operation: &'static str,
        /// The driver in use.
        driver: &'static str,
        /// What to do instead.
        hint: &'static str,
    },

    /// Any other failure, with its message.
    #[error("{0}")]
    Other(String),
}
