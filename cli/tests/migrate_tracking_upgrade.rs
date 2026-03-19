#![cfg(feature = "rusqlite")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(root: &Path, db_path: &Path, migrations_dir: &Path, table: Option<&str>) {
    let migrations_block = table
        .map(|table| format!("\n[migrations]\ntable = \"{table}\"\n"))
        .unwrap_or_default();

    fs::write(
        root.join("drizzle.config.toml"),
        format!(
            r#"
dialect = "sqlite"
schema = '{schema_path}'
out = '{out_dir}'
{migrations_block}
[dbCredentials]
url = '{db_url}'
"#,
            schema_path = root.join("schema.rs").to_string_lossy(),
            out_dir = migrations_dir.to_string_lossy(),
            db_url = db_path.to_string_lossy(),
            migrations_block = migrations_block,
        ),
    )
    .expect("write config");

    fs::write(root.join("schema.rs"), "// test schema\n").expect("write schema");
}

fn write_migration(migrations_dir: &Path, tag: &str, sql: &str) {
    let dir = migrations_dir.join(tag);
    fs::create_dir_all(&dir).expect("create migration dir");
    fs::write(dir.join("migration.sql"), sql).expect("write migration sql");
}

fn tracking_columns(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
    let pragma = format!("SELECT name FROM pragma_table_info('{table}') ORDER BY cid");
    let mut stmt = conn.prepare(&pragma).expect("prepare pragma_table_info");
    stmt.query_map([], |row| row.get::<_, String>(0))
        .expect("query pragma_table_info")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect pragma columns")
}

#[test]
fn migrate_upgrades_legacy_tracking_table_and_applies_pending_migrations() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");
    write_config(root, &db_path, &migrations_dir, None);

    write_migration(
        &migrations_dir,
        "20230331141203_initial",
        "CREATE TABLE already_applied_table (id INTEGER PRIMARY KEY);\n",
    );
    write_migration(
        &migrations_dir,
        "20230331141204_pending",
        "CREATE TABLE pending_after_upgrade (id INTEGER PRIMARY KEY);\n",
    );

    let migrations = drizzle_migrations::MigrationDir::new(&migrations_dir)
        .discover()
        .expect("discover migrations");
    let first = &migrations[0];
    let second = &migrations[1];

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    conn.execute(
        "CREATE TABLE __drizzle_migrations (id INTEGER PRIMARY KEY AUTOINCREMENT, hash text NOT NULL, created_at numeric)",
        [],
    )
    .expect("create legacy tracking table");
    conn.execute(
        "CREATE TABLE already_applied_table (id INTEGER PRIMARY KEY)",
        [],
    )
    .expect("create applied table");
    conn.execute(
        "INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?1, ?2)",
        rusqlite::params![first.hash(), first.created_at()],
    )
    .expect("insert legacy applied row");
    drop(conn);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate"])
        .assert()
        .success();

    let conn = rusqlite::Connection::open(&db_path).expect("reopen sqlite");
    assert_eq!(
        tracking_columns(&conn, "__drizzle_migrations"),
        vec!["id", "hash", "created_at", "name", "applied_at"]
    );

    let rows: Vec<(String, i64, String, Option<String>)> = {
        let mut stmt = conn
            .prepare(
                "SELECT hash, created_at, name, applied_at FROM __drizzle_migrations ORDER BY id ASC",
            )
            .expect("prepare metadata query");
        stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("query metadata rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect metadata rows")
    };

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, first.hash());
    assert_eq!(rows[0].1, first.created_at());
    assert_eq!(rows[0].2, first.name());
    assert_eq!(
        rows[0].3, None,
        "backfilled legacy rows keep NULL applied_at"
    );
    assert_eq!(rows[1].0, second.hash());
    assert_eq!(rows[1].1, second.created_at());
    assert_eq!(rows[1].2, second.name());
    assert!(rows[1].3.is_some(), "newly applied rows set applied_at");

    let pending_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pending_after_upgrade'",
            [],
            |row| row.get(0),
        )
        .expect("query pending table");
    assert_eq!(pending_exists, 1);
}

#[test]
fn migrate_upgrades_legacy_custom_tracking_table() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("dev.db");
    let migrations_dir = root.join("migrations");
    write_config(root, &db_path, &migrations_dir, Some("custom_migrations"));

    write_migration(
        &migrations_dir,
        "20230331141203_initial",
        "CREATE TABLE custom_already_applied (id INTEGER PRIMARY KEY);\n",
    );

    let migrations = drizzle_migrations::MigrationDir::new(&migrations_dir)
        .discover()
        .expect("discover migrations");
    let first = &migrations[0];

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    conn.execute(
        "CREATE TABLE custom_migrations (id INTEGER PRIMARY KEY AUTOINCREMENT, hash text NOT NULL, created_at numeric)",
        [],
    )
    .expect("create custom legacy tracking table");
    conn.execute(
        "CREATE TABLE custom_already_applied (id INTEGER PRIMARY KEY)",
        [],
    )
    .expect("create applied table");
    conn.execute(
        "INSERT INTO custom_migrations (hash, created_at) VALUES (?1, ?2)",
        rusqlite::params![first.hash(), first.created_at()],
    )
    .expect("insert custom legacy applied row");
    drop(conn);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["migrate", "--verify"])
        .assert()
        .success();

    let conn = rusqlite::Connection::open(&db_path).expect("reopen sqlite");
    assert_eq!(
        tracking_columns(&conn, "custom_migrations"),
        vec!["id", "hash", "created_at", "name", "applied_at"]
    );

    let (name, applied_at): (String, Option<String>) = conn
        .query_row(
            "SELECT name, applied_at FROM custom_migrations LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query upgraded custom metadata row");
    assert_eq!(name, first.name());
    assert_eq!(applied_at, None);
}
