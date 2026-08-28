//! Schema trait for type-safe schema definitions
//!
//! This module provides the `Schema` trait that user-defined schema structs
//! implement (via derive macros) to enable migration generation.

use crate::mysql::MySQLSnapshot;
use crate::postgres::PostgresSnapshot;
use crate::sqlite::SQLiteSnapshot;
use drizzle_types::Dialect;

/// A unified snapshot type for every migration-capable SQL dialect.
///
/// This is returned by `Schema::to_snapshot()` and used by the migration
/// generation logic to diff against previous snapshots.
#[derive(Clone, Debug)]
pub enum Snapshot {
    /// `SQLite` schema snapshot
    Sqlite(SQLiteSnapshot),
    /// `PostgreSQL` schema snapshot
    Postgres(PostgresSnapshot),
    /// `MySQL` schema snapshot
    MySQL(MySQLSnapshot),
}

impl Snapshot {
    /// Get the dialect of this snapshot
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        match self {
            Self::Sqlite(_) => Dialect::SQLite,
            Self::Postgres(_) => Dialect::PostgreSQL,
            Self::MySQL(_) => Dialect::MySQL,
        }
    }

    /// Save the snapshot to a file.
    ///
    /// # Errors
    ///
    /// Returns any [`std::io::Error`] produced by the underlying dialect-specific
    /// save operation (e.g., serialization failure or filesystem I/O failure).
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        match self {
            Self::Sqlite(s) => s.save(path),
            Self::Postgres(s) => s.save(path),
            Self::MySQL(s) => s.save(path),
        }
    }

    /// Load a snapshot from a file.
    ///
    /// # Errors
    ///
    /// Returns any [`std::io::Error`] produced by the dialect-specific load
    /// operation (file not found, parse failure, etc.).
    pub fn load(path: &std::path::Path, dialect: Dialect) -> std::io::Result<Self> {
        match dialect {
            Dialect::SQLite => Ok(Self::Sqlite(SQLiteSnapshot::load(path)?)),
            Dialect::PostgreSQL => Ok(Self::Postgres(PostgresSnapshot::load(path)?)),
            Dialect::MySQL => Ok(Self::MySQL(crate::mysql::snapshot::load(path)?)),
        }
    }

    /// Create an empty snapshot for the given dialect.
    ///
    #[must_use]
    pub fn empty(dialect: Dialect) -> Self {
        match dialect {
            Dialect::SQLite => Self::Sqlite(SQLiteSnapshot::new()),
            Dialect::PostgreSQL => Self::Postgres(PostgresSnapshot::new()),
            Dialect::MySQL => Self::MySQL(MySQLSnapshot::new()),
        }
    }

    /// Check if this snapshot is empty (no entities)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        match self {
            Self::Sqlite(s) => s.is_empty(),
            Self::Postgres(s) => s.ddl.is_empty(),
            Self::MySQL(s) => s.ddl.is_empty(),
        }
    }

    /// Get the ID of this snapshot
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Sqlite(s) => &s.id,
            Self::Postgres(s) => &s.id,
            Self::MySQL(s) => &s.id,
        }
    }

    /// Get the previous IDs chain
    #[must_use]
    pub fn prev_ids(&self) -> &[String] {
        match self {
            Self::Sqlite(s) => &s.prev_ids,
            Self::Postgres(s) => &s.prev_ids,
            Self::MySQL(s) => &s.prev_ids,
        }
    }

    /// Set the previous IDs chain
    pub fn set_prev_ids(&mut self, prev_ids: Vec<String>) {
        match self {
            Self::Sqlite(s) => s.prev_ids = prev_ids,
            Self::Postgres(s) => s.prev_ids = prev_ids,
            Self::MySQL(s) => s.prev_ids = prev_ids,
        }
    }

    /// Get the snapshot as `SQLite` if it is one
    #[must_use]
    pub const fn as_sqlite(&self) -> Option<&SQLiteSnapshot> {
        match self {
            Self::Sqlite(s) => Some(s),
            Self::Postgres(_) => None,
            Self::MySQL(_) => None,
        }
    }

    /// Get the snapshot as `PostgreSQL` if it is one
    #[must_use]
    pub const fn as_postgres(&self) -> Option<&PostgresSnapshot> {
        match self {
            Self::Postgres(s) => Some(s),
            Self::Sqlite(_) | Self::MySQL(_) => None,
        }
    }

    /// Get the snapshot as `MySQL` if it is one.
    #[must_use]
    pub const fn as_mysql(&self) -> Option<&MySQLSnapshot> {
        match self {
            Self::MySQL(snapshot) => Some(snapshot),
            Self::Sqlite(_) | Self::Postgres(_) => None,
        }
    }
}

/// Trait for database schemas that can be used with Drizzle migrations.
///
/// This trait is automatically implemented by the `#[derive(PostgresSchema)]`,
/// `#[derive(SQLiteSchema)]`, and `#[derive(MySQLSchema)]` macros. It converts a
/// schema definition into a snapshot for migration generation.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle::postgres::prelude::*;
///
/// #[derive(PostgresSchema)]
/// pub struct AppSchema {
///     pub users: Users,
///     pub posts: Posts,
/// }
///
/// // The macro implements Schema for AppSchema
/// let schema = AppSchema::default();
/// let snapshot = schema.to_snapshot();
/// # "####;
/// ```
pub trait Schema: Default + Sized {
    /// The dialect this schema targets (sqlite, postgresql, etc.)
    fn dialect(&self) -> Dialect;

    /// Convert the schema to a snapshot for migration diffing.
    ///
    /// This method traverses all table and index definitions in the schema
    /// and produces a snapshot representing the current state.
    fn to_snapshot(&self) -> Snapshot;

    /// Get an optional schema name (for `PostgreSQL` multi-schema support).
    ///
    /// Defaults to `None`, meaning the default schema will be used.
    fn schema_name(&self) -> Option<&'static str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot_sqlite() {
        let snapshot = Snapshot::empty(Dialect::SQLite);
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.dialect(), Dialect::SQLite);
    }

    #[test]
    fn test_empty_snapshot_postgres() {
        let snapshot = Snapshot::empty(Dialect::PostgreSQL);
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.dialect(), Dialect::PostgreSQL);
    }

    #[test]
    fn test_empty_snapshot_mysql() {
        let snapshot = Snapshot::empty(Dialect::MySQL);
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.dialect(), Dialect::MySQL);
    }
}
