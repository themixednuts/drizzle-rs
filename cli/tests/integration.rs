//! End-to-end integration tests for drizzle CLI
//!
//! These tests verify the actual behavior of the CLI, not just that commands run.
//! Each test validates:
//! - Exit codes
//! - Output messages
//! - File contents (not just existence)
//! - Error handling

use assert_cmd::cargo;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

/// Get the drizzle CLI binary
fn drizzle_cli() -> Command {
    Command::new(cargo::cargo_bin!("drizzle"))
}

/// Helper to create a valid SQLite snapshot JSON
fn sqlite_snapshot(id: &str, prev_id: &str, tables: &str) -> String {
    format!(
        r#"{{
            "version": "6",
            "dialect": "sqlite",
            "id": "{}",
            "prevId": "{}",
            "tables": {{{}}},
            "enums": {{}},
            "_meta": {{ "tables": {{}}, "columns": {{}} }}
        }}"#,
        id, prev_id, tables
    )
}

/// Helper to create a valid journal entry
fn journal_entry(idx: u32, tag: &str) -> String {
    format!(
        r#"{{
            "idx": {},
            "version": "6",
            "when": 1700000000000,
            "tag": "{}",
            "breakpoints": true
        }}"#,
        idx, tag
    )
}

/// Helper to create a complete journal
fn journal(dialect: &str, entries: &[String]) -> String {
    format!(
        r#"{{
            "version": "7",
            "dialect": "{}",
            "entries": [{}]
        }}"#,
        dialect,
        entries.join(",")
    )
}

// =============================================================================
// init command tests
// =============================================================================
mod init {
    use super::*;

    #[test]
    fn creates_sqlite_config() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created drizzle.toml"));

        // Verify file exists and has correct content
        assert!(config_path.exists(), "Config file should be created");

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("dialect = \"sqlite\""),
            "Should have sqlite dialect"
        );
        assert!(
            content.contains("out = \"./drizzle\""),
            "Should have default out path"
        );
        assert!(
            content.contains("breakpoints = true"),
            "Should have breakpoints enabled"
        );
    }

    #[test]
    fn creates_postgresql_config() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .arg("--dialect=postgresql")
            .assert()
            .success();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("dialect = \"postgresql\""),
            "Should have postgresql dialect"
        );
    }

    #[test]
    fn creates_mysql_config() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .arg("--dialect=mysql")
            .assert()
            .success();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("dialect = \"mysql\""),
            "Should have mysql dialect"
        );
    }

    #[test]
    fn fails_if_config_exists() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");
        std::fs::write(&config_path, "dialect = \"sqlite\"\nout = \"./drizzle\"").unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
    }

    #[test]
    fn fails_on_invalid_dialect() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .arg("--dialect=invalid")
            .assert()
            .failure();
    }
}

// =============================================================================
// status command tests
// =============================================================================
mod status {
    use super::*;

    #[test]
    fn shows_no_migrations_directory() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("status")
            .arg("--out=drizzle")
            .assert()
            .success()
            .stdout(predicate::str::contains("No migrations directory"));
    }

    #[test]
    fn shows_empty_journal() {
        let temp = TempDir::new().unwrap();
        let meta_dir = temp.path().join("drizzle").join("migrations").join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(meta_dir.join("_journal.json"), journal("sqlite", &[])).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("status")
            .arg("--out=drizzle")
            .assert()
            .success()
            .stdout(predicate::str::contains("No migrations yet"));
    }

    #[test]
    fn shows_single_migration() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test_migration")]),
        )
        .unwrap();

        std::fs::write(
            migrations_dir.join("0000_test_migration.sql"),
            "CREATE TABLE test (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("status")
            .arg("--out=drizzle")
            .assert()
            .success()
            .stdout(predicate::str::contains("0000_test_migration"))
            .stdout(predicate::str::contains("sqlite"));
    }

    #[test]
    fn shows_multiple_migrations() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal(
                "sqlite",
                &[
                    journal_entry(0, "0000_first"),
                    journal_entry(1, "0001_second"),
                    journal_entry(2, "0002_third"),
                ],
            ),
        )
        .unwrap();

        for name in &["0000_first", "0001_second", "0002_third"] {
            std::fs::write(migrations_dir.join(format!("{}.sql", name)), "-- migration").unwrap();
        }

        drizzle_cli()
            .current_dir(temp.path())
            .arg("status")
            .arg("--out=drizzle")
            .assert()
            .success()
            .stdout(predicate::str::contains("0000_first"))
            .stdout(predicate::str::contains("0001_second"))
            .stdout(predicate::str::contains("0002_third"))
            .stdout(predicate::str::contains("Migrations (3)"));
    }
}

