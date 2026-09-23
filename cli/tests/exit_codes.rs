//! Commands that could not do what they were asked must exit non-zero.
//!
//! None of these cases reach a driver, so the file runs under every feature
//! set, including `--no-default-features`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(root: &Path, credentials: &str) {
    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            "dialect = \"sqlite\"\nschema = '{}'\nout = '{}'\n{credentials}",
            root.join("schema.rs").to_string_lossy(),
            root.join("migrations").to_string_lossy(),
        ),
    )
    .expect("write config");
    fs::write(root.join("schema.rs"), "// no tables\n").expect("write schema");
}

#[test]
fn live_commands_fail_without_credentials() {
    for command in ["migrate", "push", "pull", "introspect"] {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        write_config(root, "");
        fs::create_dir_all(root.join("migrations")).expect("create migrations dir");

        cargo_bin_cmd!("drizzle")
            .current_dir(root)
            .arg(command)
            .assert()
            .failure()
            .stderr(contains("No database credentials configured"));
    }
}

#[test]
fn migrate_fails_when_the_migrations_directory_is_missing() {
    for mode in [None, Some("--plan"), Some("--verify"), Some("--safe")] {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        write_config(
            root,
            &format!(
                "\n[dbCredentials]\nurl = '{}'\n",
                root.join("dev.db").to_string_lossy()
            ),
        );

        let mut command = cargo_bin_cmd!("drizzle");
        command.current_dir(root).arg("migrate");
        if let Some(mode) = mode {
            command.arg(mode);
        }
        command
            .assert()
            .failure()
            .stderr(contains("Migrations directory not found"));
        assert!(
            !root.join("dev.db").exists(),
            "a failed `migrate {mode:?}` must not create the database"
        );
    }
}
