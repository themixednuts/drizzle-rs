//! Reading drizzle-kit's format 10 `PostgreSQL` migration folders.
//!
//! `fixtures/postgres_v10_history` was written by drizzle-kit 1.0.0-rc.6
//! from the schemas in its `schema/` folder: `init` (v1), `rename` (v2:
//! renames a table, a column and an index, drops a unique, adds a column),
//! then two branches from `rename` (`branch_a`: v3a, `branch_b`: v3b) and
//! `merged` (v4), whose snapshot follows both. `drizzle-kit-states.json`
//! holds drizzle-kit's own `reconstruct` of every snapshot.

use drizzle_migrations::history::load_merge_base;
use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::postgres::reconstruct::reconstruct_folder;
use drizzle_migrations::schema::Snapshot;
use drizzle_types::Dialect;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/postgres_v10_history")
}

/// drizzle-kit's state of every snapshot, by snapshot id.
fn drizzle_kit_states() -> BTreeMap<String, PostgresSnapshot> {
    let text = std::fs::read_to_string(fixture().join("drizzle-kit-states.json")).expect("read");
    let states: BTreeMap<String, Value> = serde_json::from_str(&text).expect("json");
    states
        .into_iter()
        .map(|(id, ddl)| {
            let snapshot = json!({
                "version": "8", "dialect": "postgresql", "id": id,
                "prevIds": [], "ddl": ddl, "renames": [],
            });
            (
                id,
                serde_json::from_value(snapshot).expect("drizzle-kit state loads"),
            )
        })
        .collect()
}

/// The entities of a snapshot as sorted JSON strings.
fn entities(snapshot: &PostgresSnapshot) -> Vec<String> {
    let mut entities: Vec<String> = snapshot
        .ddl
        .iter()
        .map(|entity| serde_json::to_string(entity).expect("serialize"))
        .collect();
    entities.sort();
    entities
}

#[test]
fn every_snapshot_reconstructs_to_drizzle_kits_state() {
    for (id, expected) in drizzle_kit_states() {
        assert!(!expected.ddl.is_empty(), "snapshot {id} has entities");
        let reconstructed = reconstruct_folder(&fixture(), &id).expect("reconstructs");

        assert_eq!(
            entities(&reconstructed),
            entities(&expected),
            "snapshot {id}"
        );
        let plan = drizzle_migrations::diff(
            &Snapshot::Postgres(expected),
            &Snapshot::Postgres(reconstructed),
        )
        .expect("diff");
        assert!(
            plan.statements.is_empty(),
            "snapshot {id}: {:#?}",
            plan.statements
        );
    }
}

#[test]
fn generate_diffs_against_the_merged_state() {
    let states = drizzle_kit_states();
    let merged = &states["b25dfae7-d985-4b77-bb19-2bc591ca525b"];

    let base = load_merge_base(&fixture(), Dialect::PostgreSQL, false).expect("merge base");

    assert!(
        base.prev_ids.is_none(),
        "the merge snapshot is the only leaf"
    );
    let Snapshot::Postgres(snapshot) = base.snapshot else {
        panic!("a postgres snapshot");
    };
    assert_eq!(snapshot.id, "b25dfae7-d985-4b77-bb19-2bc591ca525b");
    assert_eq!(entities(&snapshot), entities(merged));
}

#[test]
fn a_newer_snapshot_format_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("20260101000000_init");
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::write(
        folder.join("snapshot.json"),
        r#"{"type":"base","version":"11","dialect":"postgres","id":"a","prevIds":[],"ddl":[]}"#,
    )
    .expect("write");

    let error = load_merge_base(dir.path(), Dialect::PostgreSQL, false).expect_err("format 11");

    assert!(
        error.to_string().contains("newer than drizzle-rs reads"),
        "{error}"
    );
}