// =============================================================================
// check command tests
// =============================================================================
mod check {
    use super::*;

    #[test]
    fn valid_single_migration() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test")]),
        )
        .unwrap();

        std::fs::write(
            migrations_dir.join("0000_test.sql"),
            "CREATE TABLE test (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        std::fs::write(
            meta_dir.join("0000_snapshot.json"),
            sqlite_snapshot(
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000000",
                "",
            ),
        )
        .unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("Everything's fine"));
    }

    #[test]
    fn valid_multiple_migrations() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal(
                "sqlite",
                &[
                    journal_entry(0, "0000_first"),
                    journal_entry(1, "0001_second"),
                ],
            ),
        )
        .unwrap();

        // SQL files
        std::fs::write(
            migrations_dir.join("0000_first.sql"),
            "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        )
        .unwrap();
        std::fs::write(
            migrations_dir.join("0001_second.sql"),
            "CREATE TABLE posts (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        // Snapshots with proper prev_id chain
        std::fs::write(
            meta_dir.join("0000_snapshot.json"),
            sqlite_snapshot(
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000000",
                "",
            ),
        )
        .unwrap();
        std::fs::write(
            meta_dir.join("0001_snapshot.json"),
            sqlite_snapshot(
                "00000000-0000-0000-0000-000000000002",
                "00000000-0000-0000-0000-000000000001",
                "",
            ),
        )
        .unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("0000_first"))
            .stdout(predicate::str::contains("0001_second"))
            .stdout(predicate::str::contains("Everything's fine"));
    }

    #[test]
    fn missing_sql_file() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_missing")]),
        )
        .unwrap();

        // No SQL file created

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stdout(predicate::str::contains("Missing SQL file"));
    }

    #[test]
    fn missing_snapshot() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test")]),
        )
        .unwrap();

        std::fs::write(
            migrations_dir.join("0000_test.sql"),
            "CREATE TABLE test (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        // No snapshot created

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stdout(predicate::str::contains("Missing snapshot"));
    }

    #[test]
    fn malformed_snapshot() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test")]),
        )
        .unwrap();

        std::fs::write(
            migrations_dir.join("0000_test.sql"),
            "CREATE TABLE test (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        // Invalid JSON snapshot
        std::fs::write(meta_dir.join("0000_snapshot.json"), "not valid json").unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stdout(predicate::str::contains("Malformed snapshot"));
    }

    #[test]
    fn empty_sql_file_is_warning() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test")]),
        )
        .unwrap();

        // Empty SQL file (just whitespace)
        std::fs::write(migrations_dir.join("0000_test.sql"), "   \n  ").unwrap();

        std::fs::write(
            meta_dir.join("0000_snapshot.json"),
            sqlite_snapshot(
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000000",
                "",
            ),
        )
        .unwrap();

        // Should succeed but warn
        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("Empty SQL file"))
            .stdout(predicate::str::contains("Everything's fine"));
    }

    #[test]
    fn snapshot_collision_detected() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal(
                "sqlite",
                &[
                    journal_entry(0, "0000_first"),
                    journal_entry(1, "0001_second"),
                ],
            ),
        )
        .unwrap();

        std::fs::write(migrations_dir.join("0000_first.sql"), "CREATE TABLE a;").unwrap();
        std::fs::write(migrations_dir.join("0001_second.sql"), "CREATE TABLE b;").unwrap();

        // Both snapshots have the SAME prev_id = collision!
        std::fs::write(
            meta_dir.join("0000_snapshot.json"),
            sqlite_snapshot(
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000000", // Same prev_id
                "",
            ),
        )
        .unwrap();
        std::fs::write(
            meta_dir.join("0001_snapshot.json"),
            sqlite_snapshot(
                "00000000-0000-0000-0000-000000000002",
                "00000000-0000-0000-0000-000000000000", // Same prev_id = COLLISION
                "",
            ),
        )
        .unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stdout(predicate::str::contains("Collision"));
    }
}

// =============================================================================
// generate command tests
// =============================================================================
mod generate {
    use super::*;

    #[test]
    fn custom_migration_with_name() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--custom")
            .arg("--name=initial_setup")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created custom migration"))
            .stdout(predicate::str::contains("initial_setup"));

        // Verify files
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");

        // Journal should exist
        let journal_path = meta_dir.join("_journal.json");
        assert!(journal_path.exists(), "Journal should be created");

        let journal_content = std::fs::read_to_string(&journal_path).unwrap();
        assert!(
            journal_content.contains("initial_setup"),
            "Journal should contain migration name"
        );
        assert!(
            journal_content.contains("\"idx\": 0"),
            "Should be first migration"
        );

        // SQL file should exist with correct name pattern
        let sql_file = migrations_dir.join("0000_initial_setup.sql");
        assert!(sql_file.exists(), "SQL file should be created");

