//! The snapshot history of a migrations folder and the baseline `generate`
//! diffs against.
//!
//! Every `snapshot.json` names the snapshots it follows in `prevIds`, so a
//! migrations folder holds a graph. On one branch the graph is a chain and
//! the newest snapshot is the baseline. After a git merge brings in another
//! branch's migration folders, the graph has several open heads (*leaves*:
//! snapshots no other snapshot follows), and the newest folder alone no
//! longer describes the schema the migrations produce.
//!
//! [`load_merge_base`] follows drizzle-kit (`checkHandler` and
//! `commutativity/engine.ts`): with several leaves it checks that the
//! branches do not touch the same objects, merges every branch's changes
//! onto their lowest common ancestor, and returns that merged schema as the
//! baseline, plus every leaf id for the next snapshot's `prevIds`. Branches
//! that conflict are an error.
//!
//! drizzle-kit decides conflicts from typed diff statements per dialect;
//! this compares snapshot entities instead. Two branches conflict when they
//! change the same entity (table, column, index, constraint, enum, view,
//! ...), when one changes an entity whose table or schema the other drops
//! or renames, and on SQLite when one changes a table the other rebuilds
//! (a primary-key, foreign-key, unique, check, or column-definition change).

use crate::schema::Snapshot;
use crate::version::ORIGIN_UUID;
use drizzle_types::Dialect;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The baseline for the next `generate` diff.
#[derive(Debug, Clone)]
pub struct MergeBase {
    /// The schema the existing migrations produce.
    pub snapshot: Snapshot,
    /// `prevIds` for the next snapshot when the folder has several leaves
    /// (sorted leaf ids); `None` to follow [`snapshot`](Self::snapshot) as
    /// usual.
    pub prev_ids: Option<Vec<String>>,
}

/// Two branches of the migration history that change the same objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchConflict {
    /// Folder of the snapshot both branches start from, or `None` when they
    /// share no snapshot (both start from an empty schema).
    pub parent: Option<String>,
    /// Folder of one branch's newest snapshot.
    pub left: String,
    /// Folder of the other branch's newest snapshot.
    pub right: String,
    /// What both branches touch.
    pub reason: String,
}

impl std::fmt::Display for BranchConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "`{}` and `{}` (branched from {}): {}",
            self.left,
            self.right,
            self.parent
                .as_deref()
                .map_or_else(|| "an empty schema".to_string(), |parent| format!("`{parent}`")),
            self.reason
        )
    }
}

/// Errors from [`load_merge_base`].
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// Reading the folder or a snapshot failed.
    #[error("failed to read migration snapshots: {0}")]
    Io(#[from] std::io::Error),

    /// The history has several leaves whose changes conflict.
    #[error(
        "non-commutative migrations detected: {} conflict(s) between migration branches\n  {}\n\
         Regenerate one branch's migration on top of the other, or pass \
         `ignore_conflicts` (CLI: `--ignore-conflicts`) to diff against the newest \
         snapshot only.",
        .0.len(),
        .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n  ")
    )]
    Conflicts(Vec<BranchConflict>),
}

/// One snapshot in the history graph.
struct Node {
    folder: String,
    prev_ids: Vec<String>,
    snapshot: Snapshot,
    entities: Entities,
}

/// Identity of a snapshot entity: kind, schema/database, table, name (or,
/// for nameless entities, the whole serialized entity).
type EntityKey = (String, Option<String>, Option<String>, String);
type Entities = BTreeMap<EntityKey, Value>;
/// Entity changes from one snapshot to another: `None` means removed.
type Changes = BTreeMap<EntityKey, Option<Value>>;

