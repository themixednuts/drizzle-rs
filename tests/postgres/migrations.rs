#[cfg(feature = "postgres-sync")]
use drizzle::postgres::prelude::*;
#[cfg(feature = "postgres-sync")]
use drizzle::postgres::sync::Drizzle;
#[cfg(feature = "tokio-postgres")]
use drizzle::postgres::tokio::Drizzle as TokioDrizzle;
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use drizzle_migrations::{Migration, Tracking};
#[cfg(feature = "postgres-sync")]
use postgres::{Client, NoTls};
#[cfg(feature = "tokio-postgres")]
use tokio_postgres::NoTls as TokioNoTls;

// ---------------------------------------------------------------------------
// Each test gets its own schema so introspection doesn't see other tests'
// objects, allowing parallel execution with the full test suite.
// ---------------------------------------------------------------------------

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

/// Connect to the test database, ensuring Docker is running.
/// Creates a fresh schema for isolation.
#[cfg(feature = "postgres-sync")]
fn connect(schema_name: &str) -> Client {
    use std::process::Command;
    use std::sync::Once;
    use std::thread;
    use std::time::Duration;

    static DOCKER_STARTED: Once = Once::new();

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=postgres dbname=drizzle_test".into()
    });

    DOCKER_STARTED.call_once(|| {
        if Client::connect(&url, NoTls).is_ok() {
            return;
        }
        let status = Command::new("docker")
            .args(["compose", "up", "-d", "postgres"])
            .status();
        if let Ok(s) = status
            && s.success()
        {
            for _ in 0..30 {
                thread::sleep(Duration::from_secs(1));
                if Client::connect(&url, NoTls).is_ok() {
                    return;
                }
            }
        }
        panic!("PostgreSQL not available");
    });

    let mut client = Client::connect(&url, NoTls).expect("connect");
    client
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE; CREATE SCHEMA \"{}\"",
            schema_name, schema_name
        ))
        .expect("setup test schema");
    client
}

#[cfg(feature = "postgres-sync")]
fn legacy_tracking_columns_sync(client: &mut Client, schema: &str, table: &str) -> Vec<String> {
    client
        .query(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
            &[&schema, &table],
        )
        .expect("query information_schema.columns")
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect()
}

#[cfg(feature = "postgres-sync")]
fn create_legacy_tracking_table_sync(client: &mut Client, schema: &str, table: &str) {
    client
        .batch_execute(&format!(
            "CREATE TABLE \"{schema}\".\"{table}\" (id SERIAL PRIMARY KEY, hash TEXT NOT NULL, created_at BIGINT)"
        ))
        .expect("create legacy tracking table");
}

#[cfg(feature = "tokio-postgres")]
async fn connect_tokio(schema_name: &str) -> tokio_postgres::Client {
    use std::process::Command;
    use tokio::time::{Duration, sleep};

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=postgres dbname=drizzle_test".into()
    });

    let mut ready = tokio_postgres::connect(&url, TokioNoTls).await.ok();
    if ready.is_none() {
        let status = Command::new("docker")
            .args(["compose", "up", "-d", "postgres"])
            .status();
        if let Ok(s) = status
            && s.success()
        {
            for _ in 0..30 {
                sleep(Duration::from_secs(1)).await;
                if let Ok(conn) = tokio_postgres::connect(&url, TokioNoTls).await {
                    ready = Some(conn);
                    break;
                }
            }
        }
    }

    let (client, connection) = ready.expect("connect tokio-postgres");
    tokio::spawn(async move {
        let _ = connection.await;
    });

    client
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE; CREATE SCHEMA \"{}\"",
            schema_name, schema_name
        ))
        .await
        .expect("setup test schema");
    client
}

#[cfg(feature = "tokio-postgres")]
async fn legacy_tracking_columns_tokio(
    client: &tokio_postgres::Client,
    schema: &str,
    table: &str,
) -> Vec<String> {
    client
        .query(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
            &[&schema, &table],
        )
        .await
        .expect("query information_schema.columns")
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect()
}

