#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use drizzle::postgres::prelude::*;
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use drizzle_migrations::{Migration, Tracking};

#[cfg(feature = "postgres-sync")]
#[PostgresTable(name = "items", schema = "push_creates_test")]
struct PushCreates {
    #[column(serial, primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "postgres-sync")]
#[derive(PostgresSchema)]
struct PushCreatesSchema {
    items: PushCreates,
}

#[cfg(feature = "postgres-sync")]
#[PostgresTable(name = "items", schema = "push_idempotent_test")]
struct PushIdempotent {
    #[column(serial, primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "postgres-sync")]
#[derive(PostgresSchema)]
struct PushIdempotentSchema {
    items: PushIdempotent,
}

#[cfg(feature = "postgres-sync")]
#[PostgresTable(name = "items", schema = "push_usable_test")]
struct PushUsable {
    #[column(serial, primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "postgres-sync")]
#[derive(PostgresSchema)]
struct PushUsableSchema {
    items: PushUsable,
}

#[cfg(feature = "tokio-postgres")]
#[PostgresTable(name = "items", schema = "push_tokio_creates_test")]
struct TokioPushCreates {
    #[column(serial, primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "tokio-postgres")]
#[derive(PostgresSchema)]
struct TokioPushCreatesSchema {
    items: TokioPushCreates,
}

#[cfg(feature = "tokio-postgres")]
#[PostgresTable(name = "items", schema = "push_tokio_idempotent_test")]
struct TokioPushIdempotent {
    #[column(serial, primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "tokio-postgres")]
#[derive(PostgresSchema)]
struct TokioPushIdempotentSchema {
    items: TokioPushIdempotent,
}

#[cfg(feature = "tokio-postgres")]
#[PostgresTable(name = "items", schema = "push_tokio_usable_test")]
struct TokioPushUsable {
    #[column(serial, primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "tokio-postgres")]
#[derive(PostgresSchema)]
struct TokioPushUsableSchema {
    items: TokioPushUsable,
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_creates_table() {
    let (mut db, schema) = crate::common::helpers::postgres_sync_setup::setup_empty_named_db(
        "push_creates_test",
        PushCreatesSchema::default(),
    );
    let schema_name = db.schema_name().to_string();

    db.push(&schema).expect("push schema");

    let count = crate::common::helpers::postgres_sync_setup::table_exists(
        db.conn_mut(),
        &schema_name,
        "items",
    );
    assert_eq!(count, 1, "push should create the table");
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_is_idempotent() {
    let (mut db, schema) = crate::common::helpers::postgres_sync_setup::setup_empty_named_db(
        "push_idempotent_test",
        PushIdempotentSchema::default(),
    );

    db.push(&schema).expect("first push");
    db.push(&schema).expect("second push should be a no-op");
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_table_is_usable() {
    let (mut db, schema) = crate::common::helpers::postgres_sync_setup::setup_empty_named_db(
        "push_usable_test",
        PushUsableSchema::default(),
    );
    let schema_name = db.schema_name().to_string();

    db.push(&schema).expect("push schema");

    let id: i32 = db
        .conn_mut()
        .query_one(
            &format!(
                "INSERT INTO \"{}\".items (label) VALUES ('hello') RETURNING id",
                schema_name
            ),
            &[],
        )
        .expect("insert into pushed table")
        .get(0);

    let label: String = db
        .conn_mut()
        .query_one(
            &format!("SELECT label FROM \"{}\".items WHERE id = $1", schema_name),
            &[&id],
        )
        .expect("select from pushed table")
        .get(0);
    assert_eq!(label, "hello");
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_runtime_migrate_upgrades_legacy_tracking_table() {
    let mut db =
        crate::common::helpers::postgres_sync_setup::setup_empty_named("runtime_upgrade_sync_test");
    let schema_name = db.schema_name().to_string();

    crate::common::helpers::postgres_sync_setup::create_legacy_tracking_table(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    db.conn_mut()
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_a", &1_680_271_923_000_i64],
        )
        .expect("insert legacy migration row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
            schema_name
        )],
    );

    db.migrate(&[migration], Tracking::POSTGRES.schema(schema_name.clone()))
        .expect("upgrade legacy runtime metadata");

    let columns = crate::common::helpers::postgres_sync_setup::legacy_tracking_columns(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    assert_eq!(
        columns,
        vec!["id", "hash", "created_at", "name", "applied_at"],
        "tracking table should be upgraded in place"
    );

    let row = db
        .conn_mut()
        .query_one(
            &format!(
                "SELECT name, applied_at::text FROM \"{}\".\"__drizzle_migrations\" LIMIT 1",
                schema_name
            ),
            &[],
        )
        .expect("select upgraded migration row");
    let name: String = row.get(0);
    let applied_at: Option<String> = row.get(1);
    assert_eq!(name, "20230331141203_runtime_first");
    assert!(
        applied_at.is_some(),
        "legacy rows get applied_at backfilled so they cannot read as interrupted"
    );

    let migrated_table_exists = crate::common::helpers::postgres_sync_setup::table_exists(
        db.conn_mut(),
        &schema_name,
        "runtime_created_at_a",
    );
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let mut db = crate::common::helpers::postgres_sync_setup::setup_empty_named(
        "runtime_upgrade_collision_sync_test",
    );
    let schema_name = db.schema_name().to_string();

    crate::common::helpers::postgres_sync_setup::create_legacy_tracking_table(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    db.conn_mut()
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_b", &1_680_271_923_000_i64],
        )
        .expect("insert legacy migration row");

    let migrations = vec![
        Migration::with_hash(
            "20230331141203_runtime_alpha",
            "runtime_hash_a",
            1_680_271_923_000,
            vec![format!(
                "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
                schema_name
            )],
        ),
        Migration::with_hash(
            "20230331141203_runtime_beta",
            "runtime_hash_b",
            1_680_271_923_000,
            vec![format!(
                "CREATE TABLE \"{}\".runtime_created_at_b (id INTEGER PRIMARY KEY)",
                schema_name
            )],
        ),
    ];

    db.migrate(&migrations, Tracking::POSTGRES.schema(schema_name.clone()))
        .expect("upgrade legacy runtime metadata with timestamp collision");

    let name: String = db
        .conn_mut()
        .query_one(
            &format!(
                "SELECT name FROM \"{}\".\"__drizzle_migrations\" LIMIT 1",
                schema_name
            ),
            &[],
        )
        .expect("select upgraded migration name")
        .get(0);
    assert_eq!(name, "20230331141203_runtime_beta");
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_runtime_migrate_upgrade_rejects_unmatched_legacy_rows() {
    let mut db = crate::common::helpers::postgres_sync_setup::setup_empty_named(
        "runtime_upgrade_unmatched_sync_test",
    );
    let schema_name = db.schema_name().to_string();

    crate::common::helpers::postgres_sync_setup::create_legacy_tracking_table(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    db.conn_mut()
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"unknown_hash", &1_680_271_924_000_i64],
        )
        .expect("insert unmatched legacy row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
            schema_name
        )],
    );

    let err = db
        .migrate(&[migration], Tracking::POSTGRES.schema(schema_name.clone()))
        .expect_err("unmatched legacy metadata should fail");
    assert!(err.to_string().contains("do not match local migrations"));

    let columns = crate::common::helpers::postgres_sync_setup::legacy_tracking_columns(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    assert_eq!(columns, vec!["id", "hash", "created_at"]);
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_runtime_migrate_upgrades_legacy_tracking_table() {
    let mut db = crate::common::helpers::tokio_postgres_setup::setup_empty_named(
        "runtime_upgrade_tokio_test",
    )
    .await;
    let schema_name = db.schema_name().to_string();

    crate::common::helpers::tokio_postgres_setup::create_legacy_tracking_table(
        db.conn(),
        &schema_name,
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_a", &1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
            schema_name
        )],
    );

    db.migrate(&[migration], Tracking::POSTGRES.schema(schema_name.clone()))
        .await
        .expect("upgrade legacy runtime metadata");

    let columns = crate::common::helpers::tokio_postgres_setup::legacy_tracking_columns(
        db.conn(),
        &schema_name,
        "__drizzle_migrations",
    )
    .await;
    assert_eq!(
        columns,
        vec!["id", "hash", "created_at", "name", "applied_at"],
        "tracking table should be upgraded in place"
    );

    let row = db
        .conn()
        .query_one(
            &format!(
                "SELECT name, applied_at::text FROM \"{}\".\"__drizzle_migrations\" LIMIT 1",
                schema_name
            ),
            &[],
        )
        .await
        .expect("select upgraded migration row");
    let name: String = row.get(0);
    let applied_at: Option<String> = row.get(1);
    assert_eq!(name, "20230331141203_runtime_first");
    assert!(
        applied_at.is_some(),
        "legacy rows get applied_at backfilled so they cannot read as interrupted"
    );

    let migrated_table_exists = crate::common::helpers::tokio_postgres_setup::table_exists(
        db.conn(),
        &schema_name,
        "runtime_created_at_a",
    )
    .await;
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let mut db = crate::common::helpers::tokio_postgres_setup::setup_empty_named(
        "runtime_upgrade_collision_tokio_test",
    )
    .await;
    let schema_name = db.schema_name().to_string();

    crate::common::helpers::tokio_postgres_setup::create_legacy_tracking_table(
        db.conn(),
        &schema_name,
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_b", &1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

    let migrations = vec![
        Migration::with_hash(
            "20230331141203_runtime_alpha",
            "runtime_hash_a",
            1_680_271_923_000,
            vec![format!(
                "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
                schema_name
            )],
        ),
        Migration::with_hash(
            "20230331141203_runtime_beta",
            "runtime_hash_b",
            1_680_271_923_000,
            vec![format!(
                "CREATE TABLE \"{}\".runtime_created_at_b (id INTEGER PRIMARY KEY)",
                schema_name
            )],
        ),
    ];

    db.migrate(&migrations, Tracking::POSTGRES.schema(schema_name.clone()))
        .await
        .expect("upgrade legacy runtime metadata with timestamp collision");

    let name: String = db
        .conn()
        .query_one(
            &format!(
                "SELECT name FROM \"{}\".\"__drizzle_migrations\" LIMIT 1",
                schema_name
            ),
            &[],
        )
        .await
        .expect("select upgraded migration name")
        .get(0);
    assert_eq!(name, "20230331141203_runtime_beta");
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_runtime_migrate_upgrade_rejects_unmatched_legacy_rows() {
    let mut db = crate::common::helpers::tokio_postgres_setup::setup_empty_named(
        "runtime_upgrade_unmatched_tokio_test",
    )
    .await;
    let schema_name = db.schema_name().to_string();

    crate::common::helpers::tokio_postgres_setup::create_legacy_tracking_table(
        db.conn(),
        &schema_name,
        "__drizzle_migrations",
    )
    .await;
    db.conn()
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"unknown_hash", &1_680_271_924_000_i64],
        )
        .await
        .expect("insert unmatched legacy row");

    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
            schema_name
        )],
    );

