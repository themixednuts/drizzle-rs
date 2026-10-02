//! The error type shared by every Drizzle crate and driver.

use crate::prelude::{Box, String, ToString, Vec, format};
use compact_str::CompactString;
use thiserror::Error;

const MAX_CONTEXT_PARAMS: usize = 32;
const MAX_CONTEXT_PARAM_CHARS: usize = 128;

/// The SQL and parameters of a failed query, kept for the error message.
///
/// Attach it with [`ResultExt::with_query`].
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
    /// Captures `sql` and the `Debug` form of each parameter.
    ///
    /// At most 32 parameters are kept, each cut to 128 characters, so the
    /// error stays small.
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

/// Every error a Drizzle query, conversion or migration can return.
///
/// Driver errors are wrapped in a variant for that driver when its feature
/// is enabled. [`DrizzleError::QueryFailed`] adds the failing SQL and
/// parameters to another error.
#[derive(Debug, Error)]
pub enum DrizzleError {
    /// The statement failed to execute.
    #[error("Execution error: {0}")]
    ExecutionError(compact_str::CompactString),

    /// The statement failed to prepare.
    #[error("Prepare error: {0}")]
    PrepareError(compact_str::CompactString),

    /// No row was returned where one was expected (for example by `.get()`).
    #[error("No rows found")]
    NotFound,

    /// A transaction could not begin, commit or roll back.
    #[error("Transaction error: {0}")]
    TransactionError(compact_str::CompactString),

    /// A row could not be mapped to the target type.
    #[error("Mapping error: {0}")]
    Mapping(compact_str::CompactString),

    /// The statement is invalid.
    #[error("Statement error: {0}")]
    Statement(compact_str::CompactString),

    /// The query is invalid.
    #[error("Query error: {0}")]
    Query(CompactString),

    /// Another error, with the SQL and parameters of the query that caused
    /// it.
    #[error("{source}\n  sql: {sql}\n  params: {params}", sql = .ctx.sql, params = .ctx.params_display())]
    QueryFailed {
        /// Captured SQL and parameter context.
        ctx: Box<QueryContext>,
        /// Original error.
        #[source]
        source: Box<DrizzleError>,
    },

    /// Parameters could not be bound: missing, duplicate or unexpected
    /// bindings, or a value that could not be converted.
    #[error("Parameter conversion error: {0}")]
    ParameterError(compact_str::CompactString),

    /// An integer did not fit the target type.
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] core::num::TryFromIntError),

    /// Text could not be parsed as an integer.
    #[error("Parse int error: {0}")]
    ParseInt(#[from] core::num::ParseIntError),

    /// Text could not be parsed as a float.
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] core::num::ParseFloatError),

    /// Text could not be parsed as a boolean.
    #[error("Parse bool error: {0}")]
    ParseBool(#[from] core::str::ParseBoolError),

    /// A value could not be converted to the requested type.
    #[error("Type conversion error: {0}")]
    ConversionError(compact_str::CompactString),

    /// The schema is invalid (for example a cycle in table dependencies).
    #[error("Schema error: {0}")]
    Schema(compact_str::CompactString),

    /// A dirty migration cannot be repaired without violating its execution
    /// safety contract.
    #[error("Migration `{tag}` cannot be repaired safely: {reason}")]
    UnsafeMigrationRepair {
        /// Migration tag recorded in the tracking table.
        tag: CompactString,
        /// Safety requirement that prevents automatic repair.
        reason: CompactString,
    },

    /// The selected adapter cannot provide a migration's required execution
    /// semantics.
    #[error("{adapter} cannot execute this migration: {requirement}")]
    UnsupportedMigrationExecution {
        /// Runtime adapter that cannot provide the required semantics.
        adapter: CompactString,
        /// Migration execution capability the adapter does not provide.
        requirement: CompactString,
    },

    /// Any other error.
    #[error("Database error: {0}")]
    Other(compact_str::CompactString),

    /// Error returned by a wire driver whose concrete type is intentionally
    /// kept out of drizzle-core's public dependency graph.
    #[cfg(feature = "driver-error")]
    #[error("{driver} error: {source}")]
    Driver {
        /// Stable adapter name used in diagnostics.
        driver: CompactString,
        /// Original driver error, retained as the error source.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error from a higher-level subsystem kept as a typed source.
    #[cfg(feature = "driver-error")]
    #[error("{context}: {source}")]
    External {
        /// Operation or subsystem that failed.
        context: CompactString,
        /// Original error, retained as the error source.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// An error from `rusqlite`.
    #[cfg(feature = "rusqlite")]
    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    /// An error from `turso`.
    #[cfg(feature = "turso")]
    #[error("Turso error: {0}")]
    Turso(#[from] turso::Error),

    /// An error from `libsql`.
    #[cfg(feature = "libsql")]
    #[error("LibSQL error: {0}")]
    LibSQL(#[from] libsql::Error),

    /// An error from `tokio-postgres`.
    #[cfg(feature = "tokio-postgres")]
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    /// An error from `postgres`.
    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    #[error("Postgres error: {0}")]
    Postgres(#[from] postgres::Error),

    /// A UUID could not be parsed.
    #[cfg(feature = "uuid")]
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),

    /// A JSON value could not be serialized or deserialized.
    #[cfg(feature = "serde")]
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Never constructed; lets `?` work on infallible conversions.
    #[error("Infallible conversion error")]
    Infallible(#[from] core::convert::Infallible),
}

impl DrizzleError {
    /// Wraps a concrete wire-driver error without exposing its type in the
    /// public error enum.
    #[cfg(feature = "driver-error")]
    pub fn driver(
        driver: impl Into<CompactString>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Driver {
            driver: driver.into(),
            source: Box::new(source),
        }
    }

    /// Wraps an external subsystem error while preserving its source chain.
    #[cfg(feature = "driver-error")]
    pub fn external(
        context: impl Into<CompactString>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::External {
            context: context.into(),
            source: Box::new(source),
        }
    }
}

/// `Result` with [`DrizzleError`] as the error type.
pub type Result<T> = core::result::Result<T, DrizzleError>;

/// Adds the failing query's SQL and parameters to an error.
///
/// # Examples
///
/// ```
/// use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
///
/// let result: Result<(), DrizzleError> = Err(DrizzleError::NotFound);
/// let error = result
///     .with_query(|| QueryContext::new("SELECT * FROM users WHERE id = ?", &[&7]))
///     .unwrap_err();
///
/// assert_eq!(
///     error.to_string(),
///     "No rows found\n  sql: SELECT * FROM users WHERE id = ?\n  params: [7]",
/// );
/// ```
pub trait ResultExt<T> {
    /// On error, wraps it in [`DrizzleError::QueryFailed`] with the context
    /// from `ctx`. `ctx` only runs on the error path, and an error that
    /// already has context keeps it.
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

#[cfg(all(test, feature = "driver-error"))]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn external_errors_keep_their_source() {
        let error =
            DrizzleError::external("schema diff", std::io::Error::other("invalid snapshot"));

        assert_eq!(error.to_string(), "schema diff: invalid snapshot");
        assert_eq!(
            error.source().map(ToString::to_string),
            Some("invalid snapshot".to_string())
        );
    }
}
