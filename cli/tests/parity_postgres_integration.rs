#![cfg(feature = "postgres-sync")]

use assert_cmd::cargo::cargo_bin_cmd;
use postgres::{Client, NoTls};
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

fn pg_client() -> Client {
    Client::connect(&pg_url(), NoTls).unwrap_or_else(|e| {
        panic!(
            "failed to connect to postgres for integration test: {e}. \
             Start test DB (e.g. `docker compose up -d postgres`) or set DRIZZLE_TEST_DATABASE_URL"
        )
    })
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
fn push_explain_schema_filters_cli_override_config() {
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

#[PostgresTable(schema = "app")]
pub struct AppTemp {
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
schema = '{schema}'
schemaFilter = ["public"]
tablesFilter = ["public_users"]

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
            "app_*,!app_temp",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("CREATE SCHEMA \"app\"")
                .and(predicates::str::contains("app_events"))
                .and(predicates::str::contains("app_temp").not())
                .and(predicates::str::contains("public_users").not()),
        );
}

#[test]
fn push_explain_defaults_to_public_schema_filter_for_postgres() {
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
        .args(["--config", &cfg_path.to_string_lossy(), "push", "--explain"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("public_users")
                .and(predicates::str::contains("app_events").not())
                .and(predicates::str::contains("CREATE SCHEMA \"app\"").not()),
        );
}

#[test]
fn push_explain_extensions_filter_excludes_postgis_like_objects() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let schema_path = root.join("schema.rs");

    fs::write(
        &schema_path,
        r#"
#[PostgresTable(schema = "topology")]
pub struct TopologyLayer {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(name = "spatial_ref_sys")]
pub struct SpatialRefSys {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable]
pub struct Users {
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
            "--extensionsFilters",
            "postgis",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("users")
                .and(predicates::str::contains("spatial_ref_sys").not())
                .and(predicates::str::contains("topology").not()),
        );
}

#[test]
fn pull_postgres_applies_schema_table_filters_casing_and_breakpoints() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let out_dir = root.join("pulled");

    let suffix = unique_suffix();
    let schema = format!("cli_parity_{suffix}");
    let table_logs = format!("audit_logs_{suffix}");
    let table_meta = format!("audit_meta_{suffix}");
    let table_skip = format!("temp_logs_{suffix}");

    let mut pg = pg_client();
    pg.batch_execute(&format!(
        r#"
CREATE SCHEMA IF NOT EXISTS "{schema}";
DROP TABLE IF EXISTS "{schema}"."{table_logs}";
DROP TABLE IF EXISTS "{schema}"."{table_meta}";
DROP TABLE IF EXISTS public."{table_skip}";

CREATE TABLE "{schema}"."{table_logs}" (
  id SERIAL PRIMARY KEY,
  user_name TEXT NOT NULL
);

CREATE TABLE "{schema}"."{table_meta}" (
  id SERIAL PRIMARY KEY,
  detail TEXT
);

CREATE TABLE public."{table_skip}" (
  id SERIAL PRIMARY KEY,
  body TEXT
);
"#
    ))
    .expect("seed postgres tables");

    fs::write(
        &cfg_path,
        format!(
            r#"
dialect = "postgresql"
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
            "preserve",
            "--breakpoints",
            "false",
        ])
        .assert()
        .success();

    let schema_rs = fs::read_to_string(out_dir.join("schema.rs")).expect("read schema.rs");
    assert!(schema_rs.contains(&format!("pub {table_logs}")));
    assert!(schema_rs.contains("pub user_name: String"));
    assert!(!schema_rs.contains(&table_skip));

    let migration_dir = first_migration_dir(&out_dir);
    let migration_sql = fs::read_to_string(migration_dir.join("migration.sql")).expect("read sql");
    assert!(migration_sql.contains(&table_logs));
    assert!(migration_sql.contains(&table_meta));
    assert!(!migration_sql.contains(&table_skip));
    assert!(!migration_sql.contains("--> statement-breakpoint"));

    pg.batch_execute(&format!(
        r#"
DROP TABLE IF EXISTS "{schema}"."{table_logs}";
DROP TABLE IF EXISTS "{schema}"."{table_meta}";
DROP TABLE IF EXISTS public."{table_skip}";
DROP SCHEMA IF EXISTS "{schema}";
"#
    ))
    .expect("cleanup postgres tables");
}
