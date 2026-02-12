#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;

#[test]
fn migrate_skips_already_applied_tag_even_if_sql_changes() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

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

    // Create one migration folder and then customize its SQL.
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--custom", "--name", "init"])
        .assert()
        .success();

    let tag = fs::read_dir(&migrations_dir)
        .expect("read migrations dir")
        .filter_map(Result::ok)
        .find_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().ok()?.is_dir() && name != "meta" {
                Some(name)
            } else {
                None
            }
        })
        .expect("find generated migration tag");

    let migration_sql = migrations_dir.join(&tag).join("migration.sql");
    fs::write(
        &migration_sql,
        "CREATE TABLE first_table (id INTEGER PRIMARY KEY);\n",
    )
    .expect("write first migration sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();

    // Change SQL for the same migration tag. Drizzle ORM semantics should not re-run it.
    fs::write(
        &migration_sql,
        "CREATE TABLE second_table (id INTEGER PRIMARY KEY);\n",
    )
    .expect("rewrite migration sql");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");

    let first_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='first_table'",
            [],
            |row| row.get(0),
        )
        .expect("query first_table");
    assert_eq!(first_exists, 1);

    let second_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='second_table'",
            [],
            |row| row.get(0),
        )
        .expect("query second_table");
    assert_eq!(
        second_exists, 0,
        "migration SQL for an already-applied tag should not be executed again"
    );
}
