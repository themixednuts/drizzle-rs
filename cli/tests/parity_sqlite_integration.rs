#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

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

fn write_sqlite_db(path: &Path) {
    let conn = rusqlite::Connection::open(path).expect("open sqlite db");
    conn.execute_batch(
        r#"
CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY,
    user_name TEXT NOT NULL
);

CREATE TABLE audit_meta (
    id INTEGER PRIMARY KEY,
    detail TEXT
);

CREATE TABLE temp_logs (
    id INTEGER PRIMARY KEY,
    body TEXT
);
"#,
    )
    .expect("seed db");
}

/// Split migration SQL into individual CREATE TABLE statements, returning a
/// sorted vec of `(table_name, sorted_columns)` for order-independent comparison.
/// Each column is a trimmed line like "`id` INTEGER PRIMARY KEY".
fn parse_create_tables(sql: &str) -> Vec<(String, Vec<String>)> {
    let normalized = sql.replace("--> statement-breakpoint\n", "");
    let mut tables: Vec<(String, Vec<String>)> = normalized
        .split(";\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|stmt| {
            // Extract table name: CREATE TABLE `name` (
            let name_start = stmt.find('`').expect("backtick start") + 1;
            let name_end = stmt[name_start..].find('`').expect("backtick end") + name_start;
            let table_name = stmt[name_start..name_end].to_string();

            // Extract columns: everything between ( and )
            let paren_start = stmt.find('(').expect("open paren") + 1;
            let paren_end = stmt.rfind(')').expect("close paren");
            let mut cols: Vec<String> = stmt[paren_start..paren_end]
                .split(",\n")
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect();
            cols.sort();
            (table_name, cols)
        })
        .collect();
    tables.sort_by(|a, b| a.0.cmp(&b.0));
    tables
}

#[test]
fn generate_and_export_honor_schema_out_and_breakpoints_overrides() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let schema_a = root.join("schema_a.rs");
    let schema_b = root.join("schema_b.rs");
    let out_dir = root.join("generated");

    fs::write(
        &schema_a,
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub email: String,
}
"#,
    )
    .expect("write schema a");
    fs::write(
        &schema_b,
        r#"
#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
    pub user_id: i64,
}
"#,
    )
    .expect("write schema b");

    fs::write(
        &cfg_path,
        format!(
            r#"
dialect = "sqlite"
schema = "missing.rs"
out = '{out}'

[dbCredentials]
url = '{db_url}'
"#,
            out = root.join("from_config").to_string_lossy(),
            db_url = root.join("dev.db").to_string_lossy(),
        ),
    )
    .expect("write config");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "generate",
            "--dialect",
            "sqlite",
            "--driver",
            "rusqlite",
            "--schema",
            &format!(
                "{},{}",
                schema_a.to_string_lossy(),
                schema_b.to_string_lossy()
            ),
            "--out",
            &out_dir.to_string_lossy(),
            "--name",
            "init",
            "--breakpoints",
            "false",
        ])
        .assert()
        .success();

    let migration_dir = first_migration_dir(&out_dir);
    let migration_sql = fs::read_to_string(migration_dir.join("migration.sql")).expect("read sql");

    let tables = parse_create_tables(&migration_sql);
    assert_eq!(
        tables,
        vec![
            (
                "posts".to_string(),
                vec![
                    "`id` INTEGER PRIMARY KEY".to_string(),
                    "`user_id` INTEGER NOT NULL".to_string(),
                ]
            ),
            (
                "users".to_string(),
                vec![
                    "`email` TEXT NOT NULL".to_string(),
                    "`id` INTEGER PRIMARY KEY".to_string(),
                ]
            ),
        ]
    );
    assert!(
        !migration_sql.contains("--> statement-breakpoint"),
        "breakpoints should be disabled"
    );

    let exported_sql = root.join("export.sql");
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "export",
            "--dialect",
            "sqlite",
            "--schema",
            &format!(
                "{},{}",
                schema_a.to_string_lossy(),
                schema_b.to_string_lossy()
            ),
            "--sql",
            &exported_sql.to_string_lossy(),
        ])
        .assert()
        .success();

    let exported = fs::read_to_string(&exported_sql).expect("read export");
    let export_tables = parse_create_tables(&exported);
    assert_eq!(
        export_tables,
        vec![
            (
                "posts".to_string(),
                vec![
                    "`id` INTEGER PRIMARY KEY".to_string(),
                    "`user_id` INTEGER NOT NULL".to_string(),
                ]
            ),
            (
                "users".to_string(),
                vec![
                    "`email` TEXT NOT NULL".to_string(),
                    "`id` INTEGER PRIMARY KEY".to_string(),
                ]
            ),
        ]
    );
}

#[test]
fn push_explain_uses_cli_table_filters_over_config_and_dialect_override() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let schema_path = root.join("schema.rs");
    let db_path = root.join("dev.db");

    fs::write(
        &schema_path,
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteTable]
pub struct UsersTmp {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteTable]
pub struct Audit {
    #[column(primary)]
    pub id: i64,
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
tablesFilter = ["audit"]

[dbCredentials]
url = "postgres://postgres:postgres@localhost:5432/drizzle_test"
"#,
            schema = schema_path.to_string_lossy(),
        ),
    )
    .expect("write config");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "push",
            "--dialect",
            "sqlite",
            "--schema",
            &schema_path.to_string_lossy(),
            "--url",
            &db_path.to_string_lossy(),
            "--tablesFilter",
            "users*,!users_tmp",
            "--explain",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("CREATE TABLE `users`")
                .and(predicates::str::contains("users_tmp").not())
                .and(predicates::str::contains("audit").not()),
        );
}

