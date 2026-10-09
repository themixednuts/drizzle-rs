//! Rename-or-create decisions without a terminal: drizzle-kit's hints mode.
//!
//! Tests run with stdin not a terminal, so `generate` and `push` never
//! prompt: ambiguous renames need `--hints` / `--hints-file`, and without
//! them the command lists the decisions and exits with code 2.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const USERS: &str = r"
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Users {
  #[column(primary)]
  pub id: i64,
  pub name: String,
}
";

const ACCOUNTS: &str = r"
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Accounts {
  #[column(primary)]
  pub id: i64,
  pub full_name: String,
}
";

const RENAME_TABLE_AND_COLUMN: &str = r#"[
  {"type": "rename", "kind": "table", "from": ["public", "users"], "to": ["public", "accounts"]},
  {"type": "rename", "kind": "column", "from": ["public", "accounts", "name"], "to": ["public", "accounts", "full_name"]}
]"#;

fn write_project(root: &Path, schema: &str) {
    fs::write(root.join("schema.rs"), schema).expect("write schema");
    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            "dialect = \"sqlite\"\nschema = '{}'\nout = '{}'\n\n[migrations]\nprefix = \"index\"\n\n[dbCredentials]\nurl = '{}'\n",
            root.join("schema.rs").to_string_lossy(),
            root.join("migrations").to_string_lossy(),
            root.join("dev.db").to_string_lossy(),
        ),
    )
    .expect("write config");
}

fn migrations(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("migrations"))
        .expect("read migrations")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("migration.sql").is_file())
        .collect();
    dirs.sort();
    dirs
}

fn latest_sql(root: &Path) -> String {
    let dir = migrations(root).pop().expect("a migration");
    fs::read_to_string(dir.join("migration.sql")).expect("read migration.sql")
}

/// A project whose first migration created `users`, with the schema now
/// declaring `accounts` instead.
fn renamed_project() -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    write_project(dir.path(), USERS);
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args(["generate", "--name", "init"])
        .assert()
        .success();
    fs::write(dir.path().join("schema.rs"), ACCOUNTS).expect("rewrite schema");
    dir
}

#[test]
fn generate_without_hints_lists_the_unresolved_renames_and_exits_2() {
    let dir = renamed_project();
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args(["generate", "--name", "renamed"])
        .assert()
        .code(2)
        .stdout(
            contains("missing_hints: 1 unresolved decisions")
                .and(contains("1. Rename or create"))
                .and(contains("table"))
                .and(contains("public.accounts"))
                .and(contains("Deleted candidates: [\"public\", \"users\"]"))
                .and(contains(
                    r#"{ "type": "rename", "kind": "table", "from": ["<schema>", "<old_name>"], "to": ["public", "accounts"] }"#,
                ))
                .and(contains(
                    r#"{ "type": "create", "kind": "table", "entity": ["public", "accounts"] }"#,
                ))
                .and(contains("Re-run with --hints")),
        );
    assert_eq!(migrations(dir.path()).len(), 1, "nothing was written");
}

