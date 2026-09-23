//! Compiles standalone consumer crates against this checkout.
//!
//! Macro expansions are type-checked in the consumer's crate, against the
//! consumer's features, dependency names and lints. Tests inside this package
//! cannot see mistakes such as an emitted `#[cfg(feature = "tokio-postgres")]`
//! (evaluated against a consumer that has no such feature) or a hardcoded
//! `::uuid::` path (the consumer may reach the type through another name).
//! Each fixture depends on `drizzle` plus only the crates its own source
//! names, and denies warnings.
//!
//! The tests are `#[ignore]`d because each one builds several crates; run
//! them with `cargo test -p drizzle --test downstream_consumers -- --ignored`.
//!
//! Fixtures resolve offline against this repository's `Cargo.lock` so pull
//! request runs are deterministic. That needs every crate in the lockfile in
//! the local registry cache; in a fresh environment run `cargo fetch` first,
//! as CI does. With `DRIZZLE_DOWNSTREAM_FRESH=1` they
//! resolve every dependency fresh from the registry instead, which is what a
//! new user gets; the scheduled downstream workflow runs that mode to catch
//! upstream releases that break a supported version range.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
#[ignore = "builds standalone crates; run with --ignored (CI: Test Downstream Consumers)"]
fn postgres_derives_compile_in_consumer_crates() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = fresh_fixture_root(&root);
    for driver in ["tokio-postgres", "postgres-sync"] {
        let dir = fixtures.join(format!("postgres_{}", driver.replace('-', "_")));
        write_fixture(
            &dir,
            &format!("downstream_postgres_{}", driver.replace('-', "_")),
            &root,
            &[driver, "uuid", "query", "serde"],
            &[
                r#"uuid = { version = "1.18", features = ["v4"] }"#,
                SERDE_DEPENDENCY,
            ],
            POSTGRES_SOURCE,
        );
        cargo_check(&root, &dir);
    }
}

#[test]
#[ignore = "builds standalone crates; run with --ignored (CI: Test Downstream Consumers)"]
fn mysql_derives_compile_in_consumer_crates() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = fresh_fixture_root(&root).join("mysql_sync");
    write_fixture(
        &dir,
        "downstream_mysql_sync",
        &root,
        &["mysql-sync", "uuid", "query", "serde"],
        &[
            r#"uuid = { version = "1.18", features = ["v4"] }"#,
            SERDE_DEPENDENCY,
        ],
        MYSQL_SOURCE,
    );
    cargo_check(&root, &dir);
}

#[test]
#[ignore = "builds standalone crates; run with --ignored (CI: Test Downstream Consumers)"]
fn sqlite_uuid_columns_follow_the_declared_type() {
    // rusqlite and turso share one crate (the macros emit every enabled
    // driver's row codecs, so both decode paths are checked in one build);
    // libsql gets its own. rusqlite >= 0.40 bundles a newer SQLite than
    // libsql's fork, so a binary that links both hits duplicate `sqlite3_*`
    // symbols; that combination is an upstream limitation, not something
    // these fixtures should pin down. `sqlite_backend_feature_gating` covers
    // single-driver and unified builds against the lockfile.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = fresh_fixture_root(&root);
    for (name, drivers) in [
        ("rusqlite_turso", &["rusqlite", "turso"][..]),
        ("libsql", &["libsql"][..]),
    ] {
        let dir = fixtures.join(format!("sqlite_{name}"));
        let mut features = drivers.to_vec();
        features.extend(["uuid", "query", "serde"]);
        write_fixture(
            &dir,
            &format!("downstream_sqlite_{name}"),
            &root,
            &features,
            // Renamed on purpose: generated code must not assume `::uuid`.
            &[
                r#"ids = { package = "uuid", version = "1.18", features = ["v4"] }"#,
                SERDE_DEPENDENCY,
            ],
            SQLITE_SOURCE,
        );
        cargo_check(&root, &dir);
    }
}

/// JSON payloads only need `serde` itself: the fixtures deliberately do not
/// depend on `serde_json`, so generated code must not name it.
const SERDE_DEPENDENCY: &str = r#"serde = { version = "1", features = ["derive"] }"#;

fn fresh_mode() -> bool {
    std::env::var_os("DRIZZLE_DOWNSTREAM_FRESH").is_some_and(|value| value != "0")
}

