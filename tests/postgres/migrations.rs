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
    assert_eq!(
        applied_at, None,
        "backfilled legacy rows keep NULL applied_at"
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
    assert_eq!(
        applied_at, None,
        "backfilled legacy rows keep NULL applied_at"
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
