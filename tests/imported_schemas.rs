//! The Rust schemas `drizzle import` writes for real drizzle-kit migration
//! folders (`cli/tests/fixtures/import/expected`, kept in sync by the CLI's
//! `import` tests) compile with the macros, and the snapshot the macros
//! produce at runtime plans no change against drizzle-kit's last snapshot.
//!
//! The CLI tests check the same schemas through the schema parser (what
//! `drizzle generate` reads); this checks the compiled side.

#![cfg(feature = "std")]

#[cfg(feature = "sqlite")]
#[rustfmt::skip] // Compared byte for byte with `drizzle import` output.
#[path = "../cli/tests/fixtures/import/expected/beta_sqlite.rs"]
mod beta_sqlite;
#[cfg(feature = "sqlite")]
#[rustfmt::skip] // Compared byte for byte with `drizzle import` output.
#[path = "../cli/tests/fixtures/import/expected/stable_sqlite.rs"]
mod stable_sqlite;

#[cfg(all(
    feature = "postgres",
    feature = "serde",
    feature = "uuid",
    feature = "chrono",
    feature = "cidr"
))]
#[rustfmt::skip] // Compared byte for byte with `drizzle import` output.
#[path = "../cli/tests/fixtures/import/expected/beta_postgres.rs"]
mod beta_postgres;
#[cfg(all(
    feature = "postgres",
    feature = "serde",
    feature = "uuid",
    feature = "chrono",
    feature = "cidr"
))]
#[rustfmt::skip] // Compared byte for byte with `drizzle import` output.
#[path = "../cli/tests/fixtures/import/expected/stable_postgres.rs"]
mod stable_postgres;

#[cfg(all(feature = "mysql", feature = "serde"))]
#[rustfmt::skip] // Compared byte for byte with `drizzle import` output.
#[path = "../cli/tests/fixtures/import/expected/beta_mysql.rs"]
mod beta_mysql;
#[cfg(all(feature = "mysql", feature = "serde"))]
#[rustfmt::skip] // Compared byte for byte with `drizzle import` output.
#[path = "../cli/tests/fixtures/import/expected/stable_mysql.rs"]
mod stable_mysql;

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
mod support {
    use drizzle::migrations::schema::Snapshot;
    use drizzle::migrations::serde_json::{self, Value};
    use drizzle::migrations::upgrade::upgrade_to_latest;
    use drizzle_types::Dialect;
    use std::path::Path;

    /// The last snapshot of a drizzle-kit fixture folder, upgraded in memory.
    fn kit_snapshot(rel: &str, dialect: Dialect) -> Snapshot {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("cli/tests/fixtures/import")
            .join(rel);
        let path = if dir.join("meta/_journal.json").exists() {
            dir.join("meta/0001_snapshot.json")
        } else {
            let mut folders: Vec<_> = std::fs::read_dir(&dir)
                .expect("read fixture")
                .map(|entry| entry.expect("entry").path())
                .filter(|path| path.join("snapshot.json").exists())
                .collect();
            folders.sort();
            folders.pop().expect("a migration").join("snapshot.json")
        };
        let json: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("json");
        let upgraded = upgrade_to_latest(json, dialect);
        match dialect {
            Dialect::SQLite => Snapshot::Sqlite(serde_json::from_value(upgraded).expect("sqlite")),
            Dialect::PostgreSQL => {
                Snapshot::Postgres(serde_json::from_value(upgraded).expect("postgres"))
            }
            Dialect::MySQL => Snapshot::MySQL(serde_json::from_value(upgraded).expect("mysql")),
        }
    }

    /// Diffing drizzle-kit's snapshot against the compiled schema's must
    /// plan nothing.
    pub fn assert_matches_kit(rel: &str, compiled: &Snapshot) {
        let kit = kit_snapshot(rel, compiled.dialect());
        let plan = drizzle::migrations::diff(&kit, compiled).expect("diff");
        assert!(
            plan.statements.is_empty(),
            "the compiled schema for {rel} differs from drizzle-kit's snapshot: {:#?}",
            plan.statements
        );
    }
}

#[cfg(feature = "sqlite")]
#[test]
fn imported_sqlite_schemas_match_drizzle_kit() {
    use drizzle::migrations::schema::Schema as _;
    support::assert_matches_kit("stable/sqlite", &stable_sqlite::Schema::new().to_snapshot());
    support::assert_matches_kit("beta/sqlite", &beta_sqlite::Schema::new().to_snapshot());
}

#[cfg(all(
    feature = "postgres",
    feature = "serde",
    feature = "uuid",
    feature = "chrono",
    feature = "cidr"
))]
#[test]
fn imported_postgres_schemas_match_drizzle_kit() {
    use drizzle::core::SQLSchemaImpl;
    use drizzle::migrations::schema::Schema as _;
    support::assert_matches_kit(
        "stable/postgres",
        &stable_postgres::Schema::new().to_snapshot(),
    );
    support::assert_matches_kit("beta/postgres", &beta_postgres::Schema::new().to_snapshot());

    // The macros emit the kept constraint names in their own DDL too.
    let statements: Vec<String> = stable_postgres::Schema::new()
        .create_statements()
        .expect("create statements")
        .collect();
    let ddl = statements.join("\n");
    for expected in [
        "CONSTRAINT \"post_tags_post_id_tag_id_pk\" PRIMARY KEY(\"post_id\", \"tag_id\")",
        "CONSTRAINT \"post_tags_tag_fk\" FOREIGN KEY (\"tag_id\")",
        "CONSTRAINT \"post_tag_votes_post_id_tag_id_post_tags_post_id_tag_id_fk\" FOREIGN KEY (\"post_id\", \"tag_id\")",
        "CREATE TYPE \"auth\".\"account_status\" AS ENUM ('active', 'disabled')",
    ] {
        assert!(ddl.contains(expected), "missing `{expected}` in:\n{ddl}");
    }
}

#[cfg(all(feature = "mysql", feature = "serde"))]
#[test]
fn imported_mysql_schemas_match_drizzle_kit() {
    use drizzle::migrations::schema::Schema as _;
    support::assert_matches_kit("stable/mysql", &stable_mysql::Schema::new().to_snapshot());
    support::assert_matches_kit("beta/mysql", &beta_mysql::Schema::new().to_snapshot());
}
