//! Typestate markers for dialects and connections
//!
//! This module defines marker types used in the typestate pattern for
//! compile-time validation of configuration state.

use drizzle_types::Dialect;

// =============================================================================
// Schema Markers (Typestate)
// =============================================================================

/// Marker for no schema set
pub struct NoSchema;

// =============================================================================
// Dialect Markers (Typestate)
// =============================================================================

/// Marker for no dialect selected
pub struct NoDialect;

/// Marker for SQLite dialect
pub struct SqliteDialect;

/// Marker for PostgreSQL dialect  
pub struct PostgresDialect;

/// Marker for MySQL dialect
pub struct MysqlDialect;

/// Trait for dialect markers
pub trait DialectMarker {
    const DIALECT: Dialect;
}

impl DialectMarker for SqliteDialect {
    const DIALECT: Dialect = Dialect::SQLite;
}

impl DialectMarker for PostgresDialect {
    const DIALECT: Dialect = Dialect::PostgreSQL;
}

impl DialectMarker for MysqlDialect {
    const DIALECT: Dialect = Dialect::MySQL;
}

// =============================================================================
// Connection Markers (Typestate)
// =============================================================================

/// Marker for no connection configured
pub struct NoConnection;

/// Marker for rusqlite connection (file-based SQLite)
#[cfg(feature = "rusqlite")]
pub struct RusqliteConnection;

/// Marker for libsql connection (embedded replica)
#[cfg(feature = "libsql")]
pub struct LibsqlConnection;

/// Marker for turso connection (edge SQLite with auth)
#[cfg(feature = "turso")]
pub struct TursoConnection;

/// Marker for tokio-postgres connection (async PostgreSQL)
#[cfg(feature = "tokio-postgres")]
pub struct TokioPostgresConnection;

/// Marker for sync postgres connection  
#[cfg(feature = "postgres-sync")]
pub struct PostgresSyncConnection;

// =============================================================================
// Output Directory Markers (Typestate)
// =============================================================================

/// Marker for output directory not set
pub struct OutNotSet;

/// Marker for output directory set  
pub struct OutSet;
