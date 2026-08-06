//! `drizzle up` on a legacy stable-drizzle-kit migrations directory (flat
//! `NNNN_name.sql` + `meta/_journal.json` + `meta/NNNN_snapshot.json`) must
//! convert the whole directory to the v1-beta folder layout with snapshots
//! upgraded to the current entity-array format.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(root: &Path, migrations_dir: &Path) {
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
            db_url = root.join("dev.db").to_string_lossy(),
        ),
    )
    .expect("write config");

    fs::write(root.join("schema.rs"), "// test schema\n").expect("write schema");
}

const INITIAL_SQL: &str = "CREATE TABLE `users` (\n\t`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,\n\t`email` text NOT NULL\n);\n";
const SECOND_SQL: &str = "CREATE TABLE `posts` (\n\t`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,\n\t`author_id` integer NOT NULL,\n\tFOREIGN KEY (`author_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE cascade\n);\n--> statement-breakpoint\nCREATE UNIQUE INDEX `users_email_unique` ON `users` (`email`);\n";

fn snapshot_v6_initial() -> String {
    serde_json::json!({
        "version": "6",
        "dialect": "sqlite",
        "id": "0e2fd32d-31b7-4b70-8e9e-9c5c1a010001",
        "prevId": "00000000-0000-0000-0000-000000000000",
        "tables": {
            "users": {
                "name": "users",
                "columns": {
                    "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true, "autoincrement": true},
                    "email": {"name": "email", "type": "text", "primaryKey": false, "notNull": true, "autoincrement": false}
                },
                "indexes": {},
                "foreignKeys": {},
                "compositePrimaryKeys": {},
                "uniqueConstraints": {}
            }
        },
        "views": {},
        "enums": {},
        "_meta": {"tables": {}, "columns": {}}
    })
    .to_string()
}

fn snapshot_v6_second() -> String {
    serde_json::json!({
        "version": "6",
        "dialect": "sqlite",
        "id": "0e2fd32d-31b7-4b70-8e9e-9c5c1a010002",
        "prevId": "0e2fd32d-31b7-4b70-8e9e-9c5c1a010001",
        "tables": {
            "users": {
                "name": "users",
                "columns": {
                    "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true, "autoincrement": true},
                    "email": {"name": "email", "type": "text", "primaryKey": false, "notNull": true, "autoincrement": false}
                },
                "indexes": {
                    "users_email_unique": {"name": "users_email_unique", "columns": ["email"], "isUnique": true}
                },
                "foreignKeys": {},
                "compositePrimaryKeys": {},
                "uniqueConstraints": {}
            },
            "posts": {
                "name": "posts",
                "columns": {
                    "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true, "autoincrement": true},
                    "author_id": {"name": "author_id", "type": "integer", "primaryKey": false, "notNull": true, "autoincrement": false}
                },
                "indexes": {},
                "foreignKeys": {
                    "posts_author_id_users_id_fk": {
                        "name": "posts_author_id_users_id_fk",
                        "tableFrom": "posts",
                        "tableTo": "users",
                        "columnsFrom": ["author_id"],
                        "columnsTo": ["id"],
                        "onDelete": "cascade",
                        "onUpdate": "no action"
                    }
                },
                "compositePrimaryKeys": {},
                "uniqueConstraints": {}
            }
        },
        "views": {},
        "enums": {},
        "_meta": {"tables": {}, "columns": {}}
    })
    .to_string()
}

fn journal() -> String {
    serde_json::json!({
        "version": "6",
        "dialect": "sqlite",
        "entries": [
            {"idx": 0, "version": "6", "when": 1700000000000u64, "tag": "0000_flimsy_shard", "breakpoints": true},
            {"idx": 1, "version": "6", "when": 1700000001000u64, "tag": "0001_curved_rogue", "breakpoints": true}
        ]
    })
    .to_string()
}

fn write_legacy_layout(migrations_dir: &Path) {
    let meta = migrations_dir.join("meta");
    fs::create_dir_all(&meta).expect("create meta dir");

    fs::write(migrations_dir.join("0000_flimsy_shard.sql"), INITIAL_SQL).expect("write sql");
    fs::write(migrations_dir.join("0001_curved_rogue.sql"), SECOND_SQL).expect("write sql");
    fs::write(meta.join("_journal.json"), journal()).expect("write journal");
    fs::write(meta.join("0000_snapshot.json"), snapshot_v6_initial()).expect("write snapshot");
    fs::write(meta.join("0001_snapshot.json"), snapshot_v6_second()).expect("write snapshot");
}

