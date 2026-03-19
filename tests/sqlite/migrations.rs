#[cfg(feature = "rusqlite")]
use drizzle::sqlite::prelude::*;
#[cfg(feature = "rusqlite")]
use drizzle::sqlite::rusqlite::Drizzle;
#[cfg(feature = "rusqlite")]
use drizzle_migrations::{Migration, Tracking};

#[cfg(feature = "rusqlite")]
fn legacy_tracking_columns(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
    let pragma = format!("SELECT name FROM pragma_table_info('{table}') ORDER BY cid");
    let mut stmt = conn.prepare(&pragma).expect("prepare pragma_table_info");
    stmt.query_map([], |row| row.get::<_, String>(0))
        .expect("query pragma_table_info")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect pragma columns")
}

#[cfg(feature = "rusqlite")]
fn create_legacy_tracking_table(conn: &rusqlite::Connection, table: &str) {
    conn.execute(
        &format!(
            "CREATE TABLE \"{table}\" (id INTEGER PRIMARY KEY AUTOINCREMENT, hash text NOT NULL, created_at numeric)"
        ),
        [],
    )
    .expect("create legacy tracking table");
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_deduplicates_by_created_at() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    let (db, _) = Drizzle::new(conn, ());

    let first = vec![Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    )];
    db.migrate(&first, Tracking::SQLITE)
        .expect("first runtime migration");

    let second = vec![Migration::with_hash(
        "20230331141203_runtime_second",
        "runtime_hash_b",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_b (id INTEGER PRIMARY KEY)".to_string()],
    )];
    db.migrate(&second, Tracking::SQLITE)
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

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_upgrades_legacy_tracking_table() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    create_legacy_tracking_table(&conn, "__drizzle_migrations");
    conn.execute(
        "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
        rusqlite::params!["runtime_hash_a", 1_680_271_923_000_i64],
    )
    .expect("insert legacy migration row");

    let (db, _) = Drizzle::new(conn, ());
    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    db.migrate(&[migration], Tracking::SQLITE)
        .expect("upgrade legacy runtime metadata");

    let columns = legacy_tracking_columns(db.conn(), "__drizzle_migrations");
    assert_eq!(
        columns,
        vec!["id", "hash", "created_at", "name", "applied_at"],
        "tracking table should be upgraded in place"
    );

    let (name, applied_at): (String, Option<String>) = db
        .conn()
        .query_row(
            "SELECT name, applied_at FROM __drizzle_migrations LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("select upgraded migration row");
    assert_eq!(name, "20230331141203_runtime_first");
    assert_eq!(
        applied_at, None,
        "backfilled legacy rows keep NULL applied_at"
    );

    let migrated_table_exists: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='runtime_created_at_a'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    create_legacy_tracking_table(&conn, "__drizzle_migrations");
    conn.execute(
        "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
        rusqlite::params!["runtime_hash_b", 1_680_271_923_000_i64],
    )
    .expect("insert legacy migration row");

    let (db, _) = Drizzle::new(conn, ());
    let migrations = vec![
        Migration::with_hash(
            "20230331141203_runtime_alpha",
            "runtime_hash_a",
            1_680_271_923_000,
            vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
        ),
        Migration::with_hash(
            "20230331141203_runtime_beta",
            "runtime_hash_b",
            1_680_271_923_000,
            vec!["CREATE TABLE runtime_created_at_b (id INTEGER PRIMARY KEY)".to_string()],
        ),
    ];

    db.migrate(&migrations, Tracking::SQLITE)
        .expect("upgrade legacy runtime metadata with timestamp collision");

    let name: String = db
        .conn()
        .query_row("SELECT name FROM __drizzle_migrations LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("select upgraded migration name");
    assert_eq!(name, "20230331141203_runtime_beta");
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_upgrade_rejects_unmatched_legacy_rows() {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite in-memory");
    create_legacy_tracking_table(&conn, "__drizzle_migrations");
    conn.execute(
        "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
        rusqlite::params!["unknown_hash", 1_680_271_924_000_i64],
    )
    .expect("insert unmatched legacy row");

    let (db, _) = Drizzle::new(conn, ());
    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    let err = db
        .migrate(&[migration], Tracking::SQLITE)
        .expect_err("unmatched legacy metadata should fail");
    assert!(err.to_string().contains("do not match local migrations"));

    let columns = legacy_tracking_columns(db.conn(), "__drizzle_migrations");
    assert_eq!(columns, vec!["id", "hash", "created_at"]);
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
