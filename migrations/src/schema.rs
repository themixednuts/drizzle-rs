//! Schema trait for type-safe schema definitions
//!
//! This module provides the `Schema` trait that user-defined schema structs
//! implement (via derive macros) to enable migration generation.

use crate::postgres::PostgresSnapshot;
use crate::sqlite::SQLiteSnapshot;
use drizzle_types::Dialect;

/// A unified snapshot type that can hold either SQLite or PostgreSQL schema data.
///
/// This is returned by `Schema::to_snapshot()` and used by the migration
/// generation logic to diff against previous snapshots.
#[derive(Clone, Debug)]
pub enum Snapshot {
    /// SQLite schema snapshot
    Sqlite(SQLiteSnapshot),
    /// PostgreSQL schema snapshot
    Postgres(PostgresSnapshot),
}

impl Snapshot {
    /// Get the dialect of this snapshot
    pub fn dialect(&self) -> Dialect {
        match self {
            Snapshot::Sqlite(_) => Dialect::SQLite,
            Snapshot::Postgres(_) => Dialect::PostgreSQL,
        }
    }

    /// Save the snapshot to a file
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        match self {
            Snapshot::Sqlite(s) => s.save(path),
            Snapshot::Postgres(s) => s.save(path),
        }
    }

    /// Load a snapshot from a file
    pub fn load(path: &std::path::Path, dialect: Dialect) -> std::io::Result<Self> {
        match dialect {
            Dialect::SQLite => Ok(Snapshot::Sqlite(SQLiteSnapshot::load(path)?)),
            Dialect::PostgreSQL => Ok(Snapshot::Postgres(PostgresSnapshot::load(path)?)),
            Dialect::MySQL => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "MySQL snapshots not yet supported",
            )),
        }
    }

    /// Create an empty snapshot for the given dialect
    pub fn empty(dialect: Dialect) -> Self {
        match dialect {
            Dialect::SQLite => Snapshot::Sqlite(SQLiteSnapshot::new()),
            Dialect::PostgreSQL => Snapshot::Postgres(PostgresSnapshot::new()),
            Dialect::MySQL => {
                // TODO: Add MySQL support
                panic!("MySQL not yet supported")
            }
        }
    }

    /// Check if this snapshot is empty (no entities)
    pub fn is_empty(&self) -> bool {
        match self {
            Snapshot::Sqlite(s) => s.is_empty(),
            Snapshot::Postgres(s) => s.ddl.is_empty(),
        }
    }

    /// Get the ID of this snapshot
    pub fn id(&self) -> &str {
        match self {
            Snapshot::Sqlite(s) => &s.id,
            Snapshot::Postgres(s) => &s.id,
        }
    }

    /// Get the previous IDs chain
    pub fn prev_ids(&self) -> &[String] {
        match self {
            Snapshot::Sqlite(s) => &s.prev_ids,
            Snapshot::Postgres(s) => &s.prev_ids,
        }
    }

    /// Set the previous IDs chain
    pub fn set_prev_ids(&mut self, prev_ids: Vec<String>) {
        match self {
            Snapshot::Sqlite(s) => s.prev_ids = prev_ids,
            Snapshot::Postgres(s) => s.prev_ids = prev_ids,
        }
    }

    /// Get the snapshot as SQLite if it is one
    pub fn as_sqlite(&self) -> Option<&SQLiteSnapshot> {
        match self {
            Snapshot::Sqlite(s) => Some(s),
            _ => None,
        }
    }

    /// Get the snapshot as PostgreSQL if it is one
    pub fn as_postgres(&self) -> Option<&PostgresSnapshot> {
        match self {
            Snapshot::Postgres(s) => Some(s),
            _ => None,
        }
    }
}

/// Trait for database schemas that can be used with Drizzle migrations.
///
/// This trait is automatically implemented by the `#[derive(PostgresSchema)]`
/// and `#[derive(SQLiteSchema)]` macros. It provides the ability to convert
/// a schema definition into a snapshot for migration generation.
///
/// # Example
///
/// ```ignore
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
/// ```
pub trait Schema: Default + Sized {
    /// The dialect this schema targets (sqlite, postgresql, etc.)
    fn dialect(&self) -> Dialect;

    /// Convert the schema to a snapshot for migration diffing.
    ///
    /// This method traverses all table and index definitions in the schema
    /// and produces a snapshot representing the current state.
    fn to_snapshot(&self) -> Snapshot;

    /// Get an optional schema name (for PostgreSQL multi-schema support).
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
}
