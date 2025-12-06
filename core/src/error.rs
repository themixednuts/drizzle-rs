//! Error types for drizzle-core

use thiserror::Error;

/// Core error type for drizzle operations
#[derive(Debug, Error)]
pub enum DrizzleError {
    /// Error executing a query
    #[error("Execution error: {0}")]
    ExecutionError(compact_str::CompactString),

    /// Error preparing a statement
    #[error("Prepare error: {0}")]
    PrepareError(compact_str::CompactString),

    /// No rows returned when at least one was expected
    #[error("No rows found")]
    NotFound,

    /// Error with transaction
    #[error("Transaction error: {0}")]
    TransactionError(compact_str::CompactString),

    /// Error mapping data
    #[error("Mapping error: {0}")]
    Mapping(compact_str::CompactString),

    /// Error in statement
    #[error("Statement error: {0}")]
    Statement(compact_str::CompactString),

    /// Error in query
    #[error("Query error: {0}")]
    Query(compact_str::CompactString),

    /// Error converting parameters
    #[error("Parameter conversion error: {0}")]
    ParameterError(compact_str::CompactString),

    /// Integer conversion error
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] core::num::TryFromIntError),

    /// Parse int error
    #[error("Parse int error: {0}")]
    ParseInt(#[from] core::num::ParseIntError),

    /// Parse float error
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] core::num::ParseFloatError),

    /// Parse bool error
    #[error("Parse bool error: {0}")]
    ParseBool(#[from] core::str::ParseBoolError),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    ConversionError(compact_str::CompactString),

    /// Generic error
    #[error("Database error: {0}")]
    Other(compact_str::CompactString),

    /// Rusqlite specific errors
    #[cfg(feature = "rusqlite")]
    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    /// Turso specific errors
    #[cfg(feature = "turso")]
    #[error("Turso error: {0}")]
    Turso(#[from] turso::Error),

    /// LibSQL specific errors
    #[cfg(feature = "libsql")]
    #[error("LibSQL error: {0}")]
    LibSQL(#[from] libsql::Error),

    /// Postgres specific errors
    #[cfg(feature = "tokio-postgres")]
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    #[error("Postgres error: {0}")]
    Postgres(#[from] postgres::Error),

    /// UUID parsing error
    #[cfg(feature = "uuid")]
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),

    /// JSON serialization/deserialization error
    #[cfg(feature = "serde")]
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Infallible conversion error (should never happen)
    #[error("Infallible conversion error")]
    Infallible(#[from] core::convert::Infallible),
}

/// Result type for database operations
pub type Result<T> = core::result::Result<T, DrizzleError>;