    let err = db
        .migrate(&[migration], Tracking::POSTGRES.schema(schema_name.clone()))
        .await
        .expect_err("unmatched legacy metadata should fail");
    assert!(err.to_string().contains("do not match local migrations"));

    let columns = crate::common::helpers::tokio_postgres_setup::legacy_tracking_columns(
        db.conn(),
        &schema_name,
        "__drizzle_migrations",
    )
    .await;
    assert_eq!(columns, vec!["id", "hash", "created_at"]);
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_push_creates_table() {
    let (db, schema) = crate::common::helpers::tokio_postgres_setup::setup_empty_named_db(
        "push_tokio_creates_test",
        TokioPushCreatesSchema::default(),
    )
    .await;

    db.push(&schema).await.expect("push schema");

    let count = crate::common::helpers::tokio_postgres_setup::table_exists(
        db.conn(),
        db.schema_name(),
        "items",
    )
    .await;
    assert_eq!(count, 1, "push should create the table");
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_push_is_idempotent() {
    let (db, schema) = crate::common::helpers::tokio_postgres_setup::setup_empty_named_db(
        "push_tokio_idempotent_test",
        TokioPushIdempotentSchema::default(),
    )
    .await;

    db.push(&schema).await.expect("first push");
    db.push(&schema)
        .await
        .expect("second push should be a no-op");
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_push_table_is_usable() {
    let (db, schema) = crate::common::helpers::tokio_postgres_setup::setup_empty_named_db(
        "push_tokio_usable_test",
        TokioPushUsableSchema::default(),
    )
    .await;

    db.push(&schema).await.expect("push schema");

    let id: i32 = db
        .conn()
        .query_one(
            &format!(
                "INSERT INTO \"{}\".items (label) VALUES ('hello') RETURNING id",
                db.schema_name()
            ),
            &[],
        )
        .await
        .expect("insert into pushed table")
        .get(0);

    let label: String = db
        .conn()
        .query_one(
            &format!(
                "SELECT label FROM \"{}\".items WHERE id = $1",
                db.schema_name()
            ),
            &[&id],
        )
        .await
        .expect("select from pushed table")
        .get(0);
    assert_eq!(label, "hello");
}

// ============================================================================
// Partial-migration recovery (CONCURRENTLY path)
// ============================================================================
//
// A migration containing CREATE INDEX CONCURRENTLY cannot run in a
// transaction, so its statements execute autocommit. Two-phase tracking marks
// the migration dirty before the first statement and stamps `applied_at` only
// after the last one, so a crash in between leaves a recoverable dirty row
// instead of an untracked partial schema.

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_concurrent_migration_uses_two_phase_tracking() {
    let mut db =
        crate::common::helpers::postgres_sync_setup::setup_empty_named("two_phase_sync_test");
    let schema_name = db.schema_name().to_string();
    let tracking = Tracking::POSTGRES.schema(schema_name.clone());

    let migration = Migration::new(
        "20260801000000_concurrent",
        &format!(
            "CREATE TABLE \"{schema_name}\".two_phase_items (id INTEGER PRIMARY KEY, label TEXT);\n\
             --> statement-breakpoint\n\
             CREATE INDEX CONCURRENTLY two_phase_items_label_idx \
             ON \"{schema_name}\".two_phase_items (label);"
        ),
    );

    let outcome = db
        .migrate(std::slice::from_ref(&migration), tracking.clone())
        .expect("concurrent migration");
    assert_eq!(outcome.applied_tags(), ["20260801000000_concurrent"]);

    // Phase 3 ran: the row is complete, not dirty.
    let dirty: i64 = db
        .conn_mut()
        .query_one(
            &format!(
                "SELECT COUNT(*) FROM \"{schema_name}\".\"__drizzle_migrations\" \
                 WHERE applied_at IS NULL"
            ),
            &[],
        )
        .expect("count dirty rows")
        .get(0);
    assert_eq!(dirty, 0, "a completed migration must not stay dirty");

    let outcome = db.migrate(&[migration], tracking).expect("second migrate");
    assert!(outcome.is_up_to_date());
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_repair_finishes_an_interrupted_concurrent_migration() {
    let mut db = crate::common::helpers::postgres_sync_setup::setup_empty_named("repair_sync_test");
    let schema_name = db.schema_name().to_string();
    let tracking = Tracking::POSTGRES.schema(schema_name.clone());

    let migration = Migration::new(
        "20260801000001_interrupted",
        &format!(
            "CREATE TABLE \"{schema_name}\".repair_items (id INTEGER PRIMARY KEY, label TEXT);\n\
             --> statement-breakpoint\n\
             CREATE INDEX CONCURRENTLY repair_items_label_idx \
             ON \"{schema_name}\".repair_items (label);"
        ),
    );
    let set = drizzle_migrations::Migrations::with_tracking(
        vec![migration.clone()],
        drizzle_types::Dialect::PostgreSQL,
        tracking.clone(),
    );

    // Reproduce the incident: schema + tracking table, phase-1 dirty marker,
    // first statement applied, then "crash" before phase 3.
    if let Some(schema_sql) = set.create_schema_sql() {
        db.conn_mut()
            .execute(schema_sql.as_str(), &[])
            .expect("create tracking schema");
    }
    db.conn_mut()
        .execute(set.create_table_sql().as_str(), &[])
        .expect("create tracking table");
    db.conn_mut()
        .execute(set.record_migration_started_sql(&migration).as_str(), &[])
        .expect("record migration started");
    db.conn_mut()
        .execute(migration.statements()[0].as_str(), &[])
        .expect("apply first statement");

    // Plain migrate() refuses and names the interrupted migration.
    let error = db
        .migrate(std::slice::from_ref(&migration), tracking.clone())
        .expect_err("a dirty tracking row must block migration");
    let text = error.to_string();
    assert!(text.contains("20260801000001_interrupted"), "{text}");
    assert!(text.contains("interrupted mid-apply"), "{text}");
    assert!(text.contains("--repair"), "{text}");

    // Repair proves statement 1 already landed, runs statement 2, clears the
    // marker.
    let outcome = db
        .migrate_with_repair(std::slice::from_ref(&migration), tracking.clone())
        .expect("repair should reconcile the interrupted migration");
    assert_eq!(outcome.applied_tags(), ["20260801000001_interrupted"]);

    let index_exists: i64 = db
        .conn_mut()
        .query_one(
            "SELECT COUNT(*) FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
            &[&schema_name, &"repair_items_label_idx"],
        )
        .expect("count index")
        .get(0);
    assert_eq!(
        index_exists, 1,
        "repair must run the statement that never landed"
    );

    let outcome = db.migrate(&[migration], tracking).expect("second migrate");
    assert!(
        outcome.is_up_to_date(),
        "repaired migration must count as applied: {outcome:?}"
    );
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_repair_refuses_statements_it_cannot_prove() {
    let mut db =
        crate::common::helpers::postgres_sync_setup::setup_empty_named("repair_refuse_sync_test");
    let schema_name = db.schema_name().to_string();
    let tracking = Tracking::POSTGRES.schema(schema_name.clone());

    db.conn_mut()
        .execute(
            &format!("CREATE TABLE \"{schema_name}\".refuse_items (id INTEGER PRIMARY KEY)"),
            &[],
        )
        .expect("seed table");

    // The interrupted migration leads with an ALTER: repair cannot tell
    // whether it ran, so it must refuse instead of guessing.
    let migration = Migration::new(
        "20260801000002_unprovable",
        &format!(
            "ALTER TABLE \"{schema_name}\".refuse_items ADD COLUMN note TEXT;\n\
             --> statement-breakpoint\n\
             CREATE INDEX CONCURRENTLY refuse_items_note_idx \
             ON \"{schema_name}\".refuse_items (note);"
        ),
    );
    let set = drizzle_migrations::Migrations::with_tracking(
        vec![migration.clone()],
        drizzle_types::Dialect::PostgreSQL,
        tracking.clone(),
    );

    if let Some(schema_sql) = set.create_schema_sql() {
        db.conn_mut()
            .execute(schema_sql.as_str(), &[])
            .expect("create tracking schema");
    }
    db.conn_mut()
        .execute(set.create_table_sql().as_str(), &[])
        .expect("create tracking table");
    db.conn_mut()
        .execute(set.record_migration_started_sql(&migration).as_str(), &[])
        .expect("record migration started");

    let error = db
        .migrate_with_repair(&[migration], tracking)
        .expect_err("an unprovable statement must not be silently skipped or re-run");
    let text = error.to_string();
    assert!(text.contains("cannot repair"), "{text}");
    assert!(text.contains("statement 1"), "{text}");
    assert!(text.contains("UPDATE"), "manual completion SQL: {text}");

    let index_exists: i64 = db
        .conn_mut()
        .query_one(
            "SELECT COUNT(*) FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
            &[&schema_name, &"refuse_items_note_idx"],
        )
        .expect("count index")
        .get(0);
    assert_eq!(index_exists, 0, "a refused repair must not apply anything");
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_repair_finishes_an_interrupted_concurrent_migration() {
    let mut db =
        crate::common::helpers::tokio_postgres_setup::setup_empty_named("repair_tokio_test").await;
    let schema_name = db.schema_name().to_string();
    let tracking = Tracking::POSTGRES.schema(schema_name.clone());

    let migration = Migration::new(
        "20260801000003_interrupted",
        &format!(
            "CREATE TABLE \"{schema_name}\".repair_async_items (id INTEGER PRIMARY KEY, label TEXT);\n\
             --> statement-breakpoint\n\
             CREATE INDEX CONCURRENTLY repair_async_items_label_idx \
             ON \"{schema_name}\".repair_async_items (label);"
        ),
    );
    let set = drizzle_migrations::Migrations::with_tracking(
        vec![migration.clone()],
        drizzle_types::Dialect::PostgreSQL,
        tracking.clone(),
    );

    if let Some(schema_sql) = set.create_schema_sql() {
        db.conn()
            .execute(schema_sql.as_str(), &[])
            .await
            .expect("create tracking schema");
    }
    db.conn()
        .execute(set.create_table_sql().as_str(), &[])
        .await
        .expect("create tracking table");
    db.conn()
        .execute(set.record_migration_started_sql(&migration).as_str(), &[])
        .await
        .expect("record migration started");
    db.conn()
        .execute(migration.statements()[0].as_str(), &[])
        .await
        .expect("apply first statement");

    let error = db
        .migrate(std::slice::from_ref(&migration), tracking.clone())
        .await
        .expect_err("a dirty tracking row must block migration");
    let text = error.to_string();
    assert!(text.contains("20260801000003_interrupted"), "{text}");
    assert!(text.contains("interrupted mid-apply"), "{text}");

    let outcome = db
        .migrate_with_repair(std::slice::from_ref(&migration), tracking.clone())
        .await
        .expect("repair should reconcile the interrupted migration");
    assert_eq!(outcome.applied_tags(), ["20260801000003_interrupted"]);

    let index_exists: i64 = db
        .conn()
        .query_one(
            "SELECT COUNT(*) FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
            &[&schema_name, &"repair_async_items_label_idx"],
        )
        .await
        .expect("count index")
        .get(0);
    assert_eq!(index_exists, 1);

    let outcome = db
        .migrate(&[migration], tracking)
        .await
        .expect("second migrate");
    assert!(outcome.is_up_to_date());
}

// =============================================================================
// Runner interop and robustness
// =============================================================================

/// drizzle-orm's v0 -> v1 tracking upgrade (`up-migrations/pg.ts`) adds
/// `name`/`applied_at` and writes `applied_at = NULL` on every existing row.
/// Those rows are applied: drizzle-orm only looks at `name`.
#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_migrate_accepts_rows_upgraded_by_drizzle_orm() {
    let mut db = crate::common::helpers::postgres_sync_setup::setup_empty_named(
        "upstream_upgraded_sync_test",
    );
    let schema_name = db.schema_name().to_string();
    let first = Migration::new(
        "20240101000000_init",
        &format!("CREATE TABLE \"{schema_name}\".upstream_a (id INTEGER);"),
    );
    let second = Migration::new(
        "20240102000000_next",
        &format!("CREATE TABLE \"{schema_name}\".upstream_b (id INTEGER);"),
    );
    db.conn_mut()
        .batch_execute(&format!(
            "CREATE TABLE \"{schema_name}\".upstream_a (id INTEGER);
             CREATE TABLE \"{schema_name}\".\"__drizzle_migrations\" (
                 id SERIAL PRIMARY KEY, hash text NOT NULL, created_at bigint);
             INSERT INTO \"{schema_name}\".\"__drizzle_migrations\" (hash, created_at)
                 VALUES ('{}', 1704067200000);
             ALTER TABLE \"{schema_name}\".\"__drizzle_migrations\"
                 ADD COLUMN IF NOT EXISTS name text;
             ALTER TABLE \"{schema_name}\".\"__drizzle_migrations\"
                 ADD COLUMN IF NOT EXISTS applied_at timestamp with time zone DEFAULT now();
             UPDATE \"{schema_name}\".\"__drizzle_migrations\"
                 SET name = '20240101000000_init', applied_at = NULL WHERE id = 1;",
            first.hash()
        ))
        .expect("reproduce drizzle-orm's upgraded tracking table");

    let outcome = db
        .migrate(
            &[first.clone(), second.clone()],
            Tracking::POSTGRES.schema(schema_name.clone()),
        )
        .expect("rows upgraded by drizzle-orm must count as applied");
    assert_eq!(outcome.applied_tags(), ["20240102000000_next"]);

    let outcome = db
        .migrate(&[first, second], Tracking::POSTGRES.schema(schema_name))
        .expect("second migrate");
    assert!(outcome.is_up_to_date());
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_migrate_accepts_rows_upgraded_by_drizzle_orm() {
    let mut db = crate::common::helpers::tokio_postgres_setup::setup_empty_named(
        "upstream_upgraded_tokio_test",
    )
    .await;
    let schema_name = db.schema_name().to_string();
    let first = Migration::new(
        "20240101000000_init",
        &format!("CREATE TABLE \"{schema_name}\".upstream_a (id INTEGER);"),
    );
    let second = Migration::new(
        "20240102000000_next",
        &format!("CREATE TABLE \"{schema_name}\".upstream_b (id INTEGER);"),
    );
    db.conn()
        .batch_execute(&format!(
            "CREATE TABLE \"{schema_name}\".upstream_a (id INTEGER);
             CREATE TABLE \"{schema_name}\".\"__drizzle_migrations\" (
                 id SERIAL PRIMARY KEY, hash text NOT NULL, created_at bigint,
                 name text, applied_at timestamp with time zone DEFAULT now());
             INSERT INTO \"{schema_name}\".\"__drizzle_migrations\" (hash, created_at, name, applied_at)
                 VALUES ('{}', 1704067200000, '20240101000000_init', NULL);",
            first.hash()
        ))
        .await
        .expect("reproduce drizzle-orm's upgraded tracking table");

    let outcome = db
        .migrate(&[first, second], Tracking::POSTGRES.schema(schema_name))
        .await
        .expect("rows upgraded by drizzle-orm must count as applied");
    assert_eq!(outcome.applied_tags(), ["20240102000000_next"]);
}

#[cfg(feature = "postgres-sync")]
fn pg_test_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
    })
}

/// A pre-transactional legacy upgrade could die after adding `name` but
/// before adding `applied_at`; the next run must finish the upgrade.
#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_migrate_finishes_a_half_upgraded_tracking_table() {
    let mut db =
        crate::common::helpers::postgres_sync_setup::setup_empty_named("half_upgraded_sync_test");
    let schema_name = db.schema_name().to_string();
    crate::common::helpers::postgres_sync_setup::create_legacy_tracking_table(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    db.conn_mut()
        .batch_execute(&format!(
            "INSERT INTO \"{schema_name}\".\"__drizzle_migrations\" (hash, created_at)
                 VALUES ('half_hash', 1680271923456);
             ALTER TABLE \"{schema_name}\".\"__drizzle_migrations\" ADD COLUMN \"name\" TEXT;"
        ))
        .expect("reproduce the half-upgraded table");

    let migration = Migration::with_hash(
        "20230331141203_half",
        "half_hash",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{schema_name}\".half_upgraded (id INTEGER)"
        )],
    );
    let outcome = db
        .migrate(
            std::slice::from_ref(&migration),
            Tracking::POSTGRES.schema(schema_name.clone()),
        )
        .expect("a half-upgraded tracking table is completed");
    assert!(outcome.is_up_to_date(), "{outcome:?}");

