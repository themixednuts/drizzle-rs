#![cfg(feature = "tokio-postgres")]

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

fn pg_url() -> String {
    std::env::var("DRIZZLE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/drizzle_test".to_string())
}

fn block_on<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(future)
}

fn pg_exec(sql: &str) {
    block_on(async {
        let (client, conn) = tokio_postgres::connect(&pg_url(), tokio_postgres::NoTls)
            .await
            .expect("connect tokio-postgres");
        tokio::spawn(async move {
            let _ = conn.await;
        });

        client.batch_execute(sql).await.expect("execute sql");
    });
}

fn first_migration_dir(out_dir: &Path) -> PathBuf {
    fs::read_dir(out_dir)
        .expect("read output dir")
        .filter_map(|e| e.ok())
        .find_map(|entry| {
            let ty = entry.file_type().ok()?;
            let name = entry.file_name();
            if ty.is_dir() && name.to_string_lossy() != "meta" {
                Some(entry.path())
            } else {
                None
            }
        })
        .expect("expected migration folder")
}

fn unique_suffix() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos() as u64;
    base ^ COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn push_explain_with_tokio_driver_honors_filters() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let schema_path = root.join("schema.rs");

    fs::write(
        &schema_path,
        r#"
#[PostgresTable(schema = "public")]
pub struct PublicUsers {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(schema = "app")]
pub struct AppEvents {
    #[column(primary)]
    pub id: i32,
}
"#,
    )
    .expect("write schema");

    fs::write(
        &cfg_path,
        format!(
            r#"
dialect = "postgresql"
driver = "tokio-postgres"
schema = '{schema}'

[dbCredentials]
url = '{url}'
"#,
            schema = schema_path.to_string_lossy(),
            url = pg_url(),
        ),
    )
    .expect("write config");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "push",
            "--explain",
            "--schemaFilters",
            "app",
            "--tablesFilter",
            "app_*",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("app_events")
                .and(predicates::str::contains("public_users").not())
                .and(predicates::str::contains("CREATE SCHEMA \"app\"")),
        );
}

#[test]
fn pull_with_tokio_driver_applies_filters_casing_and_breakpoints() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let out_dir = root.join("pulled");

    let suffix = unique_suffix();
    let schema = format!("cli_async_parity_{suffix}");
    let table_logs = format!("audit_logs_{suffix}");
    let table_skip = format!("skip_logs_{suffix}");

    pg_exec(&format!(
        r#"
CREATE SCHEMA IF NOT EXISTS "{schema}";
DROP TABLE IF EXISTS "{schema}"."{table_logs}";
DROP TABLE IF EXISTS "{schema}"."{table_skip}";

CREATE TABLE "{schema}"."{table_logs}" (
  id SERIAL PRIMARY KEY,
  user_name TEXT NOT NULL
);

CREATE TABLE "{schema}"."{table_skip}" (
  id SERIAL PRIMARY KEY,
  body TEXT
);
"#
    ));

    fs::write(
        &cfg_path,
        format!(
            r#"
dialect = "postgresql"
driver = "tokio-postgres"
out = '{out}'

[dbCredentials]
url = '{url}'
"#,
            out = out_dir.to_string_lossy(),
            url = pg_url(),
        ),
    )
    .expect("write config");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "pull",
            "--schemaFilters",
            &schema,
            "--tablesFilter",
            &format!("audit_*_{suffix}"),
            "--casing",
            "camel",
            "--breakpoints",
            "true",
        ])
        .assert()
        .success();

    let schema_rs = fs::read_to_string(out_dir.join("schema.rs")).expect("read schema.rs");
    assert!(schema_rs.contains("pub userName: String"));
    assert!(!schema_rs.contains(&format!("pub {table_skip}")));

    let migration_dir = first_migration_dir(&out_dir);
    let migration_sql = fs::read_to_string(migration_dir.join("migration.sql")).expect("read sql");
    assert!(migration_sql.contains(&table_logs));
    assert!(!migration_sql.contains(&format!("CREATE TABLE \"{schema}\".\"{table_skip}\"")));
    assert!(migration_sql.contains("--> statement-breakpoint"));

    pg_exec(&format!(
        r#"
DROP TABLE IF EXISTS "{schema}"."{table_logs}";
DROP TABLE IF EXISTS "{schema}"."{table_skip}";
DROP SCHEMA IF EXISTS "{schema}";
"#
    ));
}
