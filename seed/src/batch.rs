//! Batch size constants for INSERT statements.
//!
//! Different databases have different limits on the number of parameters
//! in a single statement. We split inserts into batches to stay within limits.

/// Maximum parameter count for SQLite (SQLITE_MAX_VARIABLE_NUMBER default).
#[cfg(feature = "sqlite")]
pub(crate) const SQLITE_MAX_PARAMS: usize = 32766;

/// Maximum parameter count for PostgreSQL.
#[cfg(feature = "postgres")]
pub(crate) const POSTGRES_MAX_PARAMS: usize = 65535;
