//! Error types for drizzle-core

use crate::prelude::{Box, String, ToString, Vec, format};
use compact_str::CompactString;
use thiserror::Error;

const MAX_CONTEXT_PARAMS: usize = 32;
const MAX_CONTEXT_PARAM_CHARS: usize = 128;

/// SQL and parameter context captured when a query fails.
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// Rendered SQL statement.
    pub sql: CompactString,
    /// Debug-rendered parameter values, truncated to keep errors bounded.
    pub params: Box<[CompactString]>,
    /// Total number of parameters, including any omitted from `params`.
    pub param_count: usize,
}

impl QueryContext {
    /// Builds an owned query context from borrowed parameters.
    pub fn new<V: core::fmt::Debug>(sql: &str, params: &[&V]) -> Self {
        let rendered = params
            .iter()
            .take(MAX_CONTEXT_PARAMS)
            .map(|param| truncate_param(format!("{param:?}")))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            sql: sql.into(),
            params: rendered,
            param_count: params.len(),
        }
    }

    fn params_display(&self) -> String {
        if self.param_count == 0 {
            return "[]".to_string();
        }

        let mut rendered = String::from("[");
        for (index, param) in self.params.iter().enumerate() {
            if index > 0 {
                rendered.push_str(", ");
            }
            rendered.push_str(param.as_str());
        }
        if self.param_count > self.params.len() {
            if !self.params.is_empty() {
                rendered.push_str(", ");
            }
            rendered.push_str("...");
            rendered.push_str(&format!("(+{} more)", self.param_count - self.params.len()));
        }
        rendered.push(']');
        rendered
    }
}

fn truncate_param(mut value: String) -> CompactString {
    if value.chars().count() <= MAX_CONTEXT_PARAM_CHARS {
        return value.into();
    }

    let mut truncated = String::new();
    for ch in value.drain(..).take(MAX_CONTEXT_PARAM_CHARS) {
        truncated.push(ch);
    }
    truncated.push_str("...");
    truncated.into()
}

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
    Query(CompactString),

    /// Query error with rendered SQL and parameter context.
    #[error("{source}\n  sql: {sql}\n  params: {params}", sql = .ctx.sql, params = .ctx.params_display())]
    QueryFailed {
        /// Captured SQL and parameter context.
        ctx: Box<QueryContext>,
        /// Original error.
        #[source]
        source: Box<DrizzleError>,
    },

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

    /// Schema error (e.g. cycle in table dependencies)
    #[error("Schema error: {0}")]
    Schema(compact_str::CompactString),

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

    /// `LibSQL` specific errors
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

/// Attaches SQL and parameter context to a database error.
pub trait ResultExt<T> {
    /// Attach SQL and parameter context lazily on the error path.
    fn with_query<F>(self, ctx: F) -> Result<T>
    where
        F: FnOnce() -> QueryContext;
}

impl<T, E> ResultExt<T> for core::result::Result<T, E>
where
    E: Into<DrizzleError>,
{
    fn with_query<F>(self, ctx: F) -> Result<T>
    where
        F: FnOnce() -> QueryContext,
    {
        self.map_err(|error| {
            let source = error.into();
            match source {
                DrizzleError::QueryFailed { .. } => source,
                other => DrizzleError::QueryFailed {
                    ctx: Box::new(ctx()),
                    source: Box::new(other),
                },
            }
        })
    }
}