    let columns = crate::common::helpers::postgres_sync_setup::legacy_tracking_columns(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    assert_eq!(columns, ["id", "hash", "created_at", "name", "applied_at"]);
    let row = db
        .conn_mut()
        .query_one(
            &format!(
                "SELECT name, applied_at IS NOT NULL FROM \"{schema_name}\".\"__drizzle_migrations\""
            ),
            &[],
        )
        .expect("upgraded row");
    assert_eq!(row.get::<_, String>(0), "20230331141203_half");
    assert!(row.get::<_, bool>(1));
}

/// The legacy upgrade's statements commit together: when one fails, none of
/// them stick.
#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_legacy_tracking_upgrade_is_atomic() {
    let mut db =
        crate::common::helpers::postgres_sync_setup::setup_empty_named("atomic_upgrade_sync_test");
    let schema_name = db.schema_name().to_string();
    crate::common::helpers::postgres_sync_setup::create_legacy_tracking_table(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    // A trigger that rejects the backfill UPDATE makes the batch fail after
    // both ALTERs ran.
    db.conn_mut()
        .batch_execute(&format!(
            "INSERT INTO \"{schema_name}\".\"__drizzle_migrations\" (hash, created_at)
                 VALUES ('atomic_hash', 1680271923000);
             CREATE FUNCTION \"{schema_name}\".reject_update() RETURNS trigger
                 LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'backfill rejected'; END $$;
             CREATE TRIGGER reject_update BEFORE UPDATE ON \"{schema_name}\".\"__drizzle_migrations\"
                 FOR EACH ROW EXECUTE FUNCTION \"{schema_name}\".reject_update();"
        ))
        .expect("arm the failing backfill");

    let migration = Migration::with_hash(
        "20230331141203_atomic",
        "atomic_hash",
        1_680_271_923_000,
        vec![],
    );
    let error = db
        .migrate(
            std::slice::from_ref(&migration),
            Tracking::POSTGRES.schema(schema_name.clone()),
        )
        .expect_err("the rejected backfill fails the upgrade");
    assert!(error.to_string().contains("backfill rejected"), "{error}");

    let columns = crate::common::helpers::postgres_sync_setup::legacy_tracking_columns(
        db.conn_mut(),
        &schema_name,
        "__drizzle_migrations",
    );
    assert_eq!(
        columns,
        ["id", "hash", "created_at"],
        "a failed upgrade must not leave half the columns behind"
    );
}

/// Concurrent first runs used to race on `CREATE SCHEMA IF NOT EXISTS`
/// before taking the advisory lock.
#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_concurrent_first_migrate_creates_tracking_schema_once() {
    let db = crate::common::helpers::postgres_sync_setup::setup_empty_named(
        "concurrent_first_sync_test",
    );
    let schema_name = db.schema_name().to_string();
    let tracking_schema = format!("{schema_name}_tracking");
    let url = pg_test_url();
    let mut admin = postgres::Client::connect(&url, postgres::NoTls).expect("admin connection");

