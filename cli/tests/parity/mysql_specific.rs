use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;

#[test]
fn generate_preserves_mysql_storage_and_type_options() {
    let dir = tempdir().expect("create MySQL-specific temp directory");
    let root = dir.path();
    let config = root.join("drizzle.config.toml");
    let schema = root.join("schema.rs");
    let out = root.join("generated");
    let driver = if cfg!(feature = "mysql-sync") {
        "mysql-sync"
    } else {
        "mysql-async"
    };

    fs::write(
        &schema,
        r#"
#[derive(MySQLEnum)]
enum State { Draft, Published }

#[MySQLTable(NAME = "mysql_storage_contract", ENGINE = "InnoDB", CHARSET = "utf8mb4")]
struct Documents {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(ENUM)]
    state: State,
    sequence: u32,
    #[column(generated(STORED, "sequence + 1"))]
    next_sequence: u32,
}

#[MySQLIndex(unique, using = "BTREE", algorithm = "INPLACE", lock = "NONE")]
struct DocumentsState(Documents::state);
"#,
    )
    .expect("write MySQL-specific schema");
    fs::write(
        &config,
        format!(
            "dialect = \"mysql\"\ndriver = \"{driver}\"\nschema = '{}'\nout = '{}'\n\n[dbCredentials]\nurl = \"mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test\"\n",
            schema.to_string_lossy(),
            out.to_string_lossy(),
        ),
    )
    .expect("write MySQL-specific config");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "generate",
            "--driver",
            driver,
            "--name",
            "mysql_storage",
        ])
        .assert()
        .success();

    let migration_path = fs::read_dir(&out)
        .expect("read MySQL migration output")
        .filter_map(Result::ok)
        .find(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path().join("migration.sql"))
        .expect("find MySQL migration");
    let migration = fs::read_to_string(migration_path).expect("read MySQL migration");
    assert!(migration.contains("InnoDB"));
    assert!(migration.contains("utf8mb4"));
    assert!(migration.contains("enum('Draft', 'Published')"));
    assert!(migration.contains("GENERATED ALWAYS AS (sequence + 1) STORED"));
    assert!(migration.contains("USING BTREE"));
}