#[test]
fn generate_lists_column_questions_after_a_hinted_table_rename() {
    let dir = renamed_project();
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args([
            "generate",
            "--hints",
            r#"[{"type":"rename","kind":"table","from":["public","users"],"to":["public","accounts"]}]"#,
        ])
        .assert()
        .code(2)
        .stdout(
            contains("missing_hints: 1 unresolved decisions")
                .and(contains("column"))
                .and(contains("public.accounts.full_name"))
                .and(contains(r#""entity": ["public", "accounts", "full_name"]"#)),
        );
}

#[test]
fn generate_with_rename_hints_writes_renames() {
    let dir = renamed_project();
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args([
            "generate",
            "--name",
            "renamed",
            "--hints",
            RENAME_TABLE_AND_COLUMN,
        ])
        .assert()
        .success()
        .stdout(contains("Migration generated"));
    assert_eq!(
        latest_sql(dir.path()),
        "ALTER TABLE `users` RENAME TO `accounts`;\n--> statement-breakpoint\nALTER TABLE `accounts` RENAME COLUMN `name` TO `full_name`;"
    );
}

#[test]
fn generate_with_create_hints_from_a_file_drops_and_creates() {
    let dir = renamed_project();
    let hints = dir.path().join("hints.json");
    fs::write(
        &hints,
        r#"[{"type": "create", "kind": "table", "entity": ["public", "accounts"]}]"#,
    )
    .expect("write hints");
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args(["generate", "--name", "recreated", "--hints-file"])
        .arg(&hints)
        .assert()
        .success();
    let sql = latest_sql(dir.path());
    assert!(sql.contains("CREATE TABLE `accounts`"), "{sql}");
    assert!(sql.contains("DROP TABLE `users`"), "{sql}");
    assert!(!sql.contains("RENAME"), "{sql}");
}

#[test]
fn generate_rejects_a_rename_hint_from_a_table_that_was_not_deleted() {
    let dir = renamed_project();
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args([
            "generate",
            "--hints",
            r#"[{"type":"rename","kind":"table","from":["public","people"],"to":["public","accounts"]}]"#,
        ])
        .assert()
        .code(1)
        .stderr(contains(
            "Invalid hints: rename hint's `from` [\"public\", \"people\"] doesn't match any deleted table",
        ));
    assert_eq!(migrations(dir.path()).len(), 1);
}

#[test]
fn generate_rejects_malformed_hints() {
    let dir = renamed_project();
    cargo_bin_cmd!("drizzle")
        .current_dir(dir.path())
        .args([
            "generate",
            "--hints",
            r#"[{"type":"rename","kind":"table"}]"#,
        ])
        .assert()
        .code(1)
        .stderr(contains("Invalid hints: Invalid hint shape at [0].from"));
}

#[test]
fn generate_uses_drizzle_kit_ids_for_postgres_and_mysql() {
    let cases = [
        (
            "postgresql",
            "#[PostgresTable(schema = \"app\")]",
            r#"["app", "accounts"]"#,
            r#"[{"type":"rename","kind":"table","from":["app","users"],"to":["app","accounts"]}]"#,
            "ALTER TABLE \"app\".\"users\" RENAME TO \"accounts\";",
        ),
        (
            "mysql",
            "#[MySQLTable]",
            r#"["public", "accounts"]"#,
            r#"[{"type":"rename","kind":"table","from":["public","users"],"to":["public","accounts"]}]"#,
            "RENAME TABLE `users` TO `accounts`;",
        ),
    ];
    for (dialect, attribute, id, hints, expected) in cases {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let schema = |name: &str| {
            format!("{attribute}\npub struct {name} {{\n  #[column(primary)]\n  pub id: i32,\n}}\n")
        };
        fs::write(root.join("schema.rs"), schema("Users")).expect("write schema");
        fs::write(
            root.join("drizzle.config.toml"),
            format!(
                "dialect = \"{dialect}\"\nschema = '{}'\nout = '{}'\n\n[migrations]\nprefix = \"index\"\n",
                root.join("schema.rs").to_string_lossy(),
                root.join("migrations").to_string_lossy(),
            ),
        )
        .expect("write config");
        cargo_bin_cmd!("drizzle")
            .current_dir(root)
            .args(["generate", "--name", "init"])
            .assert()
            .success();
        fs::write(root.join("schema.rs"), schema("Accounts")).expect("rewrite schema");

        cargo_bin_cmd!("drizzle")
            .current_dir(root)
            .arg("generate")
            .assert()
            .code(2)
            .stdout(contains(format!(
                r#"{{ "type": "create", "kind": "table", "entity": {id} }}"#
            )));
        cargo_bin_cmd!("drizzle")
            .current_dir(root)
            .args(["generate", "--name", "renamed", "--hints", hints])
            .assert()
            .success();
        // Only the table is renamed: on PostgreSQL the primary key keeps
        // its implicit name (`users_pkey`), as in drizzle-kit.
        let sql = latest_sql(root);
        assert_eq!(sql.trim(), expected, "{dialect}");
        // The snapshot records the kept name, so nothing is left to do.
        cargo_bin_cmd!("drizzle")
            .current_dir(root)
            .arg("generate")
            .assert()
            .success()
            .stdout(contains("No schema changes"));
    }
}

/// A renamed, explicitly named PostgreSQL constraint is asked about like
/// drizzle-kit does, and a `unique` hint renames it in place.
#[test]
fn generate_renames_an_explicitly_named_postgres_constraint() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let schema = |name: &str| {
        format!(
            "#[PostgresTable(unique(columns(email), name = \"{name}\"))]\npub struct Users {{\n  #[column(primary)]\n  pub id: i32,\n  pub email: String,\n}}\n"
        )
    };
    fs::write(root.join("schema.rs"), schema("users_email_uq")).expect("write schema");
    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            "dialect = \"postgresql\"\nschema = '{}'\nout = '{}'\n\n[migrations]\nprefix = \"index\"\n",
            root.join("schema.rs").to_string_lossy(),
            root.join("migrations").to_string_lossy(),
        ),
    )
    .expect("write config");
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--name", "init"])
        .assert()
        .success();
    fs::write(root.join("schema.rs"), schema("users_email_key")).expect("rewrite schema");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "generate",
            "--name",
            "renamed",
            "--hints",
            r#"[{"type":"rename","kind":"unique","from":["public","users","users_email_uq"],"to":["public","users","users_email_key"]}]"#,
        ])
        .assert()
        .success();
    assert_eq!(
        latest_sql(root).trim(),
        "ALTER TABLE \"users\" RENAME CONSTRAINT \"users_email_uq\" TO \"users_email_key\";"
    );

    // A constraint holds no data, so without a hint it is dropped and
    // created rather than stopping the command.
    fs::write(root.join("schema.rs"), schema("users_email_unique")).expect("rewrite schema");
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["generate", "--name", "recreated"])
        .assert()
        .success();
    let sql = latest_sql(root);
    assert!(sql.contains("DROP CONSTRAINT \"users_email_key\""), "{sql}");
    assert!(
        sql.contains("ADD CONSTRAINT \"users_email_unique\""),
        "{sql}"
    );
}