    for round in 0..5 {
        admin
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS \"{tracking_schema}\" CASCADE"
            ))
            .expect("reset tracking schema");
        let migration = Migration::new(
            "20240101000000_concurrent_first",
            &format!("CREATE TABLE \"{schema_name}\".concurrent_first_{round} (id INTEGER);"),
        );
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(6));
        let handles = (0..6)
            .map(|_| {
                let url = url.clone();
                let barrier = barrier.clone();
                let migration = migration.clone();
                let tracking_schema = tracking_schema.clone();
                std::thread::spawn(move || {
                    let client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
                    let (mut db, ()) = drizzle::postgres::sync::Drizzle::new(client);
                    barrier.wait();
                    db.migrate(&[migration], Tracking::POSTGRES.schema(tracking_schema))
                        .map(|outcome| outcome.applied_count())
                        .map_err(|error| error.to_string())
                })
            })
            .collect::<Vec<_>>();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("migrate thread"))
            .collect::<Vec<_>>();
        let applied: usize = results
            .iter()
            .map(|result| *result.as_ref().unwrap_or_else(|error| panic!("{error}")))
            .sum();
        assert_eq!(applied, 1, "exactly one runner applies the migration");
    }

    admin
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS \"{tracking_schema}\" CASCADE"
        ))
        .expect("drop tracking schema");
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_migrate_finishes_a_half_upgraded_tracking_table() {
    let mut db =
        crate::common::helpers::tokio_postgres_setup::setup_empty_named("half_upgraded_tokio_test")
            .await;
    let schema_name = db.schema_name().to_string();
    db.conn()
        .batch_execute(&format!(
            "CREATE TABLE \"{schema_name}\".\"__drizzle_migrations\"
                 (id SERIAL PRIMARY KEY, hash TEXT NOT NULL, created_at BIGINT, name TEXT);
             INSERT INTO \"{schema_name}\".\"__drizzle_migrations\" (hash, created_at)
                 VALUES ('half_hash', 1680271923456);"
        ))
        .await
        .expect("reproduce the half-upgraded table");

    let migration = Migration::with_hash(
        "20230331141203_half",
        "half_hash",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{schema_name}\".half_upgraded (id INTEGER)"
        )],
    );
    let outcome = db
        .migrate(
            std::slice::from_ref(&migration),
            Tracking::POSTGRES.schema(schema_name.clone()),
        )
        .await
        .expect("a half-upgraded tracking table is completed");
    assert!(outcome.is_up_to_date(), "{outcome:?}");

    let row = db
        .conn()
        .query_one(
            &format!(
                "SELECT name, applied_at IS NOT NULL FROM \"{schema_name}\".\"__drizzle_migrations\""
            ),
            &[],
        )
        .await
        .expect("upgraded row");
    assert_eq!(row.get::<_, String>(0), "20230331141203_half");
    assert!(row.get::<_, bool>(1));
}

