#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use tempfile::tempdir;

#[test]
fn push_explain_prints_sql_plan_without_applying() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Minimal schema file
    fs::write(
        root.join("schema.rs"),
        r#"
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Users {
  #[column(primary)]
  pub id: i64,
  pub email: String,
}
"#,
    )
    .unwrap();

    // Minimal config using a local sqlite path in temp dir
    let db_path = root.join("dev.db");
    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            r#"
dialect = "sqlite"
schema = '{schema_path}'
out = '{out_dir}'

[migrations]
table = "__drizzle_migrations"

[dbCredentials]
url = '{db_url}'
"#,
            schema_path = root.join("schema.rs").to_string_lossy(),
            out_dir = root.join("migrations").to_string_lossy(),
            db_url = db_path.to_string_lossy()
        ),
    )
    .unwrap();

    // Run `drizzle push --explain`
    let mut cmd = cargo_bin_cmd!("drizzle");
    cmd.current_dir(root)
        .args(["push", "--explain"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("--- Planned SQL ---")
                .and(predicates::str::contains("CREATE TABLE `users`"))
                .and(predicates::str::contains("--- End SQL ---")),
        );
}
