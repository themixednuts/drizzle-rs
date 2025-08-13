use thiserror::Error;

#[derive(Debug, Error)]
pub enum DrizzleError {
    /// Error executing a query
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Error preparing a statement
    #[error("Prepare error: {0}")]
    PrepareError(String),

    /// No rows returned when at least one was expected
    #[error("No rows found")]
    NotFound,

    /// Error with transaction
    #[error("Transaction error: {0}")]
    TransactionError(String),

    /// Error mapping data
    #[error("Mapping error: {0}")]
    Mapping(String),

    /// Error in statement
    #[error("Statement error: {0}")]
    Statement(String),

    /// Error in query
    #[error("Query error: {0}")]
    Query(String),

    /// Error converting parameters (e.g. JSON serialization failure)
    #[error("Parameter conversion error: {0}")]
    ParameterError(String),

    /// Integer conversion error
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),

    /// Parse int error
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// Parse float error
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),

    /// Parse bool error
    #[error("Parse bool error: {0}")]
    ParseBool(#[from] std::str::ParseBoolError),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    ConversionError(String),

    /// Generic error
    #[error("Database error: {0}")]
    Other(String),

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
    Infallible(#[from] std::convert::Infallible),
}

/// Result type for database operations
pub type Result<T> = std::result::Result<T, DrizzleError>;