/// Statement errors carry the server's message, and a breakpoint chunk runs
/// whole even when it holds several statements (as drizzle-orm runs it).
#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_migrate_runs_whole_chunks_and_reports_server_errors() {
    let mut db =
        crate::common::helpers::postgres_sync_setup::setup_empty_named("chunk_errors_sync_test");
    let schema_name = db.schema_name().to_string();
    let tracking = Tracking::POSTGRES.schema(schema_name.clone());

    let chunked = Migration::new(
        "20240101000000_chunked",
        &format!(
            "CREATE TABLE \"{schema_name}\".chunk_a (id INTEGER);\n\
             CREATE TABLE \"{schema_name}\".chunk_b (id INTEGER);\n\
             --> statement-breakpoint\n\
             INSERT INTO \"{schema_name}\".chunk_b VALUES (E'1');"
        ),
    );
    assert_eq!(chunked.statements().len(), 2);
    db.migrate(std::slice::from_ref(&chunked), tracking.clone())
        .expect("a multi-statement chunk runs whole");

    let broken = Migration::new(
        "20240102000000_broken",
        &format!("INSERT INTO \"{schema_name}\".missing_table VALUES (1);"),
    );
    let error = db
        .migrate(&[chunked, broken], tracking)
        .expect_err("a failing statement fails the migration");
    let text = error.to_string();
    assert!(text.contains("20240102000000_broken"), "{text}");
    assert!(text.contains("missing_table"), "{text}");
    assert!(
        text.contains("does not exist"),
        "server message is kept: {text}"
    );
}
