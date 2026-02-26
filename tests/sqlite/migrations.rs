#[cfg(feature = "rusqlite")]
use drizzle::sqlite::prelude::*;
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

// -- push() tests --

#[cfg(feature = "rusqlite")]
#[SQLiteTable(NAME = "push_users")]
struct PushUser {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    email: Option<String>,
}

#[cfg(feature = "rusqlite")]
#[derive(SQLiteSchema)]
struct PushSchema {
    push_user: PushUser,
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_push_creates_table() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    let (db, schema) = Drizzle::new(conn, PushSchema::default());

    db.push(&schema).expect("push schema");

    let table_exists: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='push_users'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(table_exists, 1, "push should create the push_users table");
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_push_is_idempotent() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    let (db, schema) = Drizzle::new(conn, PushSchema::default());

    db.push(&schema).expect("first push");
    db.push(&schema).expect("second push should be a no-op");
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_push_table_is_usable() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    let (db, schema) = Drizzle::new(conn, PushSchema::default());

    db.push(&schema).expect("push schema");

    // Insert a row via raw SQL to confirm the table is real and usable
    db.conn()
        .execute("INSERT INTO push_users (id, name) VALUES (1, 'Alice')", [])
        .expect("insert into pushed table");

    let name: String = db
        .conn()
        .query_row("SELECT name FROM push_users WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("select from pushed table");
    assert_eq!(name, "Alice");
}
