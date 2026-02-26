#[cfg(feature = "postgres-sync")]
use drizzle::postgres::prelude::*;
#[cfg(feature = "postgres-sync")]
use drizzle::postgres::sync::Drizzle;
#[cfg(feature = "postgres-sync")]
use postgres::{Client, NoTls};

#[cfg(feature = "postgres-sync")]
#[PostgresTable(name = "drizzle_push_test_items")]
struct PushItem {
    #[column(primary)]
    id: i32,
    label: String,
    note: Option<String>,
}

#[cfg(feature = "postgres-sync")]
#[derive(PostgresSchema)]
struct PushSchema {
    push_item: PushItem,
}

/// Connect to the test database, ensuring Docker is running.
/// Drops the test table first to start clean.
#[cfg(feature = "postgres-sync")]
fn connect_clean() -> Client {
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
        .batch_execute("DROP TABLE IF EXISTS \"public\".\"drizzle_push_test_items\" CASCADE")
        .expect("cleanup before test");
    client
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_creates_table() {
    let client = connect_clean();
    let (mut db, schema) = Drizzle::new(client, PushSchema::default());

    db.push(&schema).expect("push schema");

    let count: i64 = db
        .conn_mut()
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_name = 'drizzle_push_test_items'",
            &[],
        )
        .expect("query information_schema")
        .get(0);
    assert_eq!(count, 1, "push should create the table");

    // cleanup
    db.conn_mut()
        .batch_execute("DROP TABLE IF EXISTS \"public\".\"drizzle_push_test_items\" CASCADE")
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_is_idempotent() {
    let client = connect_clean();
    let (mut db, schema) = Drizzle::new(client, PushSchema::default());

    db.push(&schema).expect("first push");
    db.push(&schema).expect("second push should be a no-op");

    // cleanup
    db.conn_mut()
        .batch_execute("DROP TABLE IF EXISTS \"public\".\"drizzle_push_test_items\" CASCADE")
        .unwrap();
}

#[cfg(feature = "postgres-sync")]
#[test]
fn postgres_sync_push_table_is_usable() {
    let client = connect_clean();
    let (mut db, schema) = Drizzle::new(client, PushSchema::default());

    db.push(&schema).expect("push schema");

    db.conn_mut()
        .execute(
            "INSERT INTO drizzle_push_test_items (id, label) VALUES (1, 'hello')",
            &[],
        )
        .expect("insert into pushed table");

    let label: String = db
        .conn_mut()
        .query_one(
            "SELECT label FROM drizzle_push_test_items WHERE id = 1",
            &[],
        )
        .expect("select from pushed table")
        .get(0);
    assert_eq!(label, "hello");

    // cleanup
    db.conn_mut()
        .batch_execute("DROP TABLE IF EXISTS \"public\".\"drizzle_push_test_items\" CASCADE")
        .unwrap();
}