/// Loads the baseline for the next migration in `out_dir`.
///
/// With no snapshot it is an empty schema, and with one leaf it is the
/// newest snapshot (by folder name), as before branch merges were handled.
/// With several leaves it is every branch's changes merged onto their
/// lowest common ancestor, and [`MergeBase::prev_ids`] lists the leaves.
///
/// With `ignore_conflicts`, conflicting branches fall back to the newest
/// snapshot (drizzle-kit's `--ignore-conflicts`).
///
/// # Errors
///
/// Returns [`HistoryError::Io`] when a snapshot cannot be read or parsed,
/// and [`HistoryError::Conflicts`] when branches conflict and
/// `ignore_conflicts` is off.
pub fn load_merge_base(
    out_dir: &Path,
    dialect: Dialect,
    ignore_conflicts: bool,
) -> Result<MergeBase, HistoryError> {
    let nodes = load_nodes(out_dir, dialect)?;
    let Some(latest) = nodes.last() else {
        return Ok(MergeBase {
            snapshot: Snapshot::empty(dialect),
            prev_ids: None,
        });
    };
    let newest = MergeBase {
        snapshot: latest.snapshot.clone(),
        prev_ids: None,
    };

    let mut by_id: HashMap<&str, &Node> = HashMap::new();
    for node in &nodes {
        by_id.insert(node.snapshot.id(), node);
    }
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in by_id.values() {
        for parent in &node.prev_ids {
            children
                .entry(parent.as_str())
                .or_default()
                .push(node.snapshot.id());
        }
    }
    for list in children.values_mut() {
        list.sort_unstable();
        list.dedup();
    }
    let mut leaves: Vec<&str> = by_id
        .keys()
        .copied()
        .filter(|id| !children.contains_key(id))
        .collect();
    leaves.sort_unstable();
    if leaves.len() <= 1 {
        return Ok(newest);
    }

    let empty = Entities::new();
    let entities_of = |id: &str| by_id.get(id).map_or(&empty, |node| &node.entities);
    let folder_of = |id: &str| by_id.get(id).map(|node| node.folder.clone());

    // Pairwise check at every fork, like drizzle-kit: leaves reached through
    // different children of a fork must not touch the same objects.
    let mut conflicts = Vec::new();
    let mut forks: Vec<(&str, &Vec<&str>)> = children
        .iter()
        .filter(|(_, kids)| kids.len() > 1)
        .map(|(parent, kids)| (*parent, kids))
        .collect();
    forks.sort_unstable();
    for (parent, kids) in forks {
        let base = entities_of(parent);
        let groups: Vec<Vec<&str>> = kids
            .iter()
            .map(|kid| reachable_leaves(&children, kid))
            .collect();
        for i in 0..groups.len() {
            for j in i + 1..groups.len() {
                if groups[i].iter().any(|leaf| groups[j].contains(leaf)) {
                    continue; // already merged below this fork
                }
                for left in &groups[i] {
                    for right in &groups[j] {
                        let left_changes = diff_entities(base, entities_of(left));
                        let right_changes = diff_entities(base, entities_of(right));
                        if let Some(reason) =
                            conflict_reason(dialect, base, &left_changes, &right_changes)
                        {
                            conflicts.push(BranchConflict {
                                parent: folder_of(parent),
                                left: folder_of(left).unwrap_or_default(),
                                right: folder_of(right).unwrap_or_default(),
                                reason,
                            });
                        }
                    }
                }
            }
        }
    }

    // Merge every leaf's changes onto the lowest common ancestor.
    let lca = lowest_common_ancestor(&by_id, &leaves);
    let base = lca.map_or(&empty, |id| entities_of(id));
    let mut merged = base.clone();
    let mut applied: BTreeMap<EntityKey, (Option<Value>, &str)> = BTreeMap::new();
    for leaf in &leaves {
        for (key, change) in diff_entities(base, entities_of(leaf)) {
            if let Some((previous, other)) = applied.get(&key) {
                // Inherited from a nested fork below the LCA: same change.
                if *previous != change {
                    conflicts.push(BranchConflict {
                        parent: lca.and_then(folder_of),
                        left: folder_of(other).unwrap_or_default(),
                        right: folder_of(leaf).unwrap_or_default(),
                        reason: format!("both change {}", describe(&key)),
                    });
                }
                continue;
            }
            match &change {
                Some(value) => merged.insert(key.clone(), value.clone()),
                None => merged.remove(&key),
            };
            applied.insert(key, (change, leaf));
        }
    }

    if !conflicts.is_empty() {
        conflicts.dedup();
        if ignore_conflicts {
            return Ok(newest);
        }
        return Err(HistoryError::Conflicts(conflicts));
    }

    Ok(MergeBase {
        snapshot: with_entities(&latest.snapshot, merged)?,
        prev_ids: Some(leaves.iter().map(ToString::to_string).collect()),
    })
}

