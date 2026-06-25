#![cfg(feature = "uuid")]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn sqlite_derive_macros_do_not_require_direct_backend_dependencies() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = fresh_fixture_root(&root);
    let drizzle_path = root.display().to_string().replace('\\', "/");

    #[cfg(all(feature = "libsql", feature = "uuid"))]
    {
        write_single_driver_fixture(
            &fixture_root.join("libsql_only"),
            "sqlite_feature_gate_libsql_only",
            &drizzle_path,
            "libsql",
        );
        cargo_check(&root, &fixture_root.join("libsql_only/Cargo.toml"), &[]);
    }

    #[cfg(all(feature = "rusqlite", feature = "uuid"))]
    {
        write_single_driver_fixture(
            &fixture_root.join("rusqlite_only"),
            "sqlite_feature_gate_rusqlite_only",
            &drizzle_path,
            "rusqlite",
        );
        cargo_check(&root, &fixture_root.join("rusqlite_only/Cargo.toml"), &[]);
    }

    #[cfg(all(feature = "turso", feature = "uuid"))]
    {
        write_single_driver_fixture(
            &fixture_root.join("turso_only"),
            "sqlite_feature_gate_turso_only",
            &drizzle_path,
            "turso",
        );
        cargo_check(&root, &fixture_root.join("turso_only/Cargo.toml"), &[]);
    }

    #[cfg(all(feature = "libsql", feature = "rusqlite", feature = "uuid"))]
    {
        write_unification_fixture(&fixture_root.join("unified"), &drizzle_path);
        cargo_check(
            &root,
            &fixture_root.join("unified/Cargo.toml"),
            &["--workspace"],
        );
    }
}