#[test]
fn up_converts_legacy_layout_to_folders() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let migrations_dir = root.join("migrations");
    write_config(root, &migrations_dir);
    write_legacy_layout(&migrations_dir);

    // Discovery must reject the legacy layout before the upgrade...
    assert!(
        drizzle_migrations::MigrationDir::new(&migrations_dir)
            .discover()
            .is_err(),
        "legacy layout should be rejected by discovery until upgraded"
    );

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["up"])
        .assert()
        .success();

    // SQL moved verbatim into {tag}/migration.sql.
    assert_eq!(
        fs::read_to_string(
            migrations_dir
                .join("0000_flimsy_shard")
                .join("migration.sql")
        )
        .expect("read converted sql"),
        INITIAL_SQL
    );
    assert_eq!(
        fs::read_to_string(
            migrations_dir
                .join("0001_curved_rogue")
                .join("migration.sql")
        )
        .expect("read converted sql"),
        SECOND_SQL
    );

    // Snapshots converted to the current entity-array format, ids preserved.
    let snapshot = drizzle_migrations::sqlite::SQLiteSnapshot::from_json(
        &fs::read_to_string(
            migrations_dir
                .join("0001_curved_rogue")
                .join("snapshot.json"),
        )
        .expect("read converted snapshot"),
    )
    .expect("converted snapshot parses as the current format");
    assert_eq!(
        snapshot.version,
        drizzle_migrations::version::SQLITE_SNAPSHOT_VERSION
    );
    assert_eq!(snapshot.id, "0e2fd32d-31b7-4b70-8e9e-9c5c1a010002");
    assert_eq!(
        snapshot.prev_ids,
        vec!["0e2fd32d-31b7-4b70-8e9e-9c5c1a010001".to_string()]
    );
    assert!(!snapshot.ddl.is_empty());

    // Legacy files are gone.
    assert!(!migrations_dir.join("0000_flimsy_shard.sql").exists());
    assert!(!migrations_dir.join("0001_curved_rogue.sql").exists());
    assert!(!migrations_dir.join("meta").exists());

    // ...and accepted afterwards, in order.
    let migrations = drizzle_migrations::MigrationDir::new(&migrations_dir)
        .discover()
        .expect("discovery accepts the converted layout");
    assert_eq!(migrations.len(), 2);
    assert_eq!(migrations[0].name(), "0000_flimsy_shard");
    assert_eq!(migrations[1].name(), "0001_curved_rogue");
}

#[test]
fn up_is_idempotent_after_conversion() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let migrations_dir = root.join("migrations");
    write_config(root, &migrations_dir);
    write_legacy_layout(&migrations_dir);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["up"])
        .assert()
        .success();

    let snapshot_before = fs::read_to_string(
        migrations_dir
            .join("0000_flimsy_shard")
            .join("snapshot.json"),
    )
    .expect("read snapshot");

    // Second run: nothing legacy left, snapshots already current.
    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["up"])
        .assert()
        .success()
        .stdout(predicates::str::contains("already at the latest version"));

    let snapshot_after = fs::read_to_string(
        migrations_dir
            .join("0000_flimsy_shard")
            .join("snapshot.json"),
    )
    .expect("read snapshot");
    assert_eq!(snapshot_before, snapshot_after);
}

#[test]
fn up_leaves_legacy_files_alone_when_an_entry_is_broken() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let migrations_dir = root.join("migrations");
    write_config(root, &migrations_dir);
    write_legacy_layout(&migrations_dir);

    // Corrupt the second snapshot: conversion must refuse to touch anything.
    fs::write(
        migrations_dir.join("meta").join("0001_snapshot.json"),
        "{ not json",
    )
    .expect("corrupt snapshot");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["up"])
        .assert()
        .failure();

    // The legacy layout is intact and no partial folders were written.
    assert!(migrations_dir.join("0000_flimsy_shard.sql").exists());
    assert!(migrations_dir.join("0001_curved_rogue.sql").exists());
    assert!(migrations_dir.join("meta").join("_journal.json").exists());
    assert!(!migrations_dir.join("0000_flimsy_shard").exists());
    assert!(!migrations_dir.join("0001_curved_rogue").exists());
}