/// Reads every `<folder>/snapshot.json` in `out_dir`, sorted by folder name.
fn load_nodes(out_dir: &Path, dialect: Dialect) -> Result<Vec<Node>, HistoryError> {
    if !out_dir.exists() {
        return Ok(Vec::new());
    }
    let mut folders: Vec<(String, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().join("snapshot.json");
        if folder != "meta" && path.is_file() {
            folders.push((folder, path));
        }
    }
    folders.sort();

    folders
        .into_iter()
        .map(|(folder, path)| {
            let snapshot = Snapshot::load(&path, dialect)?;
            let json = to_json(&snapshot)?;
            let prev_ids = json
                .get("prevIds")
                .and_then(Value::as_array)
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let entities = json
                .get("ddl")
                .and_then(Value::as_array)
                .map(|ddl| ddl.iter().map(|entity| (entity_key(entity), entity.clone())))
                .into_iter()
                .flatten()
                .collect();
            Ok(Node {
                folder,
                prev_ids,
                snapshot,
                entities,
            })
        })
        .collect()
}

fn to_json(snapshot: &Snapshot) -> Result<Value, std::io::Error> {
    match snapshot {
        Snapshot::Sqlite(inner) => serde_json::to_value(inner),
        Snapshot::Postgres(inner) => serde_json::to_value(inner),
        Snapshot::MySQL(inner) => serde_json::to_value(inner),
    }
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// `template` with its entities replaced by `entities`.
fn with_entities(template: &Snapshot, entities: Entities) -> Result<Snapshot, std::io::Error> {
    let mut json = to_json(template)?;
    json["ddl"] = Value::Array(entities.into_values().collect());
    let invalid = |error| std::io::Error::new(std::io::ErrorKind::InvalidData, error);
    Ok(match template {
        Snapshot::Sqlite(_) => Snapshot::Sqlite(serde_json::from_value(json).map_err(invalid)?),
        Snapshot::Postgres(_) => Snapshot::Postgres(serde_json::from_value(json).map_err(invalid)?),
        Snapshot::MySQL(_) => Snapshot::MySQL(serde_json::from_value(json).map_err(invalid)?),
    })
}

fn string_field(entity: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| entity.get(*name).and_then(Value::as_str))
        .map(str::to_string)
}

fn entity_key(entity: &Value) -> EntityKey {
    let kind = string_field(entity, &["entityType"]).unwrap_or_default();
    match string_field(entity, &["name"]) {
        Some(name) => (
            kind,
            string_field(entity, &["schema", "database"]),
            string_field(entity, &["table"]),
            name,
        ),
        None => (kind, None, None, entity.to_string()),
    }
}

fn table_key(schema: Option<String>, table: String) -> EntityKey {
    ("tables".to_string(), schema, None, table)
}

/// Tables and schemas an entity lives in or points at.
fn ancestors(entity: &Value) -> Vec<EntityKey> {
    let schema = string_field(entity, &["schema", "database"]);
    let mut keys = Vec::new();
    if let Some(table) = string_field(entity, &["table"]) {
        keys.push(table_key(schema.clone(), table));
    }
    if let Some(table) = string_field(entity, &["tableTo", "table_to"]) {
        keys.push(table_key(
            string_field(entity, &["schemaTo", "schema_to"]).or_else(|| schema.clone()),
            table,
        ));
    }
    if let Some(schema) = string_field(entity, &["schema"])
        && string_field(entity, &["entityType"]).as_deref() != Some("schemas")
    {
        keys.push(("schemas".to_string(), None, None, schema));
    }
    keys
}

