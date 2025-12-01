//! End-to-end integration tests for drizzle CLI

use assert_cmd::cargo;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

/// Get the drizzle CLI binary
fn drizzle_cli() -> Command {
    Command::new(cargo::cargo_bin!("drizzle"))
}

mod init {
    use super::*;

    #[test]
    fn init_creates_config_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .arg("--dialect=sqlite")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created drizzle.toml"));

        assert!(config_path.exists());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("dialect = \"sqlite\""));
        assert!(content.contains("out = \"./drizzle\""));
    }

    #[test]
    fn init_fails_if_config_exists() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");
        // Create a valid TOML file
        std::fs::write(&config_path, "dialect = \"sqlite\"\nout = \"./drizzle\"").unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
    }

    #[test]
    fn init_postgresql() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("drizzle.toml");

        drizzle_cli()
            .current_dir(temp.path())
            .arg("init")
            .arg("--dialect=postgresql")
            .assert()
            .success();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("dialect = \"postgresql\""));
    }
}

mod status {
    use super::*;

    #[test]
    fn status_shows_no_migrations() {
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
    fn status_shows_migrations() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        // Create a journal with one entry
        let journal = r#"{
            "version": "7",
            "dialect": "sqlite",
            "entries": [
                {
                    "idx": 0,
                    "version": "6",
                    "when": 1700000000000,
                    "tag": "0000_test_migration",
                    "breakpoints": true
                }
            ]
        }"#;
        std::fs::write(meta_dir.join("_journal.json"), journal).unwrap();

        // Create the SQL file
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
            .stdout(predicate::str::contains("0000_test_migration"));
    }
}

mod check {
    use super::*;

    #[test]
    fn check_valid_migrations() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        // Create a valid journal
        let journal = r#"{
            "version": "7",
            "dialect": "sqlite",
            "entries": [
                {
                    "idx": 0,
                    "version": "6",
                    "when": 1700000000000,
                    "tag": "0000_test_migration",
                    "breakpoints": true
                }
            ]
        }"#;
        std::fs::write(meta_dir.join("_journal.json"), journal).unwrap();

        // Create the SQL file
        std::fs::write(
            migrations_dir.join("0000_test_migration.sql"),
            "CREATE TABLE test (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        // Create a valid snapshot
        let snapshot = r#"{
            "version": "6",
            "dialect": "sqlite",
            "id": "00000000-0000-0000-0000-000000000000",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {},
            "enums": {},
            "_meta": { "tables": {}, "columns": {} }
        }"#;
        std::fs::write(meta_dir.join("0000_snapshot.json"), snapshot).unwrap();

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
    fn check_missing_sql_file() {
        let temp = TempDir::new().unwrap();
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");
        std::fs::create_dir_all(&meta_dir).unwrap();

        // Create journal referencing a missing SQL file
        let journal = r#"{
            "version": "7",
            "dialect": "sqlite",
            "entries": [
                {
                    "idx": 0,
                    "version": "6",
                    "when": 1700000000000,
                    "tag": "0000_missing",
                    "breakpoints": true
                }
            ]
        }"#;
        std::fs::write(meta_dir.join("_journal.json"), journal).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("check")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .assert()
            .failure()
            .stderr(predicate::str::contains("error"));
    }
}

mod generate {
    use super::*;

    #[test]
    fn generate_custom_migration() {
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
            .stdout(predicate::str::contains("Created custom migration"));

        // Check files were created
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let meta_dir = migrations_dir.join("meta");

        assert!(meta_dir.join("_journal.json").exists());

        // Find the SQL file (should have the name we specified)
        let sql_files: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .collect();

        assert_eq!(sql_files.len(), 1);
        let sql_name = sql_files[0].file_name();
        assert!(sql_name.to_string_lossy().contains("initial_setup"));
    }

    #[test]
    fn generate_requires_schema_for_non_custom() {
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
    fn generate_from_schema() {
        let temp = TempDir::new().unwrap();

        // Create a schema file
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

        let schema_path = temp.path().join("schema.json");
        std::fs::write(&schema_path, schema).unwrap();

        drizzle_cli()
            .current_dir(temp.path())
            .arg("generate")
            .arg("--out=drizzle")
            .arg("--dialect=sqlite")
            .arg("--schema=schema.json")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created migration"));

        // Verify the migration was created
        let migrations_dir = temp.path().join("drizzle").join("migrations");
        let sql_files: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .collect();

        assert_eq!(sql_files.len(), 1);

        // Check the SQL content
        let sql_content = std::fs::read_to_string(sql_files[0].path()).unwrap();
        assert!(sql_content.contains("CREATE TABLE"));
        assert!(sql_content.contains("users"));
    }
}

mod help {
    use super::*;

    #[test]
    fn help_shows_commands() {
        drizzle_cli()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("generate"))
            .stdout(predicate::str::contains("check"))
            .stdout(predicate::str::contains("status"))
            .stdout(predicate::str::contains("init"));
    }

    #[test]
    fn generate_help() {
        drizzle_cli()
            .arg("generate")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--schema"))
            .stdout(predicate::str::contains("--out"))
            .stdout(predicate::str::contains("--dialect"));
    }
}
