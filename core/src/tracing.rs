//! Macros that emit `tracing` events for queries and transactions.
//!
//! The macros expand to `tracing` calls when the *calling* crate has a
//! `tracing` feature enabled, and to nothing otherwise, so call sites need
//! no `#[cfg]`.

/// Emits a debug-level `drizzle.query` event with the SQL text and parameter
/// count.
///
/// # Examples
///
/// ```
/// let sql = "SELECT * FROM users WHERE id = ?";
/// drizzle_core::drizzle_trace_query!(sql, 1);
/// ```
#[macro_export]
macro_rules! drizzle_trace_query {
    ($sql:expr, $param_count:expr) => {
        #[cfg(feature = "tracing")]
        tracing::debug!(sql = %$sql, params = $param_count, "drizzle.query");
    };
}

/// Emits an info-level `drizzle.transaction` event for a transaction step
/// (`begin`, `commit`, `rollback`).
///
/// # Examples
///
/// ```
/// drizzle_core::drizzle_trace_tx!("begin", "sqlite.rusqlite");
/// drizzle_core::drizzle_trace_tx!("commit", "sqlite.rusqlite");
/// ```
#[macro_export]
macro_rules! drizzle_trace_tx {
    ($event:literal, $driver:literal) => {
        #[cfg(feature = "tracing")]
        tracing::info!(event = $event, driver = $driver, "drizzle.transaction");
    };
}
