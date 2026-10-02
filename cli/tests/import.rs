//! `drizzle import` on real drizzle-kit output.
//!
//! `fixtures/import/{stable,beta}/<dialect>` were written by drizzle-kit
//! 0.31.11 (journal layout) and 1.0.0-rc.4 (folder layout) from the schemas
//! in `fixtures/import/ts`, each with two migrations (`init`, then `full`).
//!
//! The round-trip tests import the newest snapshot, compare the Rust with
//! `fixtures/import/expected/*.rs` (which the root crate's
//! `imported_schemas` test compiles with the real macros), parse it the way
//! `drizzle generate` does, and diff it against the snapshot: any planned
//! statement means the generated schema lost or changed something.
//!
//! Set `UPDATE_IMPORT_GOLDEN=1` to rewrite the expected files.

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use drizzle_migrations::parser::SchemaParser;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::upgrade::upgrade_to_latest;
use drizzle_types::Dialect;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/import")
}

fn drizzle(dir: &Path) -> Command {
    let mut cmd = cargo_bin_cmd!("drizzle");
    cmd.current_dir(dir).env("NO_COLOR", "1");
    cmd
}

/// Copies a fixture folder so a test can change it.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy");
    for entry in fs::read_dir(from).expect("read fixture") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// The newest snapshot of a fixture folder, upgraded and loaded.
fn newest_snapshot(dir: &Path, dialect: Dialect) -> Snapshot {
    let path = if dir.join("meta/_journal.json").exists() {
        dir.join("meta/0001_snapshot.json")
    } else {
        let mut folders: Vec<_> = fs::read_dir(dir)
            .expect("read fixture")
            .map(|e| e.expect("entry").path())
            .filter(|p| p.join("snapshot.json").exists())
            .collect();
        folders.sort();
        folders
            .pop()
            .expect("a migration folder")
            .join("snapshot.json")
    };
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read snapshot")).expect("json");
    let upgraded = upgrade_to_latest(json, dialect);
    match dialect {
        Dialect::SQLite => Snapshot::Sqlite(serde_json::from_value(upgraded).expect("sqlite")),
        Dialect::PostgreSQL => {
            Snapshot::Postgres(serde_json::from_value(upgraded).expect("postgres"))
        }
        Dialect::MySQL => Snapshot::MySQL(serde_json::from_value(upgraded).expect("mysql")),
    }
}

