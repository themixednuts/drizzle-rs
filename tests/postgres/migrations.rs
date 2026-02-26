#[cfg(feature = "postgres-sync")]
use drizzle::postgres::prelude::*;
#[cfg(feature = "postgres-sync")]
use drizzle::postgres::sync::Drizzle;
#[cfg(feature = "postgres-sync")]
use postgres::{Client, NoTls};

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
        if let Ok(s) = status {
            if s.success() {
                for _ in 0..30 {
                    thread::sleep(Duration::from_secs(1));
                    if Client::connect(&url, NoTls).is_ok() {
                        return;
                    }
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
