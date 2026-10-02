//! The [`Schema`] trait and the dialect-independent [`Snapshot`] type.

use crate::mysql::MySQLSnapshot;
use crate::postgres::PostgresSnapshot;
use crate::sqlite::SQLiteSnapshot;
use drizzle_types::Dialect;

/// A schema snapshot for any supported dialect.
///
/// Returned by [`Schema::to_snapshot`] and stored as `snapshot.json` in each
/// migration folder. [`diff`](crate::diff) compares two of them.
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
    /// Returns the dialect of this snapshot.
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        match self {
            Self::Sqlite(_) => Dialect::SQLite,
            Self::Postgres(_) => Dialect::PostgreSQL,
            Self::MySQL(_) => Dialect::MySQL,
        }
    }

    /// Writes the snapshot as pretty-printed JSON to `path`, creating parent
    /// folders.
    ///
    /// # Errors
    ///
    /// Returns the [`std::io::Error`] from serializing (as
    /// [`InvalidData`](std::io::ErrorKind::InvalidData)), creating the
    /// folder, or writing the file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        match self {
            Self::Sqlite(s) => s.save(path),
            Self::Postgres(s) => s.save(path),
            Self::MySQL(s) => s.save(path),
        }
    }

    /// Reads a `dialect` snapshot from the JSON file at `path`.
    ///
    /// SQLite and PostgreSQL files must already be in the current format
    /// (run `drizzle up` first); MySQL v5 files are upgraded in memory, and
    /// MySQL snapshots are validated.
    ///
    /// # Errors
    ///
    /// Returns the [`std::io::Error`] from reading the file, or one of kind
    /// [`InvalidData`](std::io::ErrorKind::InvalidData) if the contents do not
    /// parse (or, for MySQL, fail validation).
    pub fn load(path: &std::path::Path, dialect: Dialect) -> std::io::Result<Self> {
        match dialect {
            Dialect::SQLite => Ok(Self::Sqlite(SQLiteSnapshot::load(path)?)),
            Dialect::PostgreSQL => Ok(Self::Postgres(PostgresSnapshot::load(path)?)),
            Dialect::MySQL => Ok(Self::MySQL(crate::mysql::snapshot::load(path)?)),
        }
    }

    /// Creates a snapshot with no entities, the starting point for a first
    /// migration.
    #[must_use]
    pub fn empty(dialect: Dialect) -> Self {
        match dialect {
            Dialect::SQLite => Self::Sqlite(SQLiteSnapshot::new()),
            Dialect::PostgreSQL => Self::Postgres(PostgresSnapshot::new()),
            Dialect::MySQL => Self::MySQL(MySQLSnapshot::new()),
        }
    }

    /// Returns `true` if the snapshot has no entities.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        match self {
            Self::Sqlite(s) => s.is_empty(),
            Self::Postgres(s) => s.ddl.is_empty(),
            Self::MySQL(s) => s.ddl.is_empty(),
        }
    }

    /// Returns this snapshot's ID.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Sqlite(s) => &s.id,
            Self::Postgres(s) => &s.id,
            Self::MySQL(s) => &s.id,
        }
    }

    /// Returns the IDs of the snapshots this one follows.
    #[must_use]
    pub fn prev_ids(&self) -> &[String] {
        match self {
            Self::Sqlite(s) => &s.prev_ids,
            Self::Postgres(s) => &s.prev_ids,
            Self::MySQL(s) => &s.prev_ids,
        }
    }

    /// Replaces the IDs of the snapshots this one follows.
    pub fn set_prev_ids(&mut self, prev_ids: Vec<String>) {
        match self {
            Self::Sqlite(s) => s.prev_ids = prev_ids,
            Self::Postgres(s) => s.prev_ids = prev_ids,
            Self::MySQL(s) => s.prev_ids = prev_ids,
        }
    }

    /// Returns the SQLite snapshot, or `None` for other dialects.
    #[must_use]
    pub const fn as_sqlite(&self) -> Option<&SQLiteSnapshot> {
        match self {
            Self::Sqlite(s) => Some(s),
            Self::Postgres(_) => None,
            Self::MySQL(_) => None,
        }
    }

    /// Returns the PostgreSQL snapshot, or `None` for other dialects.
    #[must_use]
    pub const fn as_postgres(&self) -> Option<&PostgresSnapshot> {
        match self {
            Self::Postgres(s) => Some(s),
            Self::Sqlite(_) | Self::MySQL(_) => None,
        }
    }

    /// Returns the MySQL snapshot, or `None` for other dialects.
    #[must_use]
    pub const fn as_mysql(&self) -> Option<&MySQLSnapshot> {
        match self {
            Self::MySQL(snapshot) => Some(snapshot),
            Self::Sqlite(_) | Self::Postgres(_) => None,
        }
    }
}

/// A database schema that can produce a [`Snapshot`] for migration diffing.
///
/// You normally get this from `#[derive(SQLiteSchema)]`,
/// `#[derive(PostgresSchema)]`, or `#[derive(MySQLSchema)]` on a struct whose
/// fields are your tables and indexes; then call
/// `AppSchema::default().to_snapshot()`.
///
/// # Examples
///
/// A hand-written implementation:
///
/// ```rust
/// use drizzle_migrations::{Schema, Snapshot};
/// use drizzle_types::Dialect;
///
/// #[derive(Default)]
/// struct EmptySchema;
///
/// impl Schema for EmptySchema {
///     fn dialect(&self) -> Dialect {
///         Dialect::SQLite
///     }
///     fn to_snapshot(&self) -> Snapshot {
///         Snapshot::empty(Dialect::SQLite)
///     }
/// }
///
/// assert!(EmptySchema.to_snapshot().is_empty());
/// ```
pub trait Schema: Default + Sized {
    /// Returns the dialect this schema targets.
    fn dialect(&self) -> Dialect;

    /// Builds a snapshot of every table, index, and other entity in the
    /// schema.
    fn to_snapshot(&self) -> Snapshot;

    /// Returns the PostgreSQL schema name, or `None` (the default) for the
    /// default schema.
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