#[cfg(feature = "rusqlite")]
mod push {
    use super::*;

    fn seeded_project() -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        write_project(dir.path(), ACCOUNTS);
        let conn = rusqlite::Connection::open(dir.path().join("dev.db")).expect("open db");
        conn.execute_batch(
            "CREATE TABLE `users` (`id` integer PRIMARY KEY NOT NULL, `name` text NOT NULL);
             INSERT INTO `users` VALUES (1, 'Ada');",
        )
        .expect("seed db");
        dir
    }

    fn tables(root: &Path) -> Vec<String> {
        let conn = rusqlite::Connection::open(root.join("dev.db")).expect("open db");
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("prepare");
        stmt.query_map([], |row| row.get(0))
            .expect("query")
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn push_without_hints_exits_2_even_with_force() {
        let dir = seeded_project();
        for args in [&["push"][..], &["push", "--force"], &["push", "--explain"]] {
            cargo_bin_cmd!("drizzle")
                .current_dir(dir.path())
                .args(args)
                .assert()
                .code(2)
                .stdout(contains("missing_hints: 1 unresolved decisions").and(contains(
                    r#"{ "type": "create", "kind": "table", "entity": ["public", "accounts"] }"#,
                )));
        }
        assert_eq!(tables(dir.path()), ["users"]);
    }

    #[test]
    fn push_explain_shows_hinted_renames() {
        let dir = seeded_project();
        cargo_bin_cmd!("drizzle")
            .current_dir(dir.path())
            .args(["push", "--explain", "--hints", RENAME_TABLE_AND_COLUMN])
            .assert()
            .success()
            .stdout(
                contains("--- Planned SQL ---")
                    .and(contains("ALTER TABLE `users` RENAME TO `accounts`;"))
                    .and(contains(
                        "ALTER TABLE `accounts` RENAME COLUMN `name` TO `full_name`;",
                    ))
                    .and(contains("DROP TABLE").not()),
            );
        assert_eq!(tables(dir.path()), ["users"]);
    }

    #[test]
    fn push_with_rename_hints_keeps_the_data() {
        let dir = seeded_project();
        cargo_bin_cmd!("drizzle")
            .current_dir(dir.path())
            .args(["push", "--hints", RENAME_TABLE_AND_COLUMN])
            .assert()
            .success()
            .stdout(contains("Push complete!"));
        assert_eq!(tables(dir.path()), ["accounts"]);
        let conn = rusqlite::Connection::open(dir.path().join("dev.db")).expect("open db");
        let name: String = conn
            .query_row("SELECT full_name FROM accounts WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("row survived the rename");
        assert_eq!(name, "Ada");
    }

    #[test]
    fn push_with_create_hints_and_force_drops_and_creates() {
        let dir = seeded_project();
        cargo_bin_cmd!("drizzle")
            .current_dir(dir.path())
            .args([
                "push",
                "--force",
                "--hints",
                r#"[{"type":"create","kind":"table","entity":["public","accounts"]}]"#,
            ])
            .assert()
            .success();
        assert_eq!(tables(dir.path()), ["accounts"]);
    }
}
