#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
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

    // A plan writes nothing: not the database file, not the tracking table.
    assert!(!db_path.exists(), "--plan created the database file");

    rusqlite::Connection::open(&db_path)
        .expect("open sqlite")
        .execute_batch("CREATE TABLE unrelated (id INTEGER PRIMARY KEY);")
        .expect("create database");
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
        .stderr(contains("verification failed with 1 integrity finding(s)"))
        .stderr(contains("has drifted"));
}

#[test]
fn migrate_plan_warns_on_drift_but_succeeds() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");

    write_config(root, &db_path, &migrations_dir);

    let tag = generate_custom_migration(root, &migrations_dir, "plan_drift");
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

    // --plan (and its --dry-run alias) reports drift as a warning but is not
    // an integrity gate; only --verify/--safe fail on findings.
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--dry-run"])
        .assert()
        .success()
        .stdout(contains("Integrity findings:"))
        .stdout(contains("has drifted"))
        .stdout(contains("Migration plan complete."));
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
        .stderr(contains("--safe can't be combined with --plan"));
}

fn write_tables(root: &Path, tables: &[&str]) {
    let schema: String = tables
        .iter()
        .map(|table| {
            format!("#[SQLiteTable]\npub struct {table} {{\n    #[column(primary)]\n    pub id: i64,\n}}\n")
        })
        .collect();
    fs::write(root.join("schema.rs"), schema).expect("write schema");
}

fn generate(root: &Path, name: &str, out: &Path) -> assert_cmd::assert::Assert {
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--name", name, "--out"])
        .arg(out)
        .assert()
        .success()
}

/// After a git merge brings in another branch's migration folder, `generate`
/// diffs against both branches (drizzle-kit's leaf handling) instead of only
/// the newest folder, which re-emitted the other branch's DDL.
#[test]
fn generate_after_a_branch_merge_does_not_repeat_the_other_branch() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");
    let other_branch = root.join("other_branch");
    write_config(root, &db_path, &migrations_dir);

    write_tables(root, &["Users"]);
    generate(root, "a_base", &migrations_dir);
    let base = migration_tags(&migrations_dir).pop().expect("base tag");
    fs::create_dir_all(other_branch.join(&base)).expect("mkdir");
    for file in ["migration.sql", "snapshot.json"] {
        fs::copy(
            migrations_dir.join(&base).join(file),
            other_branch.join(&base).join(file),
        )
        .expect("copy base");
    }

    write_tables(root, &["Users", "Alpha"]);
    generate(root, "b_alpha", &migrations_dir);
    write_tables(root, &["Users", "Beta"]);
    generate(root, "c_beta", &other_branch);
    let beta = migration_tags(&other_branch).pop().expect("beta tag");
    assert!(beta.ends_with("c_beta"), "{beta}");
    fs::create_dir_all(migrations_dir.join(&beta)).expect("mkdir");
    for file in ["migration.sql", "snapshot.json"] {
        fs::copy(
            other_branch.join(&beta).join(file),
            migrations_dir.join(&beta).join(file),
        )
        .expect("merge beta");
    }

    write_tables(root, &["Users", "Alpha", "Beta"]);
    generate(root, "d_noop", &migrations_dir).stdout(contains("No schema changes"));
    assert_eq!(migration_tags(&migrations_dir).len(), 3);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();
    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    assert_eq!(table_exists(&conn, "alpha"), 1);
    assert_eq!(table_exists(&conn, "beta"), 1);
}

#[test]
fn generate_rejects_conflicting_branches_unless_told_to_ignore_them() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let migrations_dir = root.join("migrations");
    write_config(root, &root.join("dev.db"), &migrations_dir);

    // Both branches create the same table from an empty history.
    write_tables(root, &["Users"]);
    generate(root, "a_left", &migrations_dir);
    let other_branch = root.join("other_branch");
    generate(root, "b_right", &other_branch);
    let right = migration_tags(&other_branch).pop().expect("right tag");
    fs::create_dir_all(migrations_dir.join(&right)).expect("mkdir");
    for file in ["migration.sql", "snapshot.json"] {
        fs::copy(
            other_branch.join(&right).join(file),
            migrations_dir.join(&right).join(file),
        )
        .expect("merge right");
    }

    write_tables(root, &["Users", "Extra"]);
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--name", "c_next"])
        .assert()
        .failure()
        .stderr(contains("non-commutative migrations"));

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--name", "c_next", "--ignore-conflicts"])
        .assert()
        .success();
}
