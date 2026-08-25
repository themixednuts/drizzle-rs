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

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
fn generated_style_rebuild_migration() -> Migration {
    Migration::new(
        "20260824000000_strict_parent_rebuild",
        "PRAGMA foreign_keys=OFF;
         --> statement-breakpoint
         CREATE TABLE rebuild_parent_new (id INTEGER PRIMARY KEY) STRICT;
         --> statement-breakpoint
         INSERT INTO rebuild_parent_new SELECT * FROM rebuild_parent;
         --> statement-breakpoint
         DROP TABLE rebuild_parent;
         --> statement-breakpoint
         ALTER TABLE rebuild_parent_new RENAME TO rebuild_parent;
         --> statement-breakpoint
         PRAGMA foreign_keys=ON;",
    )
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_rebuild_preserves_cascade_children_and_restores_foreign_keys() {
    let db = crate::common::helpers::rusqlite_setup::setup_empty();
    db.conn()
        .execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE rebuild_parent (id INTEGER PRIMARY KEY);
             CREATE TABLE rebuild_child (
                 id INTEGER PRIMARY KEY,
                 parent_id INTEGER NOT NULL REFERENCES rebuild_parent(id) ON DELETE CASCADE
             );
             INSERT INTO rebuild_parent VALUES (1);
             INSERT INTO rebuild_child VALUES (1, 1);",
        )
        .expect("seed related predecessor tables");
    let migration = generated_style_rebuild_migration();

    db.migrate(std::slice::from_ref(&migration), Tracking::SQLITE)
        .expect("run generated-style rebuild");

    let children: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM rebuild_child", [], |row| row.get(0))
        .expect("count cascade children");
    assert_eq!(children, 1, "rebuild must not trigger ON DELETE CASCADE");
    let foreign_keys: i64 = db
        .conn()
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("read restored foreign-key mode");
    assert_eq!(foreign_keys, 1);
    let violations: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("verify rebuilt foreign keys");
    assert_eq!(violations, 0);
    let rebuilt_sql: String = db
        .conn()
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'rebuild_parent'",
            [],
            |row| row.get(0),
        )
        .expect("read rebuilt parent schema");
    assert!(rebuilt_sql.ends_with("STRICT"), "{rebuilt_sql}");
    let tracked: i64 = db
        .conn()
        .query_row(
            "SELECT count(*) FROM __drizzle_migrations WHERE name = ?1",
            [migration.tag()],
            |row| row.get(0),
        )
        .expect("read migration tracking");
    assert_eq!(tracked, 1);
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn libsql_rebuild_preserves_cascade_children_and_restores_foreign_keys() {
    let db = crate::common::helpers::libsql_setup::setup_empty().await;
    db.conn()
        .execute_batch(
            "CREATE TABLE rebuild_parent (id INTEGER PRIMARY KEY);
             CREATE TABLE rebuild_child (
                 id INTEGER PRIMARY KEY,
                 parent_id INTEGER NOT NULL REFERENCES rebuild_parent(id) ON DELETE CASCADE
             );
             INSERT INTO rebuild_parent VALUES (1);
             INSERT INTO rebuild_child VALUES (1, 1);",
        )
        .await
        .expect("seed related predecessor tables");

    db.migrate(&[generated_style_rebuild_migration()], Tracking::SQLITE)
        .await
        .expect("run generated-style rebuild");

    let mut rows = db
        .conn()
        .query("SELECT count(*) FROM rebuild_child", ())
        .await
        .expect("query rebuild state");
    let row = rows.next().await.expect("read row").expect("state row");
    assert_eq!(row.get::<i64>(0).expect("child count"), 1);

    let mut rows = db
        .conn()
        .query("PRAGMA foreign_keys", ())
        .await
        .expect("query foreign-key mode");
    let row = rows.next().await.expect("read row").expect("pragma row");
    assert_eq!(row.get::<i64>(0).expect("foreign-key mode"), 1);

    let mut rows = db
        .conn()
        .query("PRAGMA foreign_key_check", ())
        .await
        .expect("query foreign-key violations");
    assert!(rows.next().await.expect("read violation row").is_none());
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_rebuild_preserves_cascade_children_and_restores_foreign_keys() {
    let mut db = crate::common::helpers::turso_setup::setup_empty().await;
    for statement in [
        "CREATE TABLE rebuild_parent (id INTEGER PRIMARY KEY)",
        "CREATE TABLE rebuild_child (id INTEGER PRIMARY KEY, parent_id INTEGER NOT NULL REFERENCES rebuild_parent(id) ON DELETE CASCADE)",
        "INSERT INTO rebuild_parent VALUES (1)",
        "INSERT INTO rebuild_child VALUES (1, 1)",
    ] {
        db.conn()
            .execute(statement, ())
            .await
            .expect("seed related predecessor tables");
    }

    db.migrate(&[generated_style_rebuild_migration()], Tracking::SQLITE)
        .await
        .expect("run generated-style rebuild");

    let mut rows = db
        .conn()
        .query("SELECT count(*) FROM rebuild_child", ())
        .await
        .expect("query rebuild state");
    let row = rows.next().await.expect("read row").expect("state row");
    assert_eq!(row.get::<i64>(0).expect("child count"), 1);

    let mut rows = db
        .conn()
        .query("PRAGMA foreign_keys", ())
        .await
        .expect("query foreign-key mode");
    let row = rows.next().await.expect("read row").expect("pragma row");
    assert_eq!(row.get::<i64>(0).expect("foreign-key mode"), 1);

    let mut rows = db
        .conn()
        .query("PRAGMA foreign_key_check", ())
        .await
        .expect("query foreign-key violations");
    assert!(rows.next().await.expect("read violation row").is_none());
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_ignores_invalid_sentinels_in_applied_history() {
    let mut db = crate::common::helpers::turso_setup::setup_empty().await;
    let applied = Migration::new(
        "20260824000000_legacy_rebuild",
        "CREATE TABLE legacy_rebuild (id INTEGER PRIMARY KEY)",
    );
    db.migrate(std::slice::from_ref(&applied), Tracking::SQLITE)
        .await
        .expect("apply historical migration");

    let legacy_source = Migration::new(
        applied.tag(),
        "PRAGMA foreign_keys=OFF;\n--> statement-breakpoint\nSELECT 1;",
    );
    let outcome = db
        .migrate(&[legacy_source], Tracking::SQLITE)
        .await
        .expect("applied history must not be revalidated");

    assert!(outcome.is_up_to_date());
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_serializes_concurrent_runners() {
    use std::sync::{Arc, Barrier};

    let path = crate::common::helpers::temp_db_path();
    let barrier = Arc::new(Barrier::new(2));
    let migration = Migration::new(
        "20260712000000_concurrent",
        "CREATE TABLE IF NOT EXISTS migration_effects(value INTEGER NOT NULL);
         INSERT INTO migration_effects(value) VALUES (1);",
    );

    let handles = (0..2)
        .map(|_| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            let migration = migration.clone();
            std::thread::spawn(move || {
                let connection = rusqlite::Connection::open(path).expect("open concurrent DB");
                let (database, ()) = drizzle::sqlite::rusqlite::Drizzle::new(connection, ());
                barrier.wait();
                database.migrate(&[migration], Tracking::SQLITE)
            })
        })
        .collect::<Vec<_>>();

    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("migration thread").expect("migrate"))
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, drizzle_migrations::MigrateOutcome::Applied { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.is_up_to_date())
            .count(),
        1
    );

    let connection = rusqlite::Connection::open(&path).expect("reopen concurrent DB");
    let effect_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM migration_effects", [], |row| {
            row.get(0)
        })
        .expect("count migration effects");
    assert_eq!(effect_count, 1, "migration body must execute exactly once");
    drop(connection);
    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_runtime_migrate_runs_both_when_created_at_collides() {
    let db = crate::common::helpers::rusqlite_setup::setup_empty();

    let first = vec![Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
    )];
    db.migrate(&first, Tracking::SQLITE)
        .expect("first runtime migration");

    let second = vec![
        Migration::with_hash(
            "20230331141203_runtime_first",
            "runtime_hash_a",
            1_680_271_923_000,
            vec!["CREATE TABLE runtime_created_at_a (id INTEGER PRIMARY KEY)".to_string()],
        ),
        Migration::with_hash(
            "20230331141203_runtime_second",
            "runtime_hash_b",
            1_680_271_923_000,
            vec!["CREATE TABLE runtime_created_at_b (id INTEGER PRIMARY KEY)".to_string()],
        ),
    ];
    db.migrate(&second, Tracking::SQLITE)
        .expect("second runtime migration should apply the newly introduced name");

    let applied_rows: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM __drizzle_migrations", [], |row| {
            row.get(0)
        })
        .expect("count migrations rows");
    assert_eq!(
        applied_rows, 2,
        "migration identity is by name, so the second entry must be tracked \
         even though it shares created_at with the first"
    );

    let second_table_exists =
        crate::common::helpers::rusqlite_setup::table_exists(db.conn(), "runtime_created_at_b");
    assert_eq!(
        second_table_exists, 1,
        "second migration SQL should execute because its name has not been applied yet"
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
    assert!(
        applied_at.is_some(),
        "legacy rows get applied_at backfilled so they cannot read as interrupted"
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
    assert!(
        applied_at.is_some(),
        "legacy rows get applied_at backfilled so they cannot read as interrupted"
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
async fn turso_up_to_date_migration_check_is_read_only() {
    let path = crate::common::helpers::temp_db_path();
    let path_text = path
        .to_str()
        .expect("temporary sqlite path must be valid UTF-8");
    let database = turso::Builder::new_local(path_text)
        .experimental_mvcc_passive_checkpoint(true)
        .build()
        .await
        .expect("build Turso database");
    let migration_connection = database.connect().expect("migration connection");
    let mut writer_connection = database.connect().expect("writer connection");
    let (mut db, ()) = drizzle::sqlite::turso::Drizzle::new(migration_connection, ());
    let migration = Migration::new(
        "20260731000000_read_only_current_check",
        "CREATE TABLE records(value INTEGER NOT NULL)",
    );

    db.migrate(std::slice::from_ref(&migration), Tracking::SQLITE)
        .await
        .expect("apply migration");
    let writer = writer_connection
        .transaction_with_behavior(turso::transaction::TransactionBehavior::Immediate)
        .await
        .expect("begin concurrent writer");
    writer
        .execute("INSERT INTO records(value) VALUES (1)", ())
        .await
        .expect("hold an uncommitted write");

    let outcome = db
        .migrate(&[migration], Tracking::SQLITE)
        .await
        .expect("an up-to-date migration check must not request a write lock");
    assert!(outcome.is_up_to_date());
    writer.rollback().await.expect("rollback concurrent writer");
    writer_connection
        .execute("INSERT INTO records(value) VALUES (2)", ())
        .await
        .expect("the completed migration metadata cursor must release its read transaction");
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
    assert!(
        applied_at.is_some(),
        "legacy rows get applied_at backfilled so they cannot read as interrupted"
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

// ============================================================================
// Partial-migration recovery
// ============================================================================
//
// The incident these cover: a process killed after some of a migration's
// statements landed but before the migration was recorded. The next migrate()
// used to see the migration as pending, re-run it, and die on
// `table already exists`. Two-phase tracking turns that into a dirty row
// (`applied_at` NULL) that migrate() refuses to run past, and `--repair` /
// `migrate_with_repair` reconciles statement-by-statement.

#[cfg(feature = "rusqlite")]
const PARTIAL_TAG: &str = "20260801000000_partial";

#[cfg(feature = "rusqlite")]
const PARTIAL_SQL: &str = "CREATE TABLE partial_first (id INTEGER PRIMARY KEY);\n\
                           --> statement-breakpoint\n\
                           CREATE TABLE partial_second (id INTEGER PRIMARY KEY);";

/// Reproduce a crash mid-migration: create the tracking table, write the
/// two-phase dirty marker, apply only the first statement, then stop.
#[cfg(feature = "rusqlite")]
fn simulate_interrupted_migration(path: &std::path::Path) -> Migration {
    let migration = Migration::new(PARTIAL_TAG, PARTIAL_SQL);
    let set = drizzle_migrations::Migrations::with_tracking(
        vec![migration.clone()],
        drizzle_types::Dialect::SQLite,
        Tracking::SQLITE,
    );

    let connection = rusqlite::Connection::open(path).expect("open partial DB");
    connection
        .execute(&set.create_table_sql(), [])
        .expect("create tracking table");
    // Phase 1 of two-phase tracking: the migration is marked started...
    connection
        .execute(&set.record_migration_started_sql(&migration), [])
        .expect("record migration started");
    // ...only the first statement lands, then the process dies. Phase 3 never
    // runs, so the row keeps its NULL applied_at.
    connection
        .execute(&migration.statements()[0], [])
        .expect("apply first statement");
    drop(connection);

    migration
}

#[cfg(feature = "rusqlite")]
fn sqlite_table_exists(path: &std::path::Path, table: &str) -> bool {
    let connection = rusqlite::Connection::open(path).expect("open DB");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .expect("count table");
    count == 1
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_migrate_refuses_to_rerun_an_interrupted_migration() {
    let path = crate::common::helpers::temp_db_path();
    let migration = simulate_interrupted_migration(&path);

    let connection = rusqlite::Connection::open(&path).expect("open DB");
    let (database, ()) = drizzle::sqlite::rusqlite::Drizzle::new(connection, ());
    let error = database
        .migrate(&[migration], Tracking::SQLITE)
        .expect_err("a dirty tracking row must block migration");

    let text = error.to_string();
    assert!(text.contains(PARTIAL_TAG), "{text}");
    assert!(text.contains("interrupted mid-apply"), "{text}");
    assert!(text.contains("--repair"), "{text}");
    // The crucial regression: the old behavior re-ran statement 1 and died
    // with SQLite's raw "table partial_first already exists" instead of
    // naming the real problem. (The recovery text mentions that phrasing as an
    // example, so match on the table name.)
    assert!(
        !text.contains("partial_first already exists"),
        "must not surface as a raw DDL failure: {text}"
    );

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_repair_finishes_an_interrupted_migration() {
    let path = crate::common::helpers::temp_db_path();
    let migration = simulate_interrupted_migration(&path);
    assert!(sqlite_table_exists(&path, "partial_first"));
    assert!(!sqlite_table_exists(&path, "partial_second"));

    let connection = rusqlite::Connection::open(&path).expect("open DB");
    let (database, ()) = drizzle::sqlite::rusqlite::Drizzle::new(connection, ());
    let outcome = database
        .migrate_with_repair(std::slice::from_ref(&migration), Tracking::SQLITE)
        .expect("repair should reconcile the interrupted migration");

    assert_eq!(outcome.applied_tags(), [PARTIAL_TAG]);
    // Statement 1 was proven already present and skipped; statement 2 ran.
    assert!(sqlite_table_exists(&path, "partial_second"));

    // A subsequent plain migrate() is a clean no-op: the dirty row is gone and
    // the migration reads as applied.
    let outcome = database
        .migrate(&[migration], Tracking::SQLITE)
        .expect("second migrate");
    assert!(
        outcome.is_up_to_date(),
        "repaired migration must count as applied: {outcome:?}"
    );

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_repair_refuses_statements_it_cannot_prove() {
    let path = crate::common::helpers::temp_db_path();

    // An interrupted migration whose first statement is an ALTER: repair
    // cannot tell whether it ran, so it must refuse rather than guess.
    let migration = Migration::new(
        "20260801000001_unprovable",
        "ALTER TABLE partial_first ADD COLUMN note TEXT;\n\
         --> statement-breakpoint\n\
         CREATE TABLE partial_third (id INTEGER PRIMARY KEY);",
    );
    let set = drizzle_migrations::Migrations::with_tracking(
        vec![migration.clone()],
        drizzle_types::Dialect::SQLite,
        Tracking::SQLITE,
    );

    let connection = rusqlite::Connection::open(&path).expect("open DB");
    connection
        .execute("CREATE TABLE partial_first (id INTEGER PRIMARY KEY)", [])
        .expect("seed table");
    connection
        .execute(&set.create_table_sql(), [])
        .expect("create tracking table");
    connection
        .execute(&set.record_migration_started_sql(&migration), [])
        .expect("record migration started");
    drop(connection);

    let connection = rusqlite::Connection::open(&path).expect("reopen DB");
    let (database, ()) = drizzle::sqlite::rusqlite::Drizzle::new(connection, ());
    let error = database
        .migrate_with_repair(&[migration], Tracking::SQLITE)
        .expect_err("an unprovable statement must not be silently skipped or re-run");

    let text = error.to_string();
    assert!(text.contains("cannot repair"), "{text}");
    assert!(text.contains("statement 1"), "{text}");
    assert!(text.contains("UPDATE"), "manual completion SQL: {text}");
    assert!(
        !sqlite_table_exists(&path, "partial_third"),
        "a refused repair must not apply anything"
    );

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "rusqlite")]
#[test]
fn rusqlite_migrate_is_unaffected_when_nothing_is_dirty() {
    let path = crate::common::helpers::temp_db_path();
    let migration = Migration::new(PARTIAL_TAG, PARTIAL_SQL);

    let connection = rusqlite::Connection::open(&path).expect("open DB");
    let (database, ()) = drizzle::sqlite::rusqlite::Drizzle::new(connection, ());

    let outcome = database
        .migrate(std::slice::from_ref(&migration), Tracking::SQLITE)
        .expect("first migrate");
    assert_eq!(outcome.applied_tags(), [PARTIAL_TAG]);
    assert!(sqlite_table_exists(&path, "partial_first"));
    assert!(sqlite_table_exists(&path, "partial_second"));

    let outcome = database
        .migrate(&[migration], Tracking::SQLITE)
        .expect("second migrate");
    assert!(outcome.is_up_to_date());

    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_migrate_records_two_phase_tracking() {
    // turso's crash recovery around in-transaction DDL is not trusted, so its
    // migrate() writes the tracking row *before* running statements and stamps
    // `applied_at` after. A completed run must still leave a clean row.
    let (db, _schema) =
        crate::common::helpers::turso_setup::setup_empty_db(PushSchema::default()).await;
    let mut db = db;

    let migration = Migration::new(
        "20260801000000_two_phase",
        "CREATE TABLE two_phase_a (id INTEGER PRIMARY KEY);\n\
         --> statement-breakpoint\n\
         CREATE TABLE two_phase_b (id INTEGER PRIMARY KEY);",
    );

    let outcome = db
        .migrate(std::slice::from_ref(&migration), Tracking::SQLITE)
        .await
        .expect("migrate");
    assert_eq!(outcome.applied_tags(), ["20260801000000_two_phase"]);

    // Phase 3 ran: no dirty rows are left behind.
    let mut rows = db
        .conn()
        .query(
            "SELECT COUNT(*) FROM __drizzle_migrations WHERE applied_at IS NULL",
            (),
        )
        .await
        .expect("query dirty rows");
    let dirty = rows
        .next()
        .await
        .expect("next row")
        .expect("row")
        .get::<i64>(0)
        .expect("count");
    assert_eq!(dirty, 0, "a completed migration must not stay dirty");

    let outcome = db
        .migrate(&[migration], Tracking::SQLITE)
        .await
        .expect("second migrate");
    assert!(outcome.is_up_to_date());
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_repair_finishes_an_interrupted_migration() {
    let (db, _schema) =
        crate::common::helpers::turso_setup::setup_empty_db(PushSchema::default()).await;
    let mut db = db;

    let migration = Migration::new(
        "20260801000002_turso_partial",
        "CREATE TABLE turso_partial_first (id INTEGER PRIMARY KEY);\n\
         --> statement-breakpoint\n\
         CREATE TABLE turso_partial_second (id INTEGER PRIMARY KEY);",
    );
    let set = drizzle_migrations::Migrations::with_tracking(
        vec![migration.clone()],
        drizzle_types::Dialect::SQLite,
        Tracking::SQLITE,
    );

    // Reproduce the incident by hand: tracking table, dirty marker, first
    // statement applied, then "crash".
    db.conn()
        .execute(&set.create_table_sql(), ())
        .await
        .expect("create tracking table");
    db.conn()
        .execute(&set.record_migration_started_sql(&migration), ())
        .await
        .expect("record started");
    db.conn()
        .execute(&migration.statements()[0], ())
        .await
        .expect("apply first statement");

    let error = db
        .migrate(std::slice::from_ref(&migration), Tracking::SQLITE)
        .await
        .expect_err("a dirty row must block migration");
    let text = error.to_string();
    assert!(text.contains("20260801000002_turso_partial"), "{text}");
    assert!(text.contains("interrupted mid-apply"), "{text}");

    let outcome = db
        .migrate_with_repair(std::slice::from_ref(&migration), Tracking::SQLITE)
        .await
        .expect("repair");
    assert_eq!(outcome.applied_tags(), ["20260801000002_turso_partial"]);

    let mut rows = db
        .conn()
        .query(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='turso_partial_second'",
            (),
        )
        .await
        .expect("query table");
    let exists = rows
        .next()
        .await
        .expect("next row")
        .expect("row")
        .get::<i64>(0)
        .expect("count");
    assert_eq!(exists, 1, "repair must run the statement that never landed");

    let outcome = db
        .migrate(&[migration], Tracking::SQLITE)
        .await
        .expect("second migrate");
    assert!(outcome.is_up_to_date());
}