fn fresh_fixture_root(root: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_nanos();
    let dir = root
        .join("target")
        .join("downstream-consumers")
        .join(format!("{}-{nonce}", std::process::id()));
    fs::create_dir_all(&dir).expect("create downstream fixture root");
    dir
}

fn write_fixture(
    dir: &Path,
    package_name: &str,
    root: &Path,
    drizzle_features: &[&str],
    extra_dependencies: &[&str],
    source: &str,
) {
    fs::create_dir_all(dir.join("src")).expect("create fixture src");
    let drizzle_path = root.display().to_string().replace('\\', "/");
    let features = drizzle_features
        .iter()
        .map(|feature| format!("{feature:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let manifest = format!(
        r#"[package]
name = "{package_name}"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
drizzle = {{ path = "{drizzle_path}", default-features = false, features = ["std", {features}] }}
{extra}

[workspace]
"#,
        extra = extra_dependencies.join("\n"),
    );
    fs::write(dir.join("Cargo.toml"), manifest).expect("write fixture manifest");
    fs::write(dir.join("src/lib.rs"), source).expect("write fixture source");
}

fn cargo_check(root: &Path, fixture: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let manifest_path = fixture.join("Cargo.toml");
    let mut command = Command::new(cargo);
    command
        .current_dir(root)
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .env("CARGO_TARGET_DIR", root.join("target"));
    if !fresh_mode() {
        fs::copy(root.join("Cargo.lock"), fixture.join("Cargo.lock"))
            .expect("copy root Cargo.lock into downstream fixture");
        // Workspace patches do not propagate through a path dependency. Repeat
        // this repository's parser patch so the copied lockfile resolves
        // without network access.
        let parser_path = root
            .join("bench/vendor/libsql-sqlite3-parser")
            .display()
            .to_string()
            .replace('\\', "/");
        command
            .arg("--config")
            .arg(format!(
                "patch.crates-io.libsql-sqlite3-parser.path={parser_path:?}"
            ))
            .arg("--offline");
    }
    let output = command
        .output()
        .expect("run cargo check for downstream fixture");
    assert!(
        output.status.success(),
        "cargo check failed for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        manifest_path.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const POSTGRES_SOURCE: &str = r#"
#![deny(warnings)]

use drizzle::postgres::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Address {
    pub city: String,
}

// `Address` backs JSON columns in two tables, and `Vec<String>` is a foreign
// payload: neither may receive generated impls.
#[PostgresTable]
pub struct Customers {
    #[column(primary)]
    pub id: i32,
    #[column(jsonb)]
    pub address: Address,
    #[column(json)]
    pub tags: Vec<String>,
}

#[PostgresTable]
pub struct Shipments {
    #[column(primary)]
    pub id: i32,
    #[column(jsonb)]
    pub destination: Address,
    #[column(jsonb)]
    pub previous: Option<Address>,
}

#[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum Mood {
    #[default]
    Happy,
    Sad,
}

#[repr(i32)]
#[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum Level {
    #[default]
    Low = 1,
    High = 2,
}

#[PostgresTable]
pub struct Accounts {
    #[column(primary)]
    pub id: i32,
    pub name: String,
    #[column(enum)]
    pub mood: Mood,
    #[column(enum)]
    pub level: Level,
    pub external_id: uuid::Uuid,
}

#[PostgresTable]
pub struct Posts {
    #[column(primary)]
    pub id: i32,
    #[column(references = Accounts::id)]
    pub account_id: i32,
}

#[PostgresIndex(unique)]
pub struct AccountsNameIdx(Accounts::name);

#[derive(PostgresSchema)]
pub struct Schema {
    pub mood: Mood,
    pub accounts: Accounts,
    pub posts: Posts,
    pub accounts_name_idx: AccountsNameIdx,
}

#[derive(PostgresFromRow, Debug, Default)]
pub struct AccountName {
    pub name: String,
    pub mood: Mood,
}

pub fn assert_row_impls()
where
    SelectAccounts: for<'row> ::std::convert::TryFrom<&'row drizzle::postgres::Row>,
    SelectAccounts: drizzle::core::FromDrizzleRow<drizzle::postgres::Row>,
    AccountName: for<'row> ::std::convert::TryFrom<&'row drizzle::postgres::Row>,
    AccountName: drizzle::core::FromDrizzleRow<drizzle::postgres::Row>,
    AccountName: drizzle::core::RowColumnList<drizzle::postgres::Row>,
    Mood: drizzle::core::FromDrizzleRow<drizzle::postgres::Row>,
    Mood: drizzle::core::RowColumnList<drizzle::postgres::Row>,
    Level: drizzle::core::FromDrizzleRow<drizzle::postgres::Row>,
{
    let _ = InsertAccounts::new(1, "a", Mood::Happy, Level::Low, uuid::Uuid::nil());
    let _ = InsertCustomers::new(1, Address::default(), vec!["a".into()]);
    let _ = InsertShipments::new(1, Address::default()).with_previous(Address::default());
    let _ = UpdateShipments::default().with_destination(Address::default());
    let _ = drizzle::core::expr::eq(Shipments::default().destination, drizzle::core::Json(Address::default()));
}
"#;

const MYSQL_SOURCE: &str = r#"
#![deny(warnings)]

use drizzle::mysql::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Address {
    pub city: String,
}

// `Address` backs JSON columns in two tables, and `Vec<String>` is a foreign
// payload: neither may receive generated impls.
#[MySQLTable]
pub struct Customers {
    #[column(PRIMARY)]
    pub id: i32,
    #[column(JSON)]
    pub address: Address,
    #[column(JSON)]
    pub tags: Vec<String>,
}

#[MySQLTable]
pub struct Shipments {
    #[column(PRIMARY)]
    pub id: i32,
    #[column(JSON)]
    pub destination: Address,
    #[column(JSON)]
    pub previous: Option<Address>,
}

#[derive(MySQLEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Draft,
    Published,
}

