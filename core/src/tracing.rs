//! Tracing utilities for drizzle query and transaction observability.
//!
//! Enable the `tracing` feature to emit spans and events via the `tracing` crate.
//! These macros no-op when the feature is disabled, avoiding `#[cfg]` boilerplate
//! at every call site.

/// Emit a debug-level tracing event with the SQL text and parameter count.
///
/// ```ignore
/// drizzle_trace_query!(&sql_str, params.len());
/// ```
#[macro_export]
macro_rules! drizzle_trace_query {
    ($sql:expr, $param_count:expr) => {
        #[cfg(feature = "tracing")]
        tracing::debug!(sql = %$sql, params = $param_count, "drizzle.query");
    };
}

/// Emit an info-level tracing event for transaction lifecycle (begin, commit, rollback).
///
/// ```ignore
/// drizzle_trace_tx!("begin", "sqlite.rusqlite");
/// drizzle_trace_tx!("commit", "postgres.sync");
/// ```
#[macro_export]
macro_rules! drizzle_trace_tx {
    ($event:literal, $driver:literal) => {
        #[cfg(feature = "tracing")]
        tracing::info!(event = $event, driver = $driver, "drizzle.transaction");
    };
}