fn diff_entities(from: &Entities, to: &Entities) -> Changes {
    let mut changes = Changes::new();
    for (key, value) in from {
        match to.get(key) {
            Some(new) if new == value => {}
            Some(new) => {
                changes.insert(key.clone(), Some(new.clone()));
            }
            None => {
                changes.insert(key.clone(), None);
            }
        }
    }
    for (key, value) in to {
        if !from.contains_key(key) {
            changes.insert(key.clone(), Some(value.clone()));
        }
    }
    changes
}

fn describe(key: &EntityKey) -> String {
    let (kind, schema, table, name) = key;
    let qualified = [schema.as_deref(), table.as_deref(), Some(name.as_str())]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(".");
    format!("{kind} `{qualified}`")
}

/// Tables a SQLite branch rebuilds (drizzle's `recreate_table`): constraint
/// changes, altered column definitions, or an altered table entity.
fn sqlite_rebuilt_tables(base: &Entities, changes: &Changes) -> HashSet<EntityKey> {
    let mut tables = HashSet::new();
    for (key, change) in changes {
        let kind = key.0.as_str();
        let altered = base.contains_key(key) && change.is_some();
        if kind == "tables" && altered {
            tables.insert(key.clone());
        }
        let constraint = matches!(kind, "pks" | "fks" | "uniques" | "checks");
        if (constraint || (kind == "columns" && altered))
            && let Some(table) = &key.2
        {
            tables.insert(table_key(key.1.clone(), table.clone()));
        }
    }
    tables
}

/// Why two branches' changes from the same parent cannot both apply.
fn conflict_reason(
    dialect: Dialect,
    base: &Entities,
    left: &Changes,
    right: &Changes,
) -> Option<String> {
    if let Some(key) = left.keys().find(|key| right.contains_key(*key)) {
        return Some(format!("both change {}", describe(key)));
    }
    let entity = |changes: &Changes, key: &EntityKey| {
        changes
            .get(key)
            .cloned()
            .flatten()
            .or_else(|| base.get(key).cloned())
    };
    for (changed, other) in [(left, right), (right, left)] {
        for key in changed.keys() {
            let Some(value) = entity(changed, key) else {
                continue;
            };
            for ancestor in ancestors(&value) {
                if matches!(other.get(&ancestor), Some(None)) {
                    return Some(format!(
                        "one changes {} while the other drops or renames {}",
                        describe(key),
                        describe(&ancestor)
                    ));
                }
            }
        }
        if dialect == Dialect::SQLite {
            let rebuilt = sqlite_rebuilt_tables(base, other);
            for key in changed.keys() {
                let touched = if key.0 == "tables" {
                    Some(key.clone())
                } else {
                    key.2.clone().map(|table| table_key(key.1.clone(), table))
                };
                if let Some(table) = touched.filter(|table| rebuilt.contains(table)) {
                    return Some(format!(
                        "one changes {} while the other rebuilds {}",
                        describe(key),
                        describe(&table)
                    ));
                }
            }
        }
    }
    None
}

/// Leaves reachable from `start` (inclusive).
fn reachable_leaves<'a>(children: &HashMap<&'a str, Vec<&'a str>>, start: &'a str) -> Vec<&'a str> {
    let mut leaves = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        match children.get(id) {
            Some(kids) => stack.extend(kids.iter().copied()),
            None => leaves.push(id),
        }
    }
    leaves.sort_unstable();
    leaves
}

/// Snapshots `id` follows, itself included (only snapshots in the folder).
fn ancestor_ids<'a>(by_id: &HashMap<&'a str, &'a Node>, id: &'a str) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![id];
    while let Some(current) = stack.pop() {
        let Some(node) = by_id.get(current) else {
            continue; // the origin sentinel, or a snapshot that is gone
        };
        if seen.insert(current) {
            stack.extend(
                node.prev_ids
                    .iter()
                    .map(String::as_str)
                    .filter(|prev| *prev != ORIGIN_UUID),
            );
        }
    }
    seen
}