        let sql_content = std::fs::read_to_string(&sql_file).unwrap();
        assert!(
            sql_content.contains("Custom migration"),
            "SQL should have placeholder content"
        );

        // Snapshot should exist
        let snapshot_file = meta_dir.join("0000_snapshot.json");
        assert!(snapshot_file.exists(), "Snapshot should be created");
    }

    #[test]
    fn custom_migration_without_name_generates_random() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--custom")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created custom migration"));

        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let sql_files: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .collect();

        assert_eq!(sql_files.len(), 1, "Should have one SQL file");
        let name = sql_files[0].file_name().to_string_lossy().to_string();
        assert!(name.starts_with("0000_"), "Should start with index prefix");
    }

    #[test]
    fn requires_schema_for_non_custom() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Schema path required"));
    }

    #[test]
    fn generate_from_schema_creates_table() {
        let temp = TempDir::new().unwrap();

        // Create a schema with a users table
        let schema = r#"{
            "version": "6",
            "dialect": "sqlite",
            "id": "test-schema-id",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {
                "users": {
                    "name": "users",
                    "columns": {
                        "id": {
                            "name": "id",
                            "type": "integer",
                            "primaryKey": true,
                            "notNull": true,
                            "autoincrement": true
                        },
                        "name": {
                            "name": "name",
                            "type": "text",
                            "primaryKey": false,
                            "notNull": true
                        },
                        "email": {
                            "name": "email",
                            "type": "text",
                            "primaryKey": false,
                            "notNull": false
                        }
                    },
                    "indexes": {},
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {},
                    "checkConstraints": {}
                }
            },
            "enums": {},
            "_meta": { "tables": {}, "columns": {} }
        }"#;

        std::fs::write(temp.path().join("schema.json"), schema).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created migration"))
            .stdout(predicate::str::contains("table users"));

        // Verify SQL content
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let sql_files: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .collect();

        let sql_content = std::fs::read_to_string(sql_files[0].path()).unwrap();
        assert!(
            sql_content.contains("CREATE TABLE"),
            "Should have CREATE TABLE"
        );
        assert!(sql_content.contains("users"), "Should reference users table");
        assert!(sql_content.contains("`id`"), "Should have id column");
        assert!(sql_content.contains("`name`"), "Should have name column");
        assert!(sql_content.contains("`email`"), "Should have email column");
        assert!(
            sql_content.contains("PRIMARY KEY"),
            "Should have primary key"
        );
    }

    #[test]
    fn no_changes_detected() {
        let temp = TempDir::new().unwrap();

        // Create a schema
        let schema = sqlite_snapshot(
            "test-id",
            "00000000-0000-0000-0000-000000000000",
            r#"
                "users": {
                    "name": "users",
                    "columns": {},
                    "indexes": {},
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {},
                    "checkConstraints": {}
                }
            "#,
        );

        std::fs::write(temp.path().join("schema.json"), &schema).unwrap();

        // First generation
        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .success();

        // Second generation with same schema - should report no changes
        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .success()
            .stdout(predicate::str::contains("No schema changes"));
    }

    #[test]
    fn multiple_consecutive_migrations() {
        let temp = TempDir::new().unwrap();

        // First schema: users table
        let schema1 = r#"{
            "version": "6",
            "dialect": "sqlite",
            "id": "schema-1",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {
                "users": {
                    "name": "users",
                    "columns": {
                        "id": { "name": "id", "type": "integer", "primaryKey": true, "notNull": true }
                    },
                    "indexes": {}, "foreignKeys": {}, "compositePrimaryKeys": {},
                    "uniqueConstraints": {}, "checkConstraints": {}
                }
            },
            "enums": {},
            "_meta": { "tables": {}, "columns": {} }
        }"#;

        std::fs::write(temp.path().join("schema.json"), schema1).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .success();

        // Second schema: users + posts tables
        let schema2 = r#"{
            "version": "6",
            "dialect": "sqlite",
            "id": "schema-2",
            "prevId": "schema-1",
            "tables": {
                "users": {
                    "name": "users",
                    "columns": {
                        "id": { "name": "id", "type": "integer", "primaryKey": true, "notNull": true }
                    },
                    "indexes": {}, "foreignKeys": {}, "compositePrimaryKeys": {},
                    "uniqueConstraints": {}, "checkConstraints": {}
                },
                "posts": {
                    "name": "posts",
                    "columns": {
                        "id": { "name": "id", "type": "integer", "primaryKey": true, "notNull": true }
                    },
                    "indexes": {}, "foreignKeys": {}, "compositePrimaryKeys": {},
                    "uniqueConstraints": {}, "checkConstraints": {}
                }
            },
            "enums": {},
            "_meta": { "tables": {}, "columns": {} }
        }"#;

        std::fs::write(temp.path().join("schema.json"), schema2).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .success()
            .stdout(predicate::str::contains("posts"));

        // Verify we have 2 migrations
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let sql_files: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .collect();

        assert_eq!(sql_files.len(), 2, "Should have 2 migrations");
    }

    #[test]
    fn breakpoints_disabled() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--custom")
            .arg("--name=test")
            .arg("--breakpoints=false")
            .assert()
            .success();

        // Verify journal entry has breakpoints: false
        let journal_path = temp
            .path()
            .join("drizzle")
            .join("migrations")
            .join("meta")
            .join("_journal.json");
        let journal_content = std::fs::read_to_string(journal_path).unwrap();
        assert!(
            journal_content.contains("\"breakpoints\": false")
                || journal_content.contains("\"breakpoints\":false"),
            "Journal should have breakpoints false"
        );
    }

    #[test]
    fn invalid_schema_file() {
        let temp = TempDir::new().unwrap();

        std::fs::write(temp.path().join("schema.json"), "not valid json").unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .failure();
    }

    #[test]
    fn nonexistent_schema_file() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=nonexistent.json")
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found"));
    }
}