#[test]
fn introspect_and_pull_apply_filters_casing_and_breakpoints() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let db_path = root.join("dev.db");
    write_sqlite_db(&db_path);

    fs::write(
        &cfg_path,
        format!(
            r#"
dialect = "sqlite"
schema = "src/schema.rs"
out = '{out}'

[dbCredentials]
url = '{db_url}'
"#,
            out = root.join("introspected").to_string_lossy(),
            db_url = db_path.to_string_lossy(),
        ),
    )
    .expect("write config");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "introspect",
            "--tablesFilter",
            "audit_*",
            "--casing",
            "preserve",
            "--breakpoints",
            "false",
        ])
        .assert()
        .success();

    let out_dir = root.join("introspected");
    let schema_rs =
        fs::read_to_string(out_dir.join("schema.rs")).expect("read introspected schema");
    assert_eq!(
        schema_rs,
        "\
//! Auto-generated SQLite schema from introspection
//!
//! Schema introspected from filtered database objects

use drizzle::sqlite::prelude::*;

#[SQLiteTable(name = \"audit_logs\")]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i64,
    pub user_name: String,
}

#[SQLiteTable(name = \"audit_meta\")]
pub struct AuditMeta {
    #[column(primary)]
    pub id: i64,
    pub detail: Option<String>,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub audit_logs: AuditLogs,
    pub audit_meta: AuditMeta,
}
"
    );

    let migration_dir = first_migration_dir(&out_dir);
    let migration_sql = fs::read_to_string(migration_dir.join("migration.sql")).expect("read sql");
    let tables = parse_create_tables(&migration_sql);
    assert_eq!(
        tables,
        vec![
            (
                "audit_logs".to_string(),
                vec![
                    "`id` INTEGER PRIMARY KEY".to_string(),
                    "`user_name` TEXT NOT NULL".to_string(),
                ]
            ),
            (
                "audit_meta".to_string(),
                vec![
                    "`detail` TEXT".to_string(),
                    "`id` INTEGER PRIMARY KEY".to_string(),
                ]
            ),
        ]
    );
    assert!(
        !migration_sql.contains("--> statement-breakpoint"),
        "breakpoints should be disabled"
    );
    assert!(
        !migration_sql.contains("temp_logs"),
        "temp_logs should be filtered out"
    );

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "pull",
            "--tablesFilter",
            "audit_*",
            "--casing",
            "camel",
            "--breakpoints",
            "true",
            "--out",
            &root.join("pulled").to_string_lossy(),
        ])
        .assert()
        .success();

    let pulled_dir = root.join("pulled");
    let pulled_schema =
        fs::read_to_string(pulled_dir.join("schema.rs")).expect("read pulled schema");
    assert_eq!(
        pulled_schema,
        "\
//! Auto-generated SQLite schema from introspection
//!
//! Schema introspected from filtered database objects

use drizzle::sqlite::prelude::*;

#[SQLiteTable(name = \"audit_logs\")]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i64,
    pub userName: String,
}

#[SQLiteTable(name = \"audit_meta\")]
pub struct AuditMeta {
    #[column(primary)]
    pub id: i64,
    pub detail: Option<String>,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub auditLogs: AuditLogs,
    pub auditMeta: AuditMeta,
}
"
    );

    let pulled_migration_dir = first_migration_dir(&pulled_dir);
    let pulled_sql =
        fs::read_to_string(pulled_migration_dir.join("migration.sql")).expect("read pulled sql");
    assert!(
        pulled_sql.contains("--> statement-breakpoint"),
        "breakpoints should be enabled"
    );
    let pulled_tables = parse_create_tables(&pulled_sql);
    assert_eq!(
        pulled_tables,
        vec![
            (
                "audit_logs".to_string(),
                vec![
                    "`id` INTEGER PRIMARY KEY".to_string(),
                    "`user_name` TEXT NOT NULL".to_string(),
                ]
            ),
            (
                "audit_meta".to_string(),
                vec![
                    "`detail` TEXT".to_string(),
                    "`id` INTEGER PRIMARY KEY".to_string(),
                ]
            ),
        ]
    );
    assert!(
        !pulled_sql.contains("temp_logs"),
        "temp_logs should be filtered out"
    );
}

#[test]
fn sqlite_commands_warn_when_postgres_only_filters_are_passed() {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    let cfg_path = root.join("drizzle.config.toml");
    let schema_path = root.join("schema.rs");
    let db_path = root.join("dev.db");

    write_sqlite_db(&db_path);
    fs::write(
        &schema_path,
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}
"#,
    )
    .expect("write schema");

    fs::write(
        &cfg_path,
        format!(
            r#"
dialect = "sqlite"
schema = '{schema}'
out = '{out}'

[dbCredentials]
url = '{db_url}'
"#,
            schema = schema_path.to_string_lossy(),
            out = root.join("out").to_string_lossy(),
            db_url = db_path.to_string_lossy(),
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
            "public",
            "--extensionsFilters",
            "postgis",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("Ignoring --schemaFilters: only supported for postgresql")
                .and(predicates::str::contains(
                    "Ignoring --extensionsFilters: only supported for postgresql",
                )),
        );

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &cfg_path.to_string_lossy(),
            "introspect",
            "--schemaFilters",
            "public",
            "--extensionsFilters",
            "postgis",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("Ignoring --schemaFilters: only supported for postgresql")
                .and(predicates::str::contains(
                    "Ignoring --extensionsFilters: only supported for postgresql",
                )),
        );
}