fn fresh_fixture_root(root: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_nanos();
    let dir = root
        .join("target")
        .join("sqlite-backend-feature-gating")
        .join(format!("{}-{nonce}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("remove stale sqlite feature-gating fixture");
    }
    fs::create_dir_all(&dir).expect("create sqlite feature-gating fixture root");
    dir
}

fn cargo_check(root: &Path, manifest_path: &Path, extra_args: &[&str]) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let target_dir = root.join("target");
    let fixture_workspace = manifest_path
        .parent()
        .expect("fixture manifest should have a parent directory");
    fs::copy(
        root.join("Cargo.lock"),
        fixture_workspace.join("Cargo.lock"),
    )
    .expect("copy root Cargo.lock into sqlite feature-gating fixture");
    let output = Command::new(cargo)
        .current_dir(root)
        .arg("check")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--offline")
        .args(extra_args)
        .arg("--quiet")
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .expect("run cargo check for sqlite feature-gating fixture");

    assert!(
        output.status.success(),
        "cargo check failed for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        manifest_path.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_single_driver_fixture(dir: &Path, package_name: &str, drizzle_path: &str, driver: &str) {
    fs::create_dir_all(dir.join("src")).expect("create single-driver fixture src");
    fs::write(
        dir.join("Cargo.toml"),
        package_manifest(package_name, drizzle_path, driver, true),
    )
    .expect("write single-driver fixture manifest");
    fs::write(dir.join("src/lib.rs"), consumer_source(driver))
        .expect("write single-driver fixture source");
}

#[cfg(all(feature = "libsql", feature = "rusqlite"))]
fn write_unification_fixture(dir: &Path, drizzle_path: &str) {
    fs::create_dir_all(dir).expect("create unified fixture root");
    fs::write(
        dir.join("Cargo.toml"),
        r#"[workspace]
resolver = "3"
members = ["consumer_libsql", "consumer_rusqlite"]
"#,
    )
    .expect("write unified workspace manifest");

    for driver in ["libsql", "rusqlite"] {
        let member = dir.join(format!("consumer_{driver}"));
        fs::create_dir_all(member.join("src")).expect("create unified member src");
        fs::write(
            member.join("Cargo.toml"),
            package_manifest(
                &format!("sqlite_feature_gate_unified_{driver}"),
                drizzle_path,
                driver,
                false,
            ),
        )
        .expect("write unified member manifest");
        fs::write(member.join("src/lib.rs"), consumer_source(driver))
            .expect("write unified member source");
    }
}

fn package_manifest(
    package_name: &str,
    drizzle_path: &str,
    driver: &str,
    standalone_workspace: bool,
) -> String {
    let workspace = if standalone_workspace {
        "\n[workspace]\n"
    } else {
        ""
    };
    format!(
        r#"[package]
name = "{package_name}"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
drizzle = {{ path = "{drizzle_path}", default-features = false, features = ["std", "{driver}", "uuid"] }}
uuid = {{ version = "1.18", features = ["v4"] }}
{workspace}"#
    )
}

fn consumer_source(driver: &str) -> String {
    let row_bounds = match driver {
        "rusqlite" => {
            r#"
    SelectFeatureGateItem: for<'row> ::std::convert::TryFrom<&'row drizzle::sqlite::rusqlite::Row<'row>>,
    FeatureGateProjection: for<'row> ::std::convert::TryFrom<&'row drizzle::sqlite::rusqlite::Row<'row>>,
    SelectFeatureGateItem: for<'row> drizzle::core::FromDrizzleRow<drizzle::sqlite::rusqlite::Row<'row>>,
    FeatureGateProjection: for<'row> drizzle::core::FromDrizzleRow<drizzle::sqlite::rusqlite::Row<'row>>,
    SelectFeatureGateItem: for<'row> drizzle::core::RowColumnList<drizzle::sqlite::rusqlite::Row<'row>>,
    FeatureGateProjection: for<'row> drizzle::core::RowColumnList<drizzle::sqlite::rusqlite::Row<'row>>,
    FeatureGateStatus: for<'row> drizzle::core::RowColumnList<drizzle::sqlite::rusqlite::Row<'row>>,
    FeatureGatePriority: for<'row> drizzle::core::RowColumnList<drizzle::sqlite::rusqlite::Row<'row>>,
"#
        }
        "libsql" => {
            r#"
    SelectFeatureGateItem: for<'row> ::std::convert::TryFrom<&'row drizzle::sqlite::libsql::Row>,
    FeatureGateProjection: for<'row> ::std::convert::TryFrom<&'row drizzle::sqlite::libsql::Row>,
    SelectFeatureGateItem: drizzle::core::FromDrizzleRow<drizzle::sqlite::libsql::Row>,
    FeatureGateProjection: drizzle::core::FromDrizzleRow<drizzle::sqlite::libsql::Row>,
    SelectFeatureGateItem: drizzle::core::RowColumnList<drizzle::sqlite::libsql::Row>,
    FeatureGateProjection: drizzle::core::RowColumnList<drizzle::sqlite::libsql::Row>,
    FeatureGateStatus: drizzle::core::RowColumnList<drizzle::sqlite::libsql::Row>,
    FeatureGatePriority: drizzle::core::RowColumnList<drizzle::sqlite::libsql::Row>,
"#
        }
        "turso" => {
            r#"
    SelectFeatureGateItem: for<'row> ::std::convert::TryFrom<&'row drizzle::sqlite::turso::Row>,
    FeatureGateProjection: for<'row> ::std::convert::TryFrom<&'row drizzle::sqlite::turso::Row>,
    SelectFeatureGateItem: drizzle::core::FromDrizzleRow<drizzle::sqlite::turso::Row>,
    FeatureGateProjection: drizzle::core::FromDrizzleRow<drizzle::sqlite::turso::Row>,
    SelectFeatureGateItem: drizzle::core::RowColumnList<drizzle::sqlite::turso::Row>,
    FeatureGateProjection: drizzle::core::RowColumnList<drizzle::sqlite::turso::Row>,
    FeatureGateStatus: drizzle::core::RowColumnList<drizzle::sqlite::turso::Row>,
    FeatureGatePriority: drizzle::core::RowColumnList<drizzle::sqlite::turso::Row>,
"#
        }
        other => panic!("unknown sqlite driver fixture `{other}`"),
    };

    let value_bounds = match driver {
        "rusqlite" => {
            r#"
    FeatureGateStatus: drizzle::sqlite::rusqlite::types::FromSql + drizzle::sqlite::rusqlite::types::ToSql,
    FeatureGatePriority: drizzle::sqlite::rusqlite::types::FromSql + drizzle::sqlite::rusqlite::types::ToSql,
"#
        }
        "libsql" => {
            r#"
    drizzle::sqlite::libsql::Value: ::std::convert::From<FeatureGateStatus>,
    drizzle::sqlite::libsql::Value: for<'value> ::std::convert::From<&'value FeatureGateStatus>,
    drizzle::sqlite::libsql::Value: ::std::convert::From<FeatureGatePriority>,
    drizzle::sqlite::libsql::Value: for<'value> ::std::convert::From<&'value FeatureGatePriority>,
"#
        }
        "turso" => {
            r#"
    FeatureGateStatus: drizzle::sqlite::turso::IntoValue,
    for<'value> &'value FeatureGateStatus: drizzle::sqlite::turso::IntoValue,
    FeatureGatePriority: drizzle::sqlite::turso::IntoValue,
    for<'value> &'value FeatureGatePriority: drizzle::sqlite::turso::IntoValue,
"#
        }
        other => panic!("unknown sqlite driver fixture `{other}`"),
    };

    CONSUMER_TEMPLATE
        .replace("__ROW_BOUNDS__", row_bounds)
        .replace("__VALUE_BOUNDS__", value_bounds)
}

const CONSUMER_TEMPLATE: &str = r#"
use drizzle::sqlite::prelude::*;

#[derive(SQLiteEnum, Debug, Clone, PartialEq, Eq, Default)]
pub enum FeatureGateStatus {
    #[default]
    Pending,
    Done,
}

#[repr(i64)]
#[derive(SQLiteEnum, Debug, Clone, PartialEq, Eq, Default)]
pub enum FeatureGatePriority {
    #[default]
    Low = 1,
    High = 2,
}

#[SQLiteTable(NAME = "feature_gate_items")]
pub struct FeatureGateItem {
    #[column(PRIMARY)]
    id: uuid::Uuid,
    #[column(ENUM)]
    status: FeatureGateStatus,
    #[column(integer, ENUM)]
    priority: FeatureGatePriority,
}

#[derive(SQLiteFromRow, Debug)]
pub struct FeatureGateProjection {
    id: uuid::Uuid,
    status: FeatureGateStatus,
    priority: FeatureGatePriority,
}

pub fn assert_generated_driver_impls()
where
__ROW_BOUNDS__
__VALUE_BOUNDS__
{
    let _ = InsertFeatureGateItem::new(
        uuid::Uuid::new_v4(),
        FeatureGateStatus::Pending,
        FeatureGatePriority::Low,
    );
}
"#;
