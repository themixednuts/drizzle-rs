#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(root: &Path, db_path: &Path, migrations_dir: &Path) {
    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            r#"
dialect = "sqlite"
schema = '{schema_path}'
out = '{out_dir}'

[dbCredentials]
url = '{db_url}'
"#,
            schema_path = root.join("schema.rs").to_string_lossy(),
            out_dir = migrations_dir.to_string_lossy(),
            db_url = db_path.to_string_lossy()
        ),
    )
    .expect("write config");

    fs::write(root.join("schema.rs"), "// test schema\n").expect("write schema");
}

fn migration_tags(migrations_dir: &Path) -> Vec<String> {
    if !migrations_dir.exists() {
        return Vec::new();
    }

    let mut tags = fs::read_dir(migrations_dir)
        .expect("read migrations dir")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().ok()?.is_dir() && name != "meta" {
                Some(name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    tags.sort();
    tags
}

fn generate_custom_migration(root: &Path, migrations_dir: &Path, name: &str) -> String {
    let before = migration_tags(migrations_dir)
        .into_iter()
        .collect::<HashSet<_>>();

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--custom", "--name", name])
        .assert()
        .success();

    migration_tags(migrations_dir)
        .into_iter()
        .find(|tag| !before.contains(tag))
        .expect("find generated migration tag")
}

fn table_exists(conn: &rusqlite::Connection, name: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
        [name],
        |row| row.get(0),
    )
    .expect("query sqlite_master")
}

#[test]
fn migrate_plan_is_dry_run() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    write_config(root, &db_path, &migrations_dir);

    let tag = generate_custom_migration(root, &migrations_dir, "plan");
    fs::write(
        migrations_dir.join(&tag).join("migration.sql"),
        "CREATE TABLE plan_only_table (id INTEGER PRIMARY KEY);\n",
    )
    .expect("write migration.sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--plan"])
        .assert()
        .success();

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    assert_eq!(table_exists(&conn, "plan_only_table"), 0);
    assert_eq!(table_exists(&conn, "__drizzle_migrations"), 0);
}

#[test]
fn migrate_verify_detects_hash_drift() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    write_config(root, &db_path, &migrations_dir);

    let tag = generate_custom_migration(root, &migrations_dir, "verify");
    let migration_sql = migrations_dir.join(&tag).join("migration.sql");

    fs::write(
        &migration_sql,
        "CREATE TABLE drift_original (id INTEGER PRIMARY KEY);\n",
    )
    .expect("write initial migration.sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();

    fs::write(
        &migration_sql,
        "CREATE TABLE drift_changed (id INTEGER PRIMARY KEY);\n",
    )
    .expect("rewrite migration.sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--verify"])
        .assert()
        .failure()
        .stderr(contains("hash mismatch").or(contains("hash mismatch".to_uppercase())));
}

#[test]
fn migrate_safe_applies_after_verification() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    write_config(root, &db_path, &migrations_dir);

    let tag = generate_custom_migration(root, &migrations_dir, "safe");
    fs::write(
        migrations_dir.join(&tag).join("migration.sql"),
        "CREATE TABLE safe_table (id INTEGER PRIMARY KEY);\n",
    )
    .expect("write migration.sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--safe"])
        .assert()
        .success();

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    assert_eq!(table_exists(&conn, "safe_table"), 1);
    let applied_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM __drizzle_migrations", [], |row| {
            row.get(0)
        })
        .expect("count metadata rows");
    assert_eq!(applied_count, 1);
}

#[test]
fn migrate_safe_fails_before_apply_when_verify_fails() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    write_config(root, &db_path, &migrations_dir);

    let first_tag = generate_custom_migration(root, &migrations_dir, "first");
    let first_sql = migrations_dir.join(&first_tag).join("migration.sql");
    fs::write(
        &first_sql,
        "CREATE TABLE safe_first_original (id INTEGER PRIMARY KEY);\n",
    )
    .expect("write first migration.sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();

    // Introduce drift in already-applied migration.
    fs::write(
        &first_sql,
        "CREATE TABLE safe_first_changed (id INTEGER PRIMARY KEY);\n",
    )
    .expect("rewrite first migration.sql");

    // Add a second pending migration that should not run because verification fails first.
    let second_tag = generate_custom_migration(root, &migrations_dir, "second");
    fs::write(
        migrations_dir.join(&second_tag).join("migration.sql"),
        "CREATE TABLE safe_second_pending (id INTEGER PRIMARY KEY);\n",
    )
    .expect("write second migration.sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--safe"])
        .assert()
        .failure();

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    assert_eq!(table_exists(&conn, "safe_second_pending"), 0);
}

#[test]
fn migrate_rejects_conflicting_safe_and_plan_flags() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    write_config(root, &db_path, &migrations_dir);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--safe", "--plan"])
        .assert()
        .failure()
        .stderr(contains("can't be combined"));
}
