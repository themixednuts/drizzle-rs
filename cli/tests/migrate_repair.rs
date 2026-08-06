//! `drizzle migrate --repair` end-to-end coverage.
//!
//! Reproduces the reported incident: a process killed after part of a
//! migration landed but before the migration was recorded. Two-phase tracking
//! leaves the migration marked dirty (`applied_at` NULL); a plain `migrate`
//! must refuse and name it, and `--repair` must reconcile the remainder.

#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const TAG: &str = "20260801000000_partial";
const SQL: &str = "CREATE TABLE repair_first (id INTEGER PRIMARY KEY);\n\
                   --> statement-breakpoint\n\
                   CREATE TABLE repair_second (id INTEGER PRIMARY KEY);\n";

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
            db_url = db_path.to_string_lossy(),
        ),
    )
    .expect("write config");
    fs::write(root.join("schema.rs"), "// test schema\n").expect("write schema");
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .expect("count table");
    count == 1
}

/// Lay down a migration folder and a database that is stuck mid-apply.
fn setup_interrupted(root: &Path) -> std::path::PathBuf {
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");
    write_config(root, &db_path, &migrations_dir);

    let dir = migrations_dir.join(TAG);
    fs::create_dir_all(&dir).expect("create migration dir");
    fs::write(dir.join("migration.sql"), SQL).expect("write migration sql");

    let migrations = drizzle_migrations::MigrationDir::new(&migrations_dir)
        .discover()
        .expect("discover migrations");
    let migration = &migrations[0];
    let set = drizzle_migrations::Migrations::with_tracking(
        migrations.clone(),
        drizzle_types::Dialect::SQLite,
        drizzle_types::MigrationTracking::SQLITE,
    );

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    conn.execute(&set.create_table_sql(), [])
        .expect("create tracking table");
    // Phase 1 of two-phase tracking: recorded as started...
    conn.execute(&set.record_migration_started_sql(migration), [])
        .expect("record started");
    // ...first statement lands, then the process dies before phase 3.
    conn.execute(&migration.statements()[0], [])
        .expect("apply first statement");
    drop(conn);

    db_path
}

#[test]
fn migrate_refuses_an_interrupted_migration_and_names_it() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    setup_interrupted(root);

    let output = cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .output()
        .expect("run drizzle migrate");

    assert!(!output.status.success(), "migrate must fail on a dirty row");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains(TAG), "{text}");
    assert!(text.contains("interrupted mid-apply"), "{text}");
    assert!(text.contains("--repair"), "{text}");
}

#[test]
fn migrate_repair_reconciles_and_completes_the_migration() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = setup_interrupted(root);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--repair"])
        .assert()
        .success();

    let conn = rusqlite::Connection::open(&db_path).expect("reopen sqlite");
    assert!(
        table_exists(&conn, "repair_second"),
        "repair must run the statement that never landed"
    );
    let dirty: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM __drizzle_migrations WHERE applied_at IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("count dirty rows");
    assert_eq!(dirty, 0, "repair must clear the dirty marker");
    drop(conn);

    // A plain re-run is now a clean no-op.
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No pending migrations"));
}

#[test]
fn repair_is_rejected_with_read_only_modes() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    setup_interrupted(root);

    for mode in ["--verify", "--plan"] {
        cargo_bin_cmd!("drizzle")
            .current_dir(root)
            .args(["migrate", "--repair", mode])
            .assert()
            .failure();
    }
}