/// Imports `fixtures/import/<rel>`, checks the code against the expected
/// file, and asserts that the generated schema diffs clean against the
/// imported snapshot.
fn assert_round_trip(rel: &str, dialect: Dialect) {
    let output = drizzle(&fixtures())
        .args(["import", rel, "--out", "-"])
        .output()
        .expect("run drizzle import");
    assert!(
        output.status.success(),
        "drizzle import failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let code = String::from_utf8(output.stdout).expect("utf-8 schema");

    let expected_path = fixtures()
        .join("expected")
        .join(format!("{}.rs", rel.replace('/', "_")));
    if std::env::var_os("UPDATE_IMPORT_GOLDEN").is_some() {
        fs::write(&expected_path, &code).expect("write expected schema");
    }
    let expected = fs::read_to_string(&expected_path).expect("read expected schema");
    assert_eq!(
        code, expected,
        "generated schema for {rel} changed; rerun with UPDATE_IMPORT_GOLDEN=1 if intended"
    );

    let parsed = SchemaParser::parse(&code);
    assert!(
        parsed.errors.is_empty(),
        "generated schema did not parse:\n{code}\nerrors: {:#?}",
        parsed.errors
    );
    let regenerated = Snapshot::from_parse_result(&parsed, dialect, None);
    let imported = newest_snapshot(&fixtures().join(rel), dialect);
    let plan = drizzle_migrations::diff(&imported, &regenerated).expect("diff");
    assert!(
        plan.statements.is_empty(),
        "generated schema differs from the snapshot in {rel}:\n{code}\nplanned: {:#?}",
        plan.statements
    );
}

#[test]
fn sqlite_stable_round_trips() {
    assert_round_trip("stable/sqlite", Dialect::SQLite);
}

#[test]
fn sqlite_folder_layout_round_trips() {
    assert_round_trip("beta/sqlite", Dialect::SQLite);
}

#[test]
fn postgres_stable_round_trips() {
    assert_round_trip("stable/postgres", Dialect::PostgreSQL);
}

#[test]
fn postgres_folder_layout_round_trips() {
    assert_round_trip("beta/postgres", Dialect::PostgreSQL);
}

#[test]
fn mysql_stable_round_trips() {
    assert_round_trip("stable/mysql", Dialect::MySQL);
}

#[test]
fn mysql_folder_layout_round_trips() {
    assert_round_trip("beta/mysql", Dialect::MySQL);
}

// =============================================================================
// Inputs
// =============================================================================

#[test]
fn folder_layout_uses_the_newest_migration() {
    drizzle(&fixtures())
        .args(["import", "beta/postgres", "--out", "-"])
        .assert()
        .success()
        .stderr(predicate::str::contains("_full/snapshot.json"))
        .stdout(predicate::str::contains("pub struct PostTagVotes"));
}

#[test]
fn journal_layout_uses_the_last_journal_entry() {
    drizzle(&fixtures())
        .args(["import", "stable/sqlite", "--out", "-"])
        .assert()
        .success()
        .stderr(predicate::str::contains("meta/0001_snapshot.json"))
        .stderr(predicate::str::contains("drizzle up --out stable/sqlite"))
        .stdout(predicate::str::contains("pub struct Posts"));
}

#[test]
fn single_snapshot_file_is_imported() {
    // The first migration only has `users`.
    drizzle(&fixtures())
        .args([
            "import",
            "stable/postgres/meta/0000_snapshot.json",
            "--out",
            "-",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pub struct Users"))
        .stdout(predicate::str::contains("pub struct Posts").not())
        // The file sits in a journal-layout folder: still point at `drizzle up`.
        .stderr(predicate::str::contains("drizzle up --out stable/postgres"));
}

#[test]
fn stdout_output_keeps_messages_on_stderr() {
    let output = drizzle(&fixtures())
        .args(["import", "beta/mysql", "--out", "-"])
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(
        stdout.starts_with("//! Auto-generated MySQL schema"),
        "{stdout}"
    );
    assert!(!stdout.contains("Next steps"), "{stdout}");
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(stderr.contains("Next steps"), "{stderr}");
}

#[test]
fn missing_path_and_empty_folder_are_reported() {
    let dir = tempdir().expect("temp dir");
    drizzle(dir.path())
        .args(["import", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nope does not exist"));

    fs::create_dir(dir.path().join("empty")).expect("mkdir");
    drizzle(dir.path())
        .args(["import", "empty"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No drizzle-kit snapshots found"));
}

// =============================================================================
// Dialects and versions
// =============================================================================

#[test]
fn dialect_comes_from_the_snapshot_or_the_flag() {
    // drizzle-kit 0.x writes `postgresql`, 1.x writes `postgres`.
    for rel in ["stable/postgres", "beta/postgres"] {
        drizzle(&fixtures())
            .args(["import", rel, "--out", "-"])
            .assert()
            .success()
            .stderr(predicate::str::contains("Dialect: postgresql"));
    }

    // A SQLite snapshot can be imported for Turso.
    drizzle(&fixtures())
        .args(["import", "beta/sqlite", "--out", "-", "--dialect", "turso"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Dialect: turso"))
        .stdout(predicate::str::contains("#[SQLiteTable"));

    // ...but not as another database.
    drizzle(&fixtures())
        .args(["import", "beta/sqlite", "--out", "-", "--dialect", "mysql"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "is a sqlite snapshot; --dialect mysql does not match it",
        ));
}

#[test]
fn dialect_flag_covers_snapshots_without_one() {
    let dir = tempdir().expect("temp dir");
    let mut json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixtures().join("stable/sqlite/meta/0000_snapshot.json"))
            .expect("read"),
    )
    .expect("json");
    json.as_object_mut().expect("object").remove("dialect");
    let path = dir.path().join("snapshot.json");
    fs::write(&path, json.to_string()).expect("write");

    drizzle(dir.path())
        .args(["import", "snapshot.json", "--out", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "has no \"dialect\" field; pass --dialect",
        ));
    drizzle(dir.path())
        .args([
            "import",
            "snapshot.json",
            "--out",
            "-",
            "--dialect",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pub struct Users"));
}

#[test]
fn unsupported_dialects_and_versions_are_refused() {
    let dir = tempdir().expect("temp dir");
    let write = |name: &str, json: serde_json::Value| {
        fs::write(dir.path().join(name), json.to_string()).expect("write");
    };
    write(
        "singlestore.json",
        serde_json::json!({ "version": "1", "dialect": "singlestore", "tables": {} }),
    );
    write(
        "old.json",
        serde_json::json!({ "version": "3", "dialect": "sqlite", "tables": {} }),
    );
    write(
        "future.json",
        serde_json::json!({ "version": "99", "dialect": "postgresql", "ddl": [] }),
    );

    drizzle(dir.path())
        .args(["import", "singlestore.json", "--out", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is a \"singlestore\" snapshot"));
    drizzle(dir.path())
        .args(["import", "old.json", "--out", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("at version 3"))
        .stderr(predicate::str::contains("npx drizzle-kit up"));
    drizzle(dir.path())
        .args(["import", "future.json", "--out", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("at version 99"))
        .stderr(predicate::str::contains("update drizzle-cli"));
}

// =============================================================================
// Output file
// =============================================================================

#[test]
fn writes_src_schema_rs_without_a_config_and_refuses_to_overwrite() {
    let dir = tempdir().expect("temp dir");
    let fixture = fixtures().join("beta/sqlite");

    drizzle(dir.path())
        .arg("import")
        .arg(&fixture)
        .assert()
        .success()
        .stdout(predicate::str::contains("Output: src/schema.rs"))
        .stdout(predicate::str::contains("Found 6 table(s)"));
    let schema = dir.path().join("src/schema.rs");
    assert!(
        fs::read_to_string(&schema)
            .expect("schema written")
            .contains("pub struct AuditLog")
    );

    fs::write(&schema, "// mine\n").expect("overwrite");
    drizzle(dir.path())
        .arg("import")
        .arg(&fixture)
        .assert()
        .failure()
        .stderr(predicate::str::contains("src/schema.rs already exists"))
        .stderr(predicate::str::contains("--force"));
    assert_eq!(fs::read_to_string(&schema).expect("read"), "// mine\n");

    drizzle(dir.path())
        .arg("import")
        .arg(&fixture)
        .arg("--force")
        .assert()
        .success();
    assert!(
        fs::read_to_string(&schema)
            .expect("read")
            .contains("pub struct AuditLog")
    );
}

#[test]
fn config_supplies_the_schema_path_and_casing() {
    let dir = tempdir().expect("temp dir");
    fs::write(
        dir.path().join("drizzle.config.toml"),
        "dialect = \"sqlite\"\nschema = \"db/schema.rs\"\nout = \"./drizzle\"\n\n[introspect]\ncasing = \"camel\"\n",
    )
    .expect("write config");

    drizzle(dir.path())
        .arg("import")
        .arg(fixtures().join("beta/sqlite"))
        .assert()
        .success()
        // `out` in the config points elsewhere: say how to use the folder.
        .stdout(predicate::str::contains("Set `out = "));
    let schema = fs::read_to_string(dir.path().join("db/schema.rs")).expect("schema written");
    assert!(schema.contains("pub displayName: String"), "{schema}");

    // --out and --casing win over the config.
    drizzle(dir.path())
        .arg("import")
        .arg(fixtures().join("beta/sqlite"))
        .args(["--out", "other.rs", "--casing", "preserve"])
        .assert()
        .success();
    let schema = fs::read_to_string(dir.path().join("other.rs")).expect("schema written");
    assert!(schema.contains("pub display_name: String"), "{schema}");
    assert!(schema.contains("pub createdAt: i64"), "{schema}");
}

#[test]
fn schema_name_flag_names_the_schema_struct() {
    drizzle(&fixtures())
        .args([
            "import",
            "beta/sqlite",
            "--out",
            "-",
            "--schema-name",
            "AppSchema",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pub struct AppSchema {"));
}

// =============================================================================
// Migration history
// =============================================================================

#[test]
fn next_steps_cover_config_history_and_manual_ports() {
    let dir = tempdir().expect("temp dir");
    let output = drizzle(dir.path())
        .arg("import")
        .arg(fixtures().join("beta/postgres"))
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    for expected in [
        "Create drizzle.config.toml",
        "dialect = \"postgresql\"",
        "schema = \"src/schema.rs\"",
        "__drizzle_migrations",
        "drizzle generate",
        "relations()",
        "$type<T>()",
        "$defaultFn",
        "customType()",
    ] {
        assert!(
            stdout.contains(expected),
            "missing `{expected}` in:\n{stdout}"
        );
    }
    // The folder layout needs no conversion.
    assert!(!stdout.contains("drizzle up"), "{stdout}");
    assert!(!dir.path().join("drizzle.config.toml").exists());
}

#[test]
fn upgrade_flag_converts_the_journal_layout_in_place() {
    let dir = tempdir().expect("temp dir");
    let migrations = dir.path().join("drizzle");
    copy_dir(&fixtures().join("stable/mysql"), &migrations);

    drizzle(dir.path())
        .args(["import", "drizzle", "--upgrade"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Converted 2 migration(s)"))
        .stdout(predicate::str::contains("drizzle up").not());

    assert!(!migrations.join("meta").exists());
    // `drizzle up` names each folder `<journal time, UTC>_<tag without index>`.
    let folder = |tag: &str| {
        fs::read_dir(&migrations)
            .expect("read migrations")
            .map(|entry| entry.expect("entry").path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_suffix(tag))
                    .and_then(|stamp| stamp.strip_suffix('_'))
                    .is_some_and(|stamp| {
                        stamp.len() == 14 && stamp.bytes().all(|b| b.is_ascii_digit())
                    })
            })
            .unwrap_or_else(|| panic!("no converted folder for {tag}"))
    };
    assert!(folder("init").join("migration.sql").exists());
    assert!(folder("full").join("snapshot.json").exists());
    assert!(dir.path().join("src/schema.rs").exists());

    // `--upgrade` cannot share stdout with the schema.
    drizzle(dir.path())
        .args(["import", "drizzle", "--upgrade", "--out", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--upgrade cannot be combined"));
}

#[test]
fn init_config_writes_a_config_for_the_imported_project() {
    let dir = tempdir().expect("temp dir");
    copy_dir(&fixtures().join("beta/sqlite"), &dir.path().join("drizzle"));

    drizzle(dir.path())
        .args(["import", "drizzle", "--init-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created drizzle.config.toml"));
    let config = fs::read_to_string(dir.path().join("drizzle.config.toml")).expect("config");
    assert!(config.contains("dialect = \"sqlite\""), "{config}");
    assert!(config.contains("schema = \"src/schema.rs\""), "{config}");
    assert!(config.contains("out = \"drizzle\""), "{config}");

    // The written config is valid: `drizzle status` reads the history.
    drizzle(dir.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("_full"));

    // An existing config is left alone.
    drizzle(dir.path())
        .args(["import", "drizzle", "--init-config", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a config already exists"));
}

/// The whole adoption path: import, point the config at the existing
/// folder (converting the journal layout), and `drizzle generate` reads
/// drizzle-kit's last snapshot as the previous state and finds nothing to do.
#[test]
fn generate_after_import_finds_no_changes() {
    for rel in [
        "stable/sqlite",
        "beta/sqlite",
        "stable/postgres",
        "beta/postgres",
        "stable/mysql",
        "beta/mysql",
    ] {
        let dir = tempdir().expect("temp dir");
        copy_dir(&fixtures().join(rel), &dir.path().join("drizzle"));

        drizzle(dir.path())
            .args(["import", "drizzle", "--init-config", "--upgrade"])
            .assert()
            .success();
        drizzle(dir.path())
            .arg("generate")
            .assert()
            .success()
            .stdout(predicate::str::contains("No schema changes"));
        let migrations = fs::read_dir(dir.path().join("drizzle"))
            .expect("read migrations")
            .count();
        assert_eq!(migrations, 2, "{rel}: generate must not add a migration");
    }
}

/// A database drizzle-orm 0.x migrated: its migrator ran each `NNNN_tag.sql`
/// and recorded `sha256(file)` with the journal's `when` in
/// `__drizzle_migrations`. After `drizzle import --upgrade`, `drizzle migrate`
/// must see both migrations as applied and run nothing.
#[cfg(feature = "rusqlite")]
#[test]
fn migrate_keeps_drizzle_orm_history() {
    use sha2::{Digest, Sha256};

    let dir = tempdir().expect("temp dir");
    let migrations = dir.path().join("drizzle");
    copy_dir(&fixtures().join("stable/sqlite"), &migrations);

    let journal: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(migrations.join("meta/_journal.json")).expect("journal"),
    )
    .expect("json");
    let conn = rusqlite::Connection::open(dir.path().join("dev.db")).expect("open db");
    // drizzle-orm's SQLite migrator table.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS \"__drizzle_migrations\" (id SERIAL PRIMARY KEY, hash text NOT NULL, created_at numeric)",
        [],
    )
    .expect("tracking table");
    for (idx, entry) in journal["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .enumerate()
    {
        let tag = entry["tag"].as_str().expect("tag");
        let sql = fs::read_to_string(migrations.join(format!("{tag}.sql"))).expect("sql");
        // Only the tracking rows matter here. (drizzle-kit 0.31's rebuild of
        // `users` in 0001 copies the generated column, which SQLite rejects.)
        if idx == 0 {
            for statement in sql.split("--> statement-breakpoint") {
                conn.execute_batch(statement).expect("apply migration");
            }
        }
        let hash = Sha256::digest(sql.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        conn.execute(
            "INSERT INTO \"__drizzle_migrations\" (\"hash\", \"created_at\") VALUES (?1, ?2)",
            rusqlite::params![hash, entry["when"].as_i64().expect("when")],
        )
        .expect("record migration");
    }
    drop(conn);

    drizzle(dir.path())
        .args(["import", "drizzle", "--upgrade", "--init-config"])
        .assert()
        .success();
    drizzle(dir.path())
        .args(["migrate", "--plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Applied migrations: 2"))
        .stdout(predicate::str::contains("Pending migrations: 0"));
    drizzle(dir.path())
        .arg("migrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("No pending migrations."));

    let conn = rusqlite::Connection::open(dir.path().join("dev.db")).expect("reopen db");
    let recorded: i64 = conn
        .query_row("SELECT COUNT(*) FROM \"__drizzle_migrations\"", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(recorded, 2, "no migration may run twice");
}