// =============================================================================
// help and version tests
// =============================================================================
mod help {
    use super::*;

    #[test]
    fn help_shows_all_commands() {
        drizzle_cli()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("generate"))
            .stdout(predicate::str::contains("check"))
            .stdout(predicate::str::contains("drop"))
            .stdout(predicate::str::contains("status"))
            .stdout(predicate::str::contains("init"))
            .stdout(predicate::str::contains("up"));
    }

    #[test]
    fn generate_help_shows_options() {
        drizzle_cli()
            .arg("generate")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--schema"))
            .stdout(predicate::str::contains("--out"))
            .stdout(predicate::str::contains("--dialect"))
            .stdout(predicate::str::contains("--name"))
            .stdout(predicate::str::contains("--custom"))
            .stdout(predicate::str::contains("--prefix"))
            .stdout(predicate::str::contains("--breakpoints"));
    }

    #[test]
    fn check_help_shows_options() {
        drizzle_cli()
            .arg("check")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--out"))
            .stdout(predicate::str::contains("--dialect"));
    }

    #[test]
    fn version_flag() {
        drizzle_cli()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains("drizzle"));
    }
}

// =============================================================================
// up command tests
// =============================================================================
mod up {
    use super::*;

    #[test]
    fn up_no_migrations() {
        let temp = TempDir::new().unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("up")
            .arg("--out=drizzle")
            .assert()
            .success()
            .stdout(predicate::str::contains("No migrations directory"));
    }

    #[test]
    fn up_already_current() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test")]),
        )
        .unwrap();

        std::fs::write(
            migrations_dir.join("0000_test.sql"),
            "CREATE TABLE test (id INTEGER);",
        )
        .unwrap();

        // Snapshot with current version (v6)
        std::fs::write(
            meta_dir.join("0000_snapshot.json"),
            sqlite_snapshot(
                "test-id",
                "00000000-0000-0000-0000-000000000000",
                "",
            ),
        )
        .unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("up")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("already at latest version"));
    }

    #[test]
    fn up_upgrades_old_version() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        std::fs::write(
            meta_dir.join("_journal.json"),
            journal("sqlite", &[journal_entry(0, "0000_test")]),
        )
        .unwrap();

        std::fs::write(
            migrations_dir.join("0000_test.sql"),
            "CREATE TABLE test (id INTEGER);",
        )
        .unwrap();

        // Snapshot with OLD version (v5)
        let old_snapshot = r#"{
            "version": "5",
            "dialect": "sqlite",
            "id": "test-id",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {},
            "enums": {},
            "_meta": { "tables": {}, "columns": {} }
        }"#;
        std::fs::write(meta_dir.join("0000_snapshot.json"), old_snapshot).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("up")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("Upgraded 1 snapshot"));

        // Verify the snapshot was upgraded
        let content = std::fs::read_to_string(meta_dir.join("0000_snapshot.json")).unwrap();
        assert!(
            content.contains("\"version\": \"6\""),
            "Snapshot should be upgraded to version 6"
        );
    }
}

// =============================================================================
// error handling tests
// =============================================================================
mod errors {
    use super::*;

    #[test]
    fn invalid_command() {
        drizzle_cli()
            .arg("invalid_command")
            .assert()
            .failure()
            .stderr(predicate::str::contains("error"));
    }

    #[test]
    fn check_without_journal() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        std::fs::create_dir_all(&migrations_dir).unwrap();
        // No journal file

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Journal"));
    }
}
