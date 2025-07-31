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

    /// Generic error
    #[error("Database error: {0}")]
    Other(String),

    /// Rusqlite specific errors
    #[cfg(feature = "rusqlite")]
    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
}

/// Result type for database operations
pub type Result<T> = std::result::Result<T, DrizzleError>;