#[cfg(feature = "tokio-postgres")]
async fn create_legacy_tracking_table_tokio(
    client: &tokio_postgres::Client,
    schema: &str,
    table: &str,
) {
    client
        .batch_execute(&format!(
            "CREATE TABLE \"{schema}\".\"{table}\" (id SERIAL PRIMARY KEY, hash TEXT NOT NULL, created_at BIGINT)"
        ))
        .await
        .expect("create legacy tracking table");
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_creates_table() {
    let schema_name = "push_creates_test";
    let client = connect(schema_name);
    let (mut db, schema) = Drizzle::new(client, PushCreatesSchema::default());

    db.push(&schema).expect("push schema");

    let count: i64 = db
        .conn_mut()
        .query_one(
            &format!(
                "SELECT COUNT(*) FROM information_schema.tables \
                 WHERE table_schema = '{}' AND table_name = 'items'",
                schema_name
            ),
            &[],
        )
        .expect("query information_schema")
        .get(0);
    assert_eq!(count, 1, "push should create the table");

    // cleanup
    db.conn_mut()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_is_idempotent() {
    let schema_name = "push_idempotent_test";
    let client = connect(schema_name);
    let (mut db, schema) = Drizzle::new(client, PushIdempotentSchema::default());

    db.push(&schema).expect("first push");
    db.push(&schema).expect("second push should be a no-op");

    // cleanup
    db.conn_mut()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_table_is_usable() {
    let schema_name = "push_usable_test";
    let client = connect(schema_name);
    let (mut db, schema) = Drizzle::new(client, PushUsableSchema::default());

    db.push(&schema).expect("push schema");

    // serial column auto-generates the id — don't hardcode it
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

    // cleanup
    db.conn_mut()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_runtime_migrate_upgrades_legacy_tracking_table() {
    let schema_name = "runtime_upgrade_sync_test";
    let mut client = connect(schema_name);
    create_legacy_tracking_table_sync(&mut client, schema_name, "__drizzle_migrations");
    client
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_a", &1_680_271_923_000_i64],
        )
        .expect("insert legacy migration row");

    let (mut db, _) = Drizzle::new(client, ());
    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
            schema_name
        )],
    );

    db.migrate(
        &[migration],
        Tracking::POSTGRES.schema(schema_name.to_string()),
    )
    .expect("upgrade legacy runtime metadata");

    let columns = legacy_tracking_columns_sync(db.conn_mut(), schema_name, "__drizzle_migrations");
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

    let migrated_table_exists: i64 = db
        .conn_mut()
        .query_one(
            "SELECT COUNT(*)::bigint FROM information_schema.tables WHERE table_schema = $1 AND table_name = 'runtime_created_at_a'",
            &[&schema_name],
        )
        .expect("query information_schema.tables")
        .get(0);
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );

    db.conn_mut()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_runtime_migrate_upgrade_uses_hash_for_same_timestamp() {
    let schema_name = "runtime_upgrade_collision_sync_test";
    let mut client = connect(schema_name);
    create_legacy_tracking_table_sync(&mut client, schema_name, "__drizzle_migrations");
    client
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_b", &1_680_271_923_000_i64],
        )
        .expect("insert legacy migration row");

    let (mut db, _) = Drizzle::new(client, ());
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

    db.migrate(
        &migrations,
        Tracking::POSTGRES.schema(schema_name.to_string()),
    )
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

    db.conn_mut()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_runtime_migrate_upgrade_rejects_unmatched_legacy_rows() {
    let schema_name = "runtime_upgrade_unmatched_sync_test";
    let mut client = connect(schema_name);
    create_legacy_tracking_table_sync(&mut client, schema_name, "__drizzle_migrations");
    client
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"unknown_hash", &1_680_271_924_000_i64],
        )
        .expect("insert unmatched legacy row");

    let (mut db, _) = Drizzle::new(client, ());
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
        .migrate(
            &[migration],
            Tracking::POSTGRES.schema(schema_name.to_string()),
        )
        .expect_err("unmatched legacy metadata should fail");
    assert!(err.to_string().contains("do not match local migrations"));

    let columns = legacy_tracking_columns_sync(db.conn_mut(), schema_name, "__drizzle_migrations");
    assert_eq!(columns, vec!["id", "hash", "created_at"]);

    db.conn_mut()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .unwrap();
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn tokio_postgres_runtime_migrate_upgrades_legacy_tracking_table() {
    let schema_name = "runtime_upgrade_tokio_test";
    let client = connect_tokio(schema_name).await;
    create_legacy_tracking_table_tokio(&client, schema_name, "__drizzle_migrations").await;
    client
        .execute(
            &format!(
                "INSERT INTO \"{}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)",
                schema_name
            ),
            &[&"runtime_hash_a", &1_680_271_923_000_i64],
        )
        .await
        .expect("insert legacy migration row");

    let (mut db, _) = TokioDrizzle::new(client, ());
    let migration = Migration::with_hash(
        "20230331141203_runtime_first",
        "runtime_hash_a",
        1_680_271_923_000,
        vec![format!(
            "CREATE TABLE \"{}\".runtime_created_at_a (id INTEGER PRIMARY KEY)",
            schema_name
        )],
    );

    db.migrate(
        &[migration],
        Tracking::POSTGRES.schema(schema_name.to_string()),
    )
    .await
    .expect("upgrade legacy runtime metadata");

    let columns =
        legacy_tracking_columns_tokio(db.conn(), schema_name, "__drizzle_migrations").await;
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

    let migrated_table_exists: i64 = db
        .conn()
        .query_one(
            "SELECT COUNT(*)::bigint FROM information_schema.tables WHERE table_schema = $1 AND table_name = 'runtime_created_at_a'",
            &[&schema_name],
        )
        .await
        .expect("query information_schema.tables")
        .get(0);
    assert_eq!(
        migrated_table_exists, 0,
        "already-applied migration should not run again during metadata upgrade"
    );

    db.conn()
        .batch_execute(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .await
        .unwrap();
}
