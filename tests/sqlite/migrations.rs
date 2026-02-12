#[cfg(feature = "rusqlite")]
use drizzle::sqlite::rusqlite::Drizzle;
#[cfg(feature = "rusqlite")]
use drizzle_migrations::{Migration, MigrationSet};
#[cfg(feature = "rusqlite")]
use drizzle_types::Dialect;

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_deduplicates_by_created_at() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    let (db, _) = Drizzle::new(conn, ());

    let first_set = MigrationSet::new(
        vec![Migration::with_hash(
            "20230331141203_runtime_first",
            "runtime_hash_a",
            1_680_271_923_000,
            vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
        )],
        Dialect::SQLite,
    );
    db.migrate(&first_set).expect("first runtime migration");

    let second_set = MigrationSet::new(
        vec![Migration::with_hash(
            "20230331141203_runtime_second",
            "runtime_hash_b",
            1_680_271_923_000,
            vec!["CREATE TABLE runtime_created_at_b (id INTEGER PRIMARY KEY)".to_string()],
        )],
        Dialect::SQLite,
    );
    db.migrate(&second_set)
        .expect("second runtime migration should no-op");

    let applied_rows: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM __drizzle_migrations", [], |row| {
            row.get(0)
        })
        .expect("count migrations rows");
    assert_eq!(applied_rows, 1);

    let second_table_exists: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='runtime_created_at_b'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(
        second_table_exists, 0,
        "second migration SQL should not execute when created_at is already applied"
    );
}
