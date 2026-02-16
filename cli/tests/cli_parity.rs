use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use tempfile::tempdir;

#[test]
fn push_strict_flag_is_rejected() {
    cargo_bin_cmd!("drizzle")
        .args(["push", "--strict"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "unexpected argument '--strict' found",
        ));
}

#[test]
fn push_help_includes_filter_flags() {
    cargo_bin_cmd!("drizzle")
        .args(["push", "--help"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("--tablesFilter")
                .and(predicates::str::contains("--schemaFilters"))
                .and(predicates::str::contains("--extensionsFilters"))
                .and(predicates::str::contains("--strict").not()),
        );
}

#[test]
fn check_and_up_accept_dialect_override() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let schema_path = root.join("schema.rs");

    fs::write(
        &schema_path,
        r#"
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
out = '{out}'

[dbCredentials]
url = "postgres://postgres:postgres@localhost:5432/drizzle_test"
"#,
            schema = schema_path.to_string_lossy(),
            out = root.join("migrations").to_string_lossy(),
        ),
    )
    .expect("write config");

    cargo_bin_cmd!("drizzle")
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "check",
            "--dialect",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Dialect: sqlite"));

    cargo_bin_cmd!("drizzle")
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "up",
            "--dialect",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("No migrations folder found"));
}
