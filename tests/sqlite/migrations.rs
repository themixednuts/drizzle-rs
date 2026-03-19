#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
use drizzle::sqlite::prelude::*;
#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
use drizzle_migrations::{Migration, Tracking};

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
#[SQLiteTable(NAME = "push_users")]
struct PushUser {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    email: Option<String>,
}

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
#[derive(SQLiteSchema)]
struct PushSchema {
    push_user: PushUser,
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_deduplicates_by_created_at() {
    let db = crate::common::helpers::rusqlite_setup::setup_empty();

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

    let second_table_exists =
        crate::common::helpers::rusqlite_setup::table_exists(db.conn(), "runtime_created_at_b");
    assert_eq!(
        second_table_exists, 0,
        "second migration SQL should not execute when created_at is already applied"
    );
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_upgrades_legacy_tracking_table() {
    let db = crate::common::helpers::rusqlite_setup::setup_empty();
    crate::common::helpers::rusqlite_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    );
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            rusqlite::params!["runtime_hash_a", 1_680_271_923_000_i64],
        )
        .expect("insert legacy migration row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    db.migrate(&[migration], Tracking::SQLITE)
        .expect("upgrade legacy runtime metadata");

    let columns = crate::common::helpers::rusqlite_setup::legacy_tracking_columns(
        db.conn(),
        "__drizzle_migrations",
    );
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

    let migrated_table_exists =
        crate::common::helpers::rusqlite_setup::table_exists(db.conn(), "runtime_created_at_a");
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let db = crate::common::helpers::rusqlite_setup::setup_empty();
    crate::common::helpers::rusqlite_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    );
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            rusqlite::params!["runtime_hash_b", 1_680_271_923_000_i64],
        )
        .expect("insert legacy migration row");

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
    let db = crate::common::helpers::rusqlite_setup::setup_empty();
    crate::common::helpers::rusqlite_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    );
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            rusqlite::params!["unknown_hash", 1_680_271_924_000_i64],
        )
        .expect("insert unmatched legacy row");

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

    let columns = crate::common::helpers::rusqlite_setup::legacy_tracking_columns(
        db.conn(),
        "__drizzle_migrations",
    );
    assert_eq!(columns, vec!["id", "hash", "created_at"]);
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_runtime_migrate_upgrades_legacy_tracking_table() {
    let db = crate::common::helpers::libsql_setup::setup_empty().await;
    crate::common::helpers::libsql_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            libsql::params!["runtime_hash_a", 1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    db.migrate(&[migration], Tracking::SQLITE)
        .await
        .expect("upgrade legacy runtime metadata");

    let columns = crate::common::helpers::libsql_setup::legacy_tracking_columns(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    assert_eq!(
        columns,
        vec!["id", "hash", "created_at", "name", "applied_at"],
        "tracking table should be upgraded in place"
    );

    let mut rows = db
        .conn()
        .query(
            "SELECT name, applied_at FROM __drizzle_migrations LIMIT 1",
            (),
        )
        .await
        .expect("query upgraded migration row");
    let row = rows
        .next()
        .await
        .expect("next upgraded row")
        .expect("upgraded row");
    let name = row.get::<String>(0).expect("migration name");
    let applied_at = row.get::<Option<String>>(1).ok().flatten();
    assert_eq!(name, "20230331141203_runtime_first");
    assert_eq!(
        applied_at, None,
        "backfilled legacy rows keep NULL applied_at"
    );

    let migrated_table_exists =
        crate::common::helpers::libsql_setup::table_exists(db.conn(), "runtime_created_at_a").await;
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let db = crate::common::helpers::libsql_setup::setup_empty().await;
    crate::common::helpers::libsql_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            libsql::params!["runtime_hash_b", 1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

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
        .await
        .expect("upgrade legacy runtime metadata with timestamp collision");

    let mut rows = db
        .conn()
        .query("SELECT name FROM __drizzle_migrations LIMIT 1", ())
        .await
        .expect("query upgraded migration name");
    let row = rows
        .next()
        .await
        .expect("next upgraded row")
        .expect("upgraded row");
    let name = row.get::<String>(0).expect("migration name");
    assert_eq!(name, "20230331141203_runtime_beta");
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_runtime_migrate_upgrade_rejects_unmatched_legacy_rows() {
    let db = crate::common::helpers::libsql_setup::setup_empty().await;
    crate::common::helpers::libsql_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            libsql::params!["unknown_hash", 1_680_271_924_000_i64],
        )
        .await
        .expect("insert unmatched legacy row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    let err = db
        .migrate(&[migration], Tracking::SQLITE)
        .await
        .expect_err("unmatched legacy metadata should fail");
    assert!(err.to_string().contains("do not match local migrations"));

    let columns = crate::common::helpers::libsql_setup::legacy_tracking_columns(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    assert_eq!(columns, vec!["id", "hash", "created_at"]);
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_runtime_migrate_upgrades_legacy_tracking_table() {
    let mut db = crate::common::helpers::turso_setup::setup_empty().await;
    crate::common::helpers::turso_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            turso::params!["runtime_hash_a", 1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    db.migrate(&[migration], Tracking::SQLITE)
        .await
        .expect("upgrade legacy runtime metadata");

    let columns = crate::common::helpers::turso_setup::legacy_tracking_columns(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    assert_eq!(
        columns,
        vec!["id", "hash", "created_at", "name", "applied_at"],
        "tracking table should be upgraded in place"
    );

    let mut rows = db
        .conn()
        .query(
            "SELECT name, applied_at FROM __drizzle_migrations LIMIT 1",
            (),
        )
        .await
        .expect("query upgraded migration row");
    let row = rows
        .next()
        .await
        .expect("next upgraded row")
        .expect("upgraded row");
    let name = row.get::<String>(0).expect("migration name");
    let applied_at = row.get::<Option<String>>(1).ok().flatten();
    assert_eq!(name, "20230331141203_runtime_first");
    assert_eq!(
        applied_at, None,
        "backfilled legacy rows keep NULL applied_at"
    );

    let migrated_table_exists =
        crate::common::helpers::turso_setup::table_exists(db.conn(), "runtime_created_at_a").await;
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let mut db = crate::common::helpers::turso_setup::setup_empty().await;
    crate::common::helpers::turso_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            turso::params!["runtime_hash_b", 1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

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
        .await
        .expect("upgrade legacy runtime metadata with timestamp collision");

    let mut rows = db
        .conn()
        .query("SELECT name FROM __drizzle_migrations LIMIT 1", ())
        .await
        .expect("query upgraded migration name");
    let row = rows
        .next()
        .await
        .expect("next upgraded row")
        .expect("upgraded row");
    let name = row.get::<String>(0).expect("migration name");
    assert_eq!(name, "20230331141203_runtime_beta");
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_runtime_migrate_upgrade_rejects_unmatched_legacy_rows() {
    let mut db = crate::common::helpers::turso_setup::setup_empty().await;
    crate::common::helpers::turso_setup::create_legacy_tracking_table(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
            turso::params!["unknown_hash", 1_680_271_924_000_i64],
        )
        .await
        .expect("insert unmatched legacy row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    );

    let err = db
        .migrate(&[migration], Tracking::SQLITE)
        .await
        .expect_err("unmatched legacy metadata should fail");
    assert!(err.to_string().contains("do not match local migrations"));

    let columns = crate::common::helpers::turso_setup::legacy_tracking_columns(
        db.conn(),
        "__drizzle_migrations",
    )
    .await;
    assert_eq!(columns, vec!["id", "hash", "created_at"]);
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_push_creates_table() {
    let (db, schema) =
        crate::common::helpers::rusqlite_setup::setup_empty_db(PushSchema::default());

    db.push(&schema).expect("push schema");

    let table_exists =
        crate::common::helpers::rusqlite_setup::table_exists(db.conn(), "push_users");
    assert_eq!(table_exists, 1, "push should create the push_users table");
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_push_is_idempotent() {
    let (db, schema) =
        crate::common::helpers::rusqlite_setup::setup_empty_db(PushSchema::default());

    db.push(&schema).expect("first push");
    db.push(&schema).expect("second push should be a no-op");
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_push_table_is_usable() {
    let (db, schema) =
        crate::common::helpers::rusqlite_setup::setup_empty_db(PushSchema::default());

    db.push(&schema).expect("push schema");
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

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_push_creates_table() {
    let (db, schema) =
        crate::common::helpers::libsql_setup::setup_empty_db(PushSchema::default()).await;

    db.push(&schema).await.expect("push schema");

    let table_exists =
        crate::common::helpers::libsql_setup::table_exists(db.conn(), "push_users").await;
    assert_eq!(table_exists, 1, "push should create the push_users table");
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_push_is_idempotent() {
    let (db, schema) =
        crate::common::helpers::libsql_setup::setup_empty_db(PushSchema::default()).await;

    db.push(&schema).await.expect("first push");
    db.push(&schema)
        .await
        .expect("second push should be a no-op");
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_push_table_is_usable() {
    let (db, schema) =
        crate::common::helpers::libsql_setup::setup_empty_db(PushSchema::default()).await;

    db.push(&schema).await.expect("push schema");
    db.conn()
        .execute("INSERT INTO push_users (id, name) VALUES (1, 'Alice')", ())
        .await
        .expect("insert into pushed table");

    let mut rows = db
        .conn()
        .query("SELECT name FROM push_users WHERE id = 1", ())
        .await
        .expect("select from pushed table");
    let row = rows
        .next()
        .await
        .expect("next selected row")
        .expect("selected row");
    let name = row.get::<String>(0).expect("selected name");
    assert_eq!(name, "Alice");
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_push_creates_table() {
    let (db, schema) =
        crate::common::helpers::turso_setup::setup_empty_db(PushSchema::default()).await;

    db.push(&schema).await.expect("push schema");

    let table_exists =
        crate::common::helpers::turso_setup::table_exists(db.conn(), "push_users").await;
    assert_eq!(table_exists, 1, "push should create the push_users table");
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_push_is_idempotent() {
    let (db, schema) =
        crate::common::helpers::turso_setup::setup_empty_db(PushSchema::default()).await;

    db.push(&schema).await.expect("first push");
    db.push(&schema)
        .await
        .expect("second push should be a no-op");
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_push_table_is_usable() {
    let (db, schema) =
        crate::common::helpers::turso_setup::setup_empty_db(PushSchema::default()).await;

    db.push(&schema).await.expect("push schema");
    db.conn()
        .execute("INSERT INTO push_users (id, name) VALUES (1, 'Alice')", ())
        .await
        .expect("insert into pushed table");

    let mut rows = db
        .conn()
        .query("SELECT name FROM push_users WHERE id = 1", ())
        .await
        .expect("select from pushed table");
    let row = rows
        .next()
        .await
        .expect("next selected row")
        .expect("selected row");
    let name = row.get::<String>(0).expect("selected name");
    assert_eq!(name, "Alice");
}