/// The deepest snapshot every leaf follows, or `None` (empty schema).
fn lowest_common_ancestor<'a>(
    by_id: &HashMap<&'a str, &'a Node>,
    leaves: &[&'a str],
) -> Option<&'a str> {
    let mut common: Option<BTreeSet<&str>> = None;
    for leaf in leaves {
        let ancestors = ancestor_ids(by_id, leaf);
        common = Some(match common {
            None => ancestors,
            Some(common) => common.intersection(&ancestors).copied().collect(),
        });
    }
    let common = common?;
    let depth = |id: &'a str| ancestor_ids(by_id, id).len();
    // The common ancestor below every other one; on a diamond with no single
    // deepest one, the deepest candidate (as drizzle-kit falls back to).
    common
        .iter()
        .copied()
        .filter(|candidate| {
            let below = ancestor_ids(by_id, candidate);
            common.iter().all(|id| below.contains(id))
        })
        .max_by_key(|id| depth(id))
        .or_else(|| common.iter().copied().max_by_key(|id| depth(id)))
}

#[cfg(test)]
mod tests {
    use super::{HistoryError, load_merge_base};
    use crate::schema::Snapshot;
    use crate::version::ORIGIN_UUID;
    use drizzle_types::Dialect;
    use serde_json::json;
    use std::path::Path;

    fn table(name: &str) -> serde_json::Value {
        json!({"entityType": "tables", "name": name})
    }

    fn column(table: &str, name: &str, sql_type: &str) -> serde_json::Value {
        json!({
            "entityType": "columns", "table": table, "name": name, "type": sql_type,
            "notNull": false, "autoincrement": false, "default": null, "generated": null
        })
    }

    fn write(out: &Path, folder: &str, id: &str, prev: &[&str], ddl: serde_json::Value) {
        let dir = out.join(folder);
        std::fs::create_dir_all(&dir).expect("folder");
        std::fs::write(dir.join("migration.sql"), "").expect("sql");
        std::fs::write(
            dir.join("snapshot.json"),
            json!({"version": "7", "dialect": "sqlite", "id": id, "prevIds": prev, "ddl": ddl})
                .to_string(),
        )
        .expect("snapshot");
    }

    fn table_names(snapshot: &Snapshot) -> Vec<String> {
        let mut names: Vec<String> = snapshot
            .as_sqlite()
            .expect("sqlite")
            .ddl
            .iter()
            .filter_map(|entity| match entity {
                drizzle_types::sqlite::ddl::SqliteEntity::Table(table) => {
                    Some(table.name.to_string())
                }
                _ => None,
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn linear_history_uses_the_newest_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "20240101000000_a", "a", &[ORIGIN_UUID], json!([table("alpha")]));
        write(
            dir.path(),
            "20240102000000_b",
            "b",
            &["a"],
            json!([table("alpha"), table("beta")]),
        );

        let base = load_merge_base(dir.path(), Dialect::SQLite, false).expect("base");
        assert_eq!(base.snapshot.id(), "b");
        assert_eq!(base.prev_ids, None);
    }

    #[test]
    fn merged_branches_diff_against_the_union_of_their_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "20240101000000_init", "init", &[ORIGIN_UUID], json!([table("users")]));
        // Two branches off `init`, one of them two migrations long.
        write(
            dir.path(),
            "20240102000000_alpha",
            "alpha",
            &["init"],
            json!([table("users"), table("alpha")]),
        );
        write(
            dir.path(),
            "20240103000000_alpha_cols",
            "alpha2",
            &["alpha"],
            json!([table("users"), table("alpha"), column("alpha", "id", "integer")]),
        );
        write(
            dir.path(),
            "20240102000001_beta",
            "beta",
            &["init"],
            json!([table("users"), table("beta"), column("users", "email", "text")]),
        );

