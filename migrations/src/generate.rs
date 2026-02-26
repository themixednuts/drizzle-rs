//! Programmatic migration generation API.
//!
//! Diff two schema snapshots and get SQL statements — no file I/O, no CLI needed.
//!
//! # Example
//!
//! ```rust
//! use drizzle_migrations::{Snapshot, generate};
//!
//! let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let current = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let statements = generate(&prev, &current).unwrap();
//! assert!(statements.is_empty());
//! ```

use crate::schema::Snapshot;
use crate::writer::MigrationError;

/// Diff two snapshots and return the migration SQL statements.
///
/// Both snapshots must be for the same dialect (e.g., both SQLite or both PostgreSQL).
/// Returns `Ok(vec![])` if no changes are detected.
///
/// This is a pure function — no file I/O, no side effects.
pub fn generate(prev: &Snapshot, current: &Snapshot) -> Result<Vec<String>, MigrationError> {
    match (prev, current) {
        (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => {
            let prev_ddl = crate::sqlite::collection::SQLiteDDL::from_entities(p.ddl.clone());
            let cur_ddl = crate::sqlite::collection::SQLiteDDL::from_entities(c.ddl.clone());
            let diff = crate::sqlite::diff::compute_migration(&prev_ddl, &cur_ddl);
            Ok(diff.sql_statements)
        }
        (Snapshot::Postgres(p), Snapshot::Postgres(c)) => {
            let diff = crate::postgres::diff::compute_migration_from_snapshots(p, c);
            Ok(diff.sql_statements)
        }
        _ => Err(MigrationError::DialectMismatch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SQLiteSnapshot;
    use crate::sqlite::ddl::{Column, SqliteEntity, Table};

    #[test]
    fn test_generate_empty_to_empty() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let cur = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let stmts = generate(&prev, &cur).unwrap();
        assert!(stmts.is_empty());
    }

    #[test]
    fn test_generate_create_table() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);

        let mut cur_snap = SQLiteSnapshot::new();
        cur_snap.add_entity(SqliteEntity::Table(Table::new("users")));
        cur_snap.add_entity(SqliteEntity::Column(
            Column::new("users", "id", "integer").not_null(),
        ));
        cur_snap.add_entity(SqliteEntity::Column(
            Column::new("users", "name", "text").not_null(),
        ));
        let cur = Snapshot::Sqlite(cur_snap);

        let stmts = generate(&prev, &cur).unwrap();
        assert!(!stmts.is_empty());
        assert!(stmts[0].contains("CREATE TABLE"));
        assert!(stmts[0].contains("users"));
    }

    #[test]
    fn test_generate_dialect_mismatch() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let cur = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let result = generate(&prev, &cur);
        assert!(matches!(result, Err(MigrationError::DialectMismatch)));
    }

    #[test]
    fn test_generate_postgres_empty() {
        let prev = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let cur = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let stmts = generate(&prev, &cur).unwrap();
        assert!(stmts.is_empty());
    }
}
