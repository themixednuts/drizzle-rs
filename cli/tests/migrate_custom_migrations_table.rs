#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;

#[test]
fn migrate_uses_custom_migrations_table_name() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    // Minimal config pointing at temp directories + custom migrations table
    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            r#"
dialect = "sqlite"
schema = '{schema_path}'
out = '{out_dir}'

[migrations]
table = "custom_migrations"

[dbCredentials]
url = '{db_url}'
"#,
            schema_path = root.join("schema.rs").to_string_lossy(),
            out_dir = migrations_dir.to_string_lossy(),
            db_url = db_path.to_string_lossy()
        ),
    )
    .unwrap();

    // Schema file isn't used by `generate --custom`, but config validation expects it to exist.
    fs::write(root.join("schema.rs"), "// test schema\n").unwrap();

    // Create a custom migration folder via CLI (ensures journal exists and naming is consistent)
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--custom", "--name", "init"])
        .assert()
        .success();

    // Find the created migration directory (any folder inside migrations/ that isn't "meta")
    let tag = fs::read_dir(&migrations_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if e.file_type().ok()?.is_dir() && name != "meta" {
                Some(name)
            } else {
                None
            }
        })
        .expect("expected a generated migration directory");

    // Write actual SQL into the custom migration
    fs::write(
        migrations_dir.join(&tag).join("migration.sql"),
        "CREATE TABLE test_table (id INTEGER PRIMARY KEY);\n",
    )
    .unwrap();

    // Run `drizzle migrate`
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();

    // Verify the custom migrations table was created (SQLite)
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='custom_migrations';",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(exists, 1, "custom migrations table should exist");
}