#[MySQLTable]
pub struct Accounts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(VARCHAR(255))]
    pub email: String,
    #[column(ENUM)]
    pub status: Status,
    pub external_id: uuid::Uuid,
}

#[MySQLTable]
pub struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(references = Accounts::id)]
    pub account_id: u64,
}

#[MySQLIndex]
pub struct AccountsEmailIdx(Accounts::email);

#[derive(MySQLSchema)]
pub struct Schema {
    pub accounts: Accounts,
    pub posts: Posts,
    pub accounts_email_idx: AccountsEmailIdx,
}

#[derive(MySQLFromRow, Debug, Default)]
#[from(Accounts)]
pub struct AccountEmail {
    pub id: u64,
    pub email: String,
}

pub fn build() {
    let _ = InsertAccounts::new("a@example.com", Status::Draft, uuid::Uuid::nil());
    let _ = InsertCustomers::new(1, Address::default(), vec!["a".into()]);
    let _ = InsertShipments::new(1, Address::default()).with_previous(Address::default());
    let _ = UpdateShipments::default().with_destination(Address::default());
    let _ = drizzle::core::expr::eq(Shipments::default().destination, drizzle::core::Json(Address::default()));
    let _ = Schema::new();
}
"#;

const SQLITE_SOURCE: &str = r#"
#![deny(warnings)]

use drizzle::sqlite::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Address {
    pub city: String,
}

// `Address` backs JSON columns in two tables, and `Vec<String>` is a foreign
// payload: neither may receive generated impls.
#[SQLiteTable]
pub struct Customers {
    #[column(primary)]
    pub id: i64,
    #[column(json)]
    pub address: Address,
    #[column(json)]
    pub tags: Vec<String>,
}

#[SQLiteTable]
pub struct Shipments {
    #[column(primary)]
    pub id: i64,
    #[column(json)]
    pub destination: Address,
    #[column(json)]
    pub previous: Option<Address>,
}

#[SQLiteTable]
pub struct Items {
    #[column(primary)]
    pub id: i64,
    pub blob_id: ids::Uuid,
    #[column(text)]
    pub text_id: ids::Uuid,
    pub maybe_id: Option<ids::Uuid>,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub items: Items,
}

#[derive(SQLiteFromRow, Debug)]
pub struct ItemIds {
    pub blob_id: ids::Uuid,
    pub text_id: ids::Uuid,
}

pub fn build() {
    let _ = InsertItems::new(ids::Uuid::nil(), ids::Uuid::nil()).with_maybe_id(ids::Uuid::nil());
    let _ = InsertCustomers::new(Address::default(), vec!["a".into()]);
    let _ = InsertShipments::new(Address::default()).with_previous(Address::default());
    let _ = UpdateShipments::default().with_destination(Address::default());
    let _ = drizzle::core::expr::eq(Shipments::default().destination, drizzle::core::Json(Address::default()));
    let _ = Schema::new();
}
"#;