        let base = load_merge_base(dir.path(), Dialect::SQLite, false).expect("merge base");
        assert_eq!(base.prev_ids, Some(vec!["alpha2".to_string(), "beta".to_string()]));
        assert_eq!(table_names(&base.snapshot), ["alpha", "beta", "users"]);
        assert_eq!(base.snapshot.as_sqlite().expect("sqlite").ddl.len(), 5);
    }

    #[test]
    fn branches_from_an_empty_schema_merge() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "20240101000000_a", "a", &[ORIGIN_UUID], json!([table("alpha")]));
        write(dir.path(), "20240101000001_b", "b", &[ORIGIN_UUID], json!([table("beta")]));

        let base = load_merge_base(dir.path(), Dialect::SQLite, false).expect("merge base");
        assert_eq!(table_names(&base.snapshot), ["alpha", "beta"]);
        assert_eq!(base.prev_ids, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn conflicting_branches_are_rejected_unless_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "20240101000000_init", "init", &[ORIGIN_UUID], json!([table("users")]));
        write(
            dir.path(),
            "20240102000000_left",
            "left",
            &["init"],
            json!([table("users"), column("users", "email", "text")]),
        );
        write(
            dir.path(),
            "20240102000001_right",
            "right",
            &["init"],
            json!([table("users"), column("users", "email", "integer")]),
        );

        let error = load_merge_base(dir.path(), Dialect::SQLite, false).expect_err("conflict");
        let HistoryError::Conflicts(conflicts) = &error else {
            panic!("{error}");
        };
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].reason.contains("columns `users.email`"), "{error}");
        assert!(error.to_string().contains("20240102000000_left"), "{error}");

        let base = load_merge_base(dir.path(), Dialect::SQLite, true).expect("ignored");
        assert_eq!(base.snapshot.id(), "right");
        assert_eq!(base.prev_ids, None);
    }

    #[test]
    fn dropping_a_table_conflicts_with_changes_inside_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "20240101000000_init", "init", &[ORIGIN_UUID], json!([table("users")]));
        write(dir.path(), "20240102000000_drop", "drop", &["init"], json!([]));
        write(
            dir.path(),
            "20240102000001_add",
            "add",
            &["init"],
            json!([table("users"), column("users", "email", "text")]),
        );

        let error = load_merge_base(dir.path(), Dialect::SQLite, false).expect_err("conflict");
        assert!(error.to_string().contains("drops or renames tables `users`"), "{error}");
    }

    #[test]
    fn sqlite_rebuild_conflicts_with_a_column_added_on_the_other_branch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let users = json!([table("users"), column("users", "id", "integer")]);
        write(dir.path(), "20240101000000_init", "init", &[ORIGIN_UUID], users);
        // Changing a column definition rebuilds `users` from a copy that does
        // not know about the other branch's new column.
        write(
            dir.path(),
            "20240102000000_retype",
            "retype",
            &["init"],
            json!([table("users"), column("users", "id", "text")]),
        );
        write(
            dir.path(),
            "20240102000001_add",
            "add",
            &["init"],
            json!([table("users"), column("users", "id", "integer"), column("users", "note", "text")]),
        );

        let error = load_merge_base(dir.path(), Dialect::SQLite, false).expect_err("conflict");
        assert!(error.to_string().contains("rebuilds tables `users`"), "{error}");
    }

    #[test]
    fn an_already_merged_fork_is_not_merged_again() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "20240101000000_init", "init", &[ORIGIN_UUID], json!([]));
        write(dir.path(), "20240102000000_a", "a", &["init"], json!([table("alpha")]));
        write(dir.path(), "20240102000001_b", "b", &["init"], json!([table("beta")]));
        write(
            dir.path(),
            "20240103000000_merge",
            "merge",
            &["a", "b"],
            json!([table("alpha"), table("beta")]),
        );

        let base = load_merge_base(dir.path(), Dialect::SQLite, false).expect("base");
        assert_eq!(base.snapshot.id(), "merge");
        assert_eq!(base.prev_ids, None);
    }
}
