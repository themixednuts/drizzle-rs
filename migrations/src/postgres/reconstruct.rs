//! Reading drizzle-kit's format 9 and 10 `PostgreSQL` snapshots.
//!
//! From format 9 on, drizzle-kit stores the full schema only in the first
//! snapshot of a migrations folder (`"type": "base"`, with `ddl`). Every
//! later snapshot (`"type": "diff"`) stores the `deltas` that turn the state
//! of its first parent into its own state. Format 10 is format 9 without the
//! `concurrently` field of indexes.
//!
//! This module follows drizzle-kit's `dialects/postgres/reconstruct.ts` and
//! `upToV10`: it applies the deltas along the first parents from the base
//! snapshot, and turns each snapshot into the full-state shape that the rest
//! of this crate reads ([`PostgresSnapshot`]). A rename moves the entities
//! that name the renamed one (the columns of a renamed table, the indexes of
//! a renamed column, ...) the way drizzle-kit's DDL edges do, and a merge
//! applies the deltas that only its other parents reach, in folder-name
//! order.

use super::PostgresSnapshot;
use crate::version::{ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

type Row = Map<String, Value>;

/// The snapshot formats that store base and diff snapshots.
const DELTA_VERSIONS: [&str; 2] = ["9", "10"];

/// Whether `json` is a drizzle-kit format 9 or 10 `PostgreSQL` snapshot,
/// which [`reconstruct`] and [`reconstruct_folder`] read.
#[must_use]
pub fn is_delta_snapshot(json: &Value) -> bool {
    json.get("version")
        .and_then(Value::as_str)
        .is_some_and(|version| DELTA_VERSIONS.contains(&version))
        && matches!(
            json.get("type").and_then(Value::as_str),
            Some("base" | "diff")
        )
}

/// The full state of snapshot `id` of the migrations folder `out_dir`.
///
/// `out_dir` holds `<folder>/snapshot.json` files of format 9 or 10, as
/// drizzle-kit writes them. Snapshots of other formats in the folder are
/// ignored.
///
/// # Errors
///
/// Returns an [`io::Error`] when the folder cannot be read, a snapshot is
/// not valid JSON, or the snapshot does not reconstruct (see
/// [`reconstruct`]).
pub fn reconstruct_folder(out_dir: &Path, id: &str) -> io::Result<PostgresSnapshot> {
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path().join("snapshot.json");
        if entry.file_type()?.is_dir() && path.is_file() {
            folders.push((entry.file_name(), path));
        }
    }
    folders.sort();

    let mut snapshots = Vec::new();
    for (_, path) in folders {
        let json: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)
            .map_err(|e| invalid(format!("{}: {e}", path.display())))?;
        if is_delta_snapshot(&json) {
            snapshots.push(json);
        }
    }
    reconstruct(&snapshots, id)
}

/// The full state of snapshot `id` of `snapshots`, the format 9 or 10
/// snapshots of one migrations folder in folder-name order (a merge applies
/// its other branches in that order). A base snapshot reconstructs on its
/// own.
///
/// # Errors
///
/// Returns an [`io::Error`] of kind [`io::ErrorKind::InvalidData`] when
/// `id` or one of its ancestors is missing, its history does not reach a
/// base snapshot, a delta gives one key to two different entities, or the
/// state holds entities drizzle-rs does not support.
pub fn reconstruct(snapshots: &[Value], id: &str) -> io::Result<PostgresSnapshot> {
    let history = History::new(snapshots)?;
    let state = history.reconstruct(id, &mut HashMap::new())?;
    let snapshot = history.lookup(id)?;
    serde_json::from_value(full_snapshot(snapshot, state))
        .map_err(|e| invalid(format!("snapshot {id} does not convert to a snapshot: {e}")))
}

/// Reads the snapshots of one migrations folder, `paths` in folder-name
/// order, and turns every format 9 or 10 snapshot into the full-state shape.
///
/// # Errors
///
/// Returns an [`io::Error`] when a file cannot be read, is not valid JSON,
/// has a format this crate does not read, or does not reconstruct (see
/// [`reconstruct_folder`]).
pub(crate) fn load_snapshots(paths: &[PathBuf]) -> io::Result<Vec<PostgresSnapshot>> {
    let mut snapshots = Vec::with_capacity(paths.len());
    for path in paths {
        let json: Value = serde_json::from_str(&std::fs::read_to_string(path)?)
            .map_err(|e| invalid(format!("{}: {e}", path.display())))?;
        check_version(&json).map_err(|e| invalid(format!("{}: {e}", path.display())))?;
        snapshots.push(json);
    }
    to_full_snapshots(snapshots)?
        .into_iter()
        .zip(paths)
        .map(|(json, path)| {
            serde_json::from_value(json).map_err(|e| invalid(format!("{}: {e}", path.display())))
        })
        .collect()
}

/// Accepts the current full-state format and drizzle-kit's formats 9 and
/// 10, and explains every other version.
fn check_version(json: &Value) -> Result<(), String> {
    let Some(version) = json.get("version").and_then(Value::as_str) else {
        return Err("the snapshot has no \"version\"".into());
    };
    if version == POSTGRES_SNAPSHOT_VERSION || is_delta_snapshot(json) {
        return Ok(());
    }
    let newest: u32 = DELTA_VERSIONS[DELTA_VERSIONS.len() - 1]
        .parse()
        .expect("a version number");
    if version.parse::<u32>().is_ok_and(|v| v > newest) {
        Err(format!(
            "snapshot format {version} is newer than drizzle-rs reads (up to {newest}); \
             update drizzle-rs"
        ))
    } else {
        Err(format!(
            "snapshot format {version} is outdated; run `drizzle up` to upgrade the folder"
        ))
    }
}

/// Turns every format 9 or 10 snapshot of `snapshots` into the full-state
/// shape of [`PostgresSnapshot`], and keeps the others as they are.
///
/// `snapshots` must be the snapshots of one migrations folder in folder-name
/// order, because a merge applies its other branches in that order.
fn to_full_snapshots(snapshots: Vec<Value>) -> io::Result<Vec<Value>> {
    let deltas: Vec<Value> = snapshots
        .iter()
        .filter(|json| is_delta_snapshot(json))
        .cloned()
        .collect();
    if deltas.is_empty() {
        return Ok(snapshots);
    }
    let history = History::new(&deltas)?;
    let mut states = HashMap::new();
    snapshots
        .into_iter()
        .map(|json| {
            if !is_delta_snapshot(&json) {
                return Ok(json);
            }
            let id = json.get("id").and_then(Value::as_str).unwrap_or_default();
            let state = history.reconstruct(id, &mut states)?;
            Ok(full_snapshot(&json, state))
        })
        .collect()
}

/// drizzle-kit's order of entity types, which its snapshots list entities
/// in.
const ENTITY_TYPES: [&str; 14] = [
    "schemas",
    "tables",
    "enums",
    "columns",
    "indexes",
    "fks",
    "pks",
    "uniques",
    "checks",
    "sequences",
    "roles",
    "privileges",
    "policies",
    "views",
];

/// The full-state document of `snapshot` with the entities `state`, listed
/// by entity type like drizzle-kit lists them.
fn full_snapshot(snapshot: &Value, state: State) -> Value {
    let mut rows = state.rows;
    rows.sort_by_key(|(_, row)| {
        let entity_type = row.get("entityType").and_then(Value::as_str);
        ENTITY_TYPES
            .iter()
            .position(|known| Some(*known) == entity_type)
    });
    let ddl: Vec<Value> = rows
        .into_iter()
        .map(|(_, row)| Value::Object(row))
        .collect();
    let mut full = serde_json::json!({
        "version": POSTGRES_SNAPSHOT_VERSION,
        "dialect": "postgresql",
        "id": snapshot.get("id").cloned().unwrap_or(Value::Null),
        "prevIds": snapshot.get("prevIds").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "ddl": ddl,
        "renames": [],
    });
    crate::upgrade::patch_postgres_required_nulls(&mut full);
    full
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

// =============================================================================
// Snapshot history
// =============================================================================

/// The format 10 snapshots of a folder by id, in folder-name order.
struct History {
    order: Vec<String>,
    by_id: HashMap<String, Value>,
}

impl History {
    fn new(snapshots: &[Value]) -> io::Result<Self> {
        let mut order = Vec::new();
        let mut by_id = HashMap::new();
        for json in snapshots {
            let id = json
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("a snapshot has no \"id\""))?
                .to_string();
            // drizzle-kit keeps the last file of a duplicated id.
            if by_id.insert(id.clone(), up_to_v10(json.clone())).is_none() {
                order.push(id);
            }
        }
        Ok(Self { order, by_id })
    }

    fn lookup(&self, id: &str) -> io::Result<&Value> {
        self.by_id.get(id).ok_or_else(|| {
            invalid(format!(
                "Cannot reconstruct snapshot {id}: it is not in the migrations folder. \
                 Check the snapshot id, or restore the migration folder from git."
            ))
        })
    }

    /// `ids` and all their ancestors, stopping at the ids in `known`.
    fn history_of(&self, ids: &[String], known: &HashSet<String>) -> io::Result<HashSet<String>> {
        let mut found = HashSet::new();
        let mut stack = ids.to_vec();
        while let Some(id) = stack.pop() {
            if id == ORIGIN_UUID || found.contains(&id) || known.contains(&id) {
                continue;
            }
            stack.extend(prev_ids(self.lookup(&id)?));
            found.insert(id);
        }
        Ok(found)
    }

    /// The state after snapshot `id`. `states` keeps the state of every
    /// snapshot reconstructed so far, so that the snapshots of a folder are
    /// each reconstructed once.
    fn reconstruct(&self, id: &str, states: &mut HashMap<String, State>) -> io::Result<State> {
        // The diff snapshots on the path of first parents from `id` back to a
        // known state or to the base snapshot.
        let mut diffs = Vec::new();
        let mut current = id.to_string();
        let mut state = loop {
            if let Some(state) = states.get(&current) {
                break state.clone();
            }
            let snapshot = self.lookup(&current)?;
            if !is_diff(snapshot) {
                let mut state = State::default();
                for row in list(snapshot, "ddl") {
                    state.create(row, snapshot)?;
                }
                states.insert(current.clone(), state.clone());
                break state;
            }
            diffs.push(snapshot);
            current = match prev_ids(snapshot).into_iter().next() {
                Some(parent) if parent != ORIGIN_UUID => parent,
                _ => {
                    return Err(invalid(format!(
                        "Cannot reconstruct snapshot {id}: its history does not reach a base \
                         snapshot. Restore the first migration folder from git."
                    )));
                }
            };
        };
        for diff in diffs.into_iter().rev() {
            let parents = prev_ids(diff);
            if parents.len() > 1 {
                self.apply_other_parents(&mut state, &parents)?;
            }
            state.apply(diff)?;
            states.insert(id_of(diff).to_string(), state.clone());
        }
        Ok(state)
    }

    /// Applies the diff snapshots that only the parents after the first
    /// reach, in folder-name order.
    fn apply_other_parents(&self, state: &mut State, parents: &[String]) -> io::Result<()> {
        let (first, others) = parents.split_first().expect("a merge has parents");
        let first_history = self.history_of(std::slice::from_ref(first), &HashSet::new())?;
        let rest = self.history_of(others, &first_history)?;
        for id in &self.order {
            let snapshot = &self.by_id[id];
            if rest.contains(id) && is_diff(snapshot) {
                state.apply(snapshot)?;
            }
        }
        Ok(())
    }
}

fn is_diff(snapshot: &Value) -> bool {
    snapshot.get("type").and_then(Value::as_str) == Some("diff")
}

fn prev_ids(snapshot: &Value) -> Vec<String> {
    snapshot
        .get("prevIds")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn list<'v>(snapshot: &'v Value, field: &str) -> impl Iterator<Item = &'v Value> {
    snapshot
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// drizzle-kit's `upToV10`: format 10 has no `concurrently` on indexes. An
/// alter delta whose only field was `concurrently` changed no SQL, so it is
/// dropped.
fn up_to_v10(mut snapshot: Value) -> Value {
    if snapshot.get("version").and_then(Value::as_str) != Some("9") {
        return snapshot;
    }
    let is_index = |row: &Value| row.get("entityType").and_then(Value::as_str) == Some("indexes");
    if let Some(ddl) = snapshot.get_mut("ddl").and_then(Value::as_array_mut) {
        for row in ddl.iter_mut().filter(|row| is_index(row)) {
            if let Some(row) = row.as_object_mut() {
                row.remove("concurrently");
            }
        }
    }
    if let Some(deltas) = snapshot.get_mut("deltas").and_then(Value::as_array_mut) {
        deltas.retain_mut(|delta| match delta.get("op").and_then(Value::as_str) {
            Some("create") => {
                if let Some(row) = delta
                    .get_mut("row")
                    .filter(|row| is_index(row))
                    .and_then(Value::as_object_mut)
                {
                    row.remove("concurrently");
                }
                true
            }
            Some("alter") if entity_type_of_key(delta).as_deref() == Some("indexes") => {
                let Some(set) = delta.get_mut("set").and_then(Value::as_object_mut) else {
                    return true;
                };
                set.remove("concurrently");
                !set.is_empty()
            }
            _ => true,
        });
    }
    snapshot["version"] = Value::String("10".into());
    snapshot
}

fn entity_type_of_key(delta: &Value) -> Option<String> {
    let key = delta.get("key")?.as_str()?;
    Some(key.split_once('\0')?.0.to_string())
}

// =============================================================================
// Entity state
// =============================================================================

/// The entity types this crate reads, with drizzle-kit's identity fields.
/// The identity of an entity is the JSON array of these fields, and its key
/// is `<entityType>\0<identity>`.
fn identity_fields(entity_type: &str) -> Option<&'static [&'static str]> {
    Some(match entity_type {
        "schemas" | "roles" => &["name"],
        "tables" | "enums" | "indexes" | "sequences" | "views" => &["schema", "name"],
        "columns" | "fks" | "pks" | "uniques" | "checks" | "policies" => {
            &["schema", "table", "name"]
        }
        "privileges" => &["schema", "table", "grantor", "grantee", "type"],
        _ => return None,
    })
}

fn identity(entity_type: &str, row: &Row) -> Option<String> {
    let fields = identity_fields(entity_type)?;
    let values: Vec<Value> = fields
        .iter()
        .map(|field| row.get(*field).cloned().unwrap_or(Value::Null))
        .collect();
    Some(Value::Array(values).to_string())
}

/// Where an edge reads the name of the entity it points at.
enum Source {
    /// A scalar field of the row.
    Field(&'static str),
    /// The elements of a list field, or a field of each element (`pick`).
    /// Elements with a true `skip_when` field point at nothing.
    List {
        list: &'static str,
        pick: Option<&'static str>,
        skip_when: Option<&'static str>,
    },
}

/// A field of a row that names another entity: the entity of type `to`
/// whose identity fields (the first item of each `map` pair) hold the values
/// of the row's fields.
struct Edge {
    from: &'static str,
    to: &'static str,
    map: &'static [(&'static str, Source)],
    when: Option<fn(&Row) -> bool>,
}

const fn field(name: &'static str) -> Source {
    Source::Field(name)
}

const fn names(list: &'static str) -> Source {
    Source::List {
        list,
        pick: None,
        skip_when: None,
    }
}

fn names_a_user_type(row: &Row) -> bool {
    row.get("typeSchema")
        .and_then(Value::as_str)
        .is_some_and(|schema| !schema.is_empty() && schema != "pg_catalog")
}

/// drizzle-kit's `edges` of the `PostgreSQL` DDL.
static EDGES: &[Edge] = &[
    Edge {
        from: "tables",
        to: "schemas",
        map: &[("name", field("schema"))],
        when: None,
    },
    Edge {
        from: "enums",
        to: "schemas",
        map: &[("name", field("schema"))],
        when: None,
    },
    Edge {
        from: "sequences",
        to: "schemas",
        map: &[("name", field("schema"))],
        when: None,
    },
    Edge {
        from: "views",
        to: "schemas",
        map: &[("name", field("schema"))],
        when: None,
    },
    Edge {
        from: "checks",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "policies",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "policies",
        to: "roles",
        map: &[("name", names("roles"))],
        when: None,
    },
    Edge {
        from: "privileges",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "privileges",
        to: "roles",
        map: &[("name", field("grantee"))],
        when: None,
    },
    Edge {
        from: "privileges",
        to: "roles",
        map: &[("name", field("grantor"))],
        when: None,
    },
    Edge {
        from: "columns",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "columns",
        to: "enums",
        map: &[("schema", field("typeSchema")), ("name", field("type"))],
        when: Some(names_a_user_type),
    },
    Edge {
        from: "indexes",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "indexes",
        to: "columns",
        map: &[
            ("schema", field("schema")),
            ("table", field("table")),
            (
                "name",
                Source::List {
                    list: "columns",
                    pick: Some("value"),
                    skip_when: Some("isExpression"),
                },
            ),
        ],
        when: None,
    },
    Edge {
        from: "pks",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "pks",
        to: "columns",
        map: &[
            ("schema", field("schema")),
            ("table", field("table")),
            ("name", names("columns")),
        ],
        when: None,
    },
    Edge {
        from: "uniques",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "uniques",
        to: "columns",
        map: &[
            ("schema", field("schema")),
            ("table", field("table")),
            ("name", names("columns")),
        ],
        when: None,
    },
    Edge {
        from: "fks",
        to: "tables",
        map: &[("schema", field("schema")), ("name", field("table"))],
        when: None,
    },
    Edge {
        from: "fks",
        to: "columns",
        map: &[
            ("schema", field("schema")),
            ("table", field("table")),
            ("name", names("columns")),
        ],
        when: None,
    },
    Edge {
        from: "fks",
        to: "tables",
        map: &[("schema", field("schemaTo")), ("name", field("tableTo"))],
        when: None,
    },
    Edge {
        from: "fks",
        to: "columns",
        map: &[
            ("schema", field("schemaTo")),
            ("table", field("tableTo")),
            ("name", names("columnsTo")),
        ],
        when: None,
    },
];

impl Edge {
    fn scalars(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.map.iter().filter_map(|(target, source)| match source {
            Source::Field(field) => Some((*target, *field)),
            Source::List { .. } => None,
        })
    }

    fn list(&self) -> Option<(&'static str, &Source)> {
        self.map
            .iter()
            .find(|(_, source)| matches!(source, Source::List { .. }))
            .map(|(target, source)| (*target, source))
    }

    fn applies_to(&self, row: &Row) -> bool {
        self.when.is_none_or(|when| when(row))
    }

    /// The keys of the entities `row` points at through this edge.
    fn targets(&self, row: &Row) -> Vec<String> {
        if !self.applies_to(row) {
            return Vec::new();
        }
        let mut target = Row::new();
        for (target_field, source_field) in self.scalars() {
            target.insert(target_field.into(), get(row, source_field));
        }
        let Some((
            target_field,
            Source::List {
                list,
                pick,
                skip_when,
            },
        )) = self.list()
        else {
            return identity(self.to, &target)
                .map(|id| vec![qualified(self.to, &id)])
                .unwrap_or_default();
        };
        let Some(elements) = row.get(*list).and_then(Value::as_array) else {
            return Vec::new();
        };
        elements
            .iter()
            .filter(|element| !skip_when.is_some_and(|skip| is_truthy(element.get(skip))))
            .filter_map(|element| {
                let name = match pick {
                    Some(pick) => element.get(*pick)?,
                    None => element,
                };
                name.as_str()?;
                let mut target = target.clone();
                target.insert(target_field.into(), name.clone());
                identity(self.to, &target).map(|id| qualified(self.to, &id))
            })
            .collect()
    }

    /// The fields that point `row` at `to` instead of `from`, or `None` when
    /// `row` does not point at `from` through this edge.
    fn relocate(&self, row: &Row, from: &Row, to: &Row) -> Option<Row> {
        if !self.applies_to(row) {
            return None;
        }
        if self
            .scalars()
            .any(|(target_field, source_field)| get(row, source_field) != get(from, target_field))
        {
            return None;
        }
        let mut patch = Row::new();
        for (target_field, source_field) in self.scalars() {
            patch.insert(source_field.into(), get(to, target_field));
        }
        let Some((
            target_field,
            Source::List {
                list,
                pick,
                skip_when,
            },
        )) = self.list()
        else {
            return Some(patch);
        };
        let elements = row.get(*list)?.as_array()?;
        let old_name = get(from, target_field);
        let new_name = get(to, target_field);
        let mut changed = false;
        let next: Vec<Value> = elements
            .iter()
            .map(|element| {
                if skip_when.is_some_and(|skip| is_truthy(element.get(skip))) {
                    return element.clone();
                }
                let name = match pick {
                    Some(pick) => element.get(*pick).cloned().unwrap_or(Value::Null),
                    None => element.clone(),
                };
                if name != old_name {
                    return element.clone();
                }
                changed = true;
                match (pick, element) {
                    (Some(pick), Value::Object(object)) => {
                        let mut object = object.clone();
                        object.insert((*pick).into(), new_name.clone());
                        Value::Object(object)
                    }
                    _ => new_name.clone(),
                }
            })
            .collect();
        if !changed {
            return None;
        }
        patch.insert((*list).into(), Value::Array(next));
        Some(patch)
    }
}

fn get(row: &Row, field: &str) -> Value {
    row.get(field).cloned().unwrap_or(Value::Null)
}

fn is_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

fn qualified(entity_type: &str, identity: &str) -> String {
    format!("{entity_type}\0{identity}")
}

/// The entities of a schema state with their keys, in creation order.
#[derive(Default, Clone)]
struct State {
    rows: Vec<(String, Row)>,
}

impl State {
    fn key_of(row: &Row) -> io::Result<String> {
        let entity_type = row
            .get("entityType")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let identity = identity(entity_type, row).ok_or_else(|| {
            invalid(format!(
                "drizzle-rs does not support \"{entity_type}\" entities of drizzle-kit snapshots"
            ))
        })?;
        Ok(qualified(entity_type, &identity))
    }

    fn position(&self, key: &str) -> Option<usize> {
        self.rows.iter().position(|(row_key, _)| row_key == key)
    }

    fn apply(&mut self, snapshot: &Value) -> io::Result<()> {
        for delta in list(snapshot, "deltas") {
            let key = || {
                delta
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("a delta has no \"key\""))
            };
            match delta.get("op").and_then(Value::as_str) {
                Some("create") => {
                    let row = delta.get("row").unwrap_or(&Value::Null);
                    self.create(row, snapshot)?;
                }
                Some("drop") => {
                    if let Some(index) = self.position(key()?) {
                        self.rows.remove(index);
                    }
                }
                Some(op @ ("alter" | "rename")) => {
                    let set = delta
                        .get("set")
                        .and_then(Value::as_object)
                        .ok_or_else(|| invalid(format!("an {op} delta has no \"set\"")))?;
                    self.update(key()?, set, op)
                        .map_err(|e| collision(e, snapshot))?;
                }
                other => {
                    return Err(invalid(format!(
                        "snapshot {} has a delta with an unknown op {other:?}",
                        id_of(snapshot)
                    )));
                }
            }
        }
        Ok(())
    }

    /// Adds `row`. A create of an equal entity does nothing; another entity
    /// under the same key is a collision.
    fn create(&mut self, row: &Value, snapshot: &Value) -> io::Result<()> {
        let row = row.as_object().ok_or_else(|| {
            invalid(format!(
                "snapshot {} has a row that is not an object",
                id_of(snapshot)
            ))
        })?;
        let key = Self::key_of(row)?;
        match self.position(&key) {
            None => self.rows.push((key, row.clone())),
            Some(index) => {
                let stored = &self.rows[index].1;
                let equal = stored
                    .keys()
                    .chain(row.keys())
                    .all(|field| get(stored, field) == get(row, field));
                if !equal {
                    return Err(collision(
                        invalid(format!(
                            "The create delta adds {}, and another entity with other values \
                             already has this key.",
                            describe(&key)
                        )),
                        snapshot,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Sets the fields of the entity under `key`, and moves the entities
    /// that point at it when its key changes. A missing entity does nothing.
    fn update(&mut self, key: &str, set: &Row, op: &str) -> io::Result<()> {
        let Some(index) = self.position(key) else {
            return Ok(());
        };
        let before = self.rows.clone();
        let old_row = self.rows[index].1.clone();
        for (field, value) in set {
            self.rows[index].1.insert(field.clone(), value.clone());
        }
        let new_key = Self::key_of(&self.rows[index].1)?;
        if new_key == key {
            return Ok(());
        }
        self.rows[index].0.clone_from(&new_key);

        // Which rows point at which keys, read before any dependent moves.
        let mut dependents: HashMap<String, Vec<(usize, &Edge)>> = HashMap::new();
        for (row_index, (_, row)) in self.rows.iter().enumerate() {
            let entity_type = row
                .get("entityType")
                .and_then(Value::as_str)
                .unwrap_or_default();
            for edge in EDGES.iter().filter(|edge| edge.from == entity_type) {
                for target in edge.targets(row) {
                    dependents
                        .entry(target)
                        .or_default()
                        .push((row_index, edge));
                }
            }
        }

        let mut visited: HashSet<String> = HashSet::from([key.to_string()]);
        let mut queue = vec![(key.to_string(), old_row, self.rows[index].1.clone())];
        let mut next = 0;
        while let Some((old_key, from, to)) = queue.get(next).cloned() {
            next += 1;
            for &(row_index, edge) in dependents.get(&old_key).into_iter().flatten() {
                let (dependent_key, dependent) = &mut self.rows[row_index];
                let Some(patch) = edge.relocate(dependent, &from, &to) else {
                    continue;
                };
                let dependent_old = dependent.clone();
                dependent.extend(patch);
                let moved_key = Self::key_of(dependent)?;
                if moved_key != *dependent_key {
                    let old_key = std::mem::replace(dependent_key, moved_key);
                    if visited.insert(old_key.clone()) {
                        queue.push((old_key, dependent_old, dependent.clone()));
                    }
                }
            }
        }

        let mut held = HashSet::new();
        let taken = self
            .rows
            .iter()
            .find(|(row_key, _)| !held.insert(row_key))
            .map(|(row_key, _)| describe(row_key));
        if let Some(taken) = taken {
            self.rows = before;
            return Err(invalid(format!(
                "The {op} delta moves {} to {}, and another entity already has the key {taken}.",
                describe(key),
                describe(&new_key),
            )));
        }
        Ok(())
    }
}

fn id_of(snapshot: &Value) -> &str {
    snapshot.get("id").and_then(Value::as_str).unwrap_or("?")
}

/// `<entityType> <identity>` for messages.
fn describe(key: &str) -> String {
    key.replacen('\0', " ", 1)
}

fn collision(error: io::Error, snapshot: &Value) -> io::Error {
    invalid(format!(
        "{error} Two migrations gave one name to two different entities (snapshot {}).",
        id_of(snapshot)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base(id: &str, ddl: Value) -> Value {
        json!({
            "type": "base", "version": "10", "dialect": "postgres", "id": id,
            "prevIds": [ORIGIN_UUID], "footprints": [], "ddl": ddl,
        })
    }

    fn diff(id: &str, prev_ids: &[&str], deltas: Value) -> Value {
        json!({
            "type": "diff", "version": "10", "dialect": "postgres", "id": id,
            "prevIds": prev_ids, "footprints": [], "deltas": deltas,
        })
    }

    fn table(name: &str) -> Value {
        json!({"entityType": "tables", "schema": "public", "name": name, "isRlsEnabled": false})
    }

    fn column(table: &str, name: &str) -> Value {
        json!({
            "entityType": "columns", "schema": "public", "table": table, "name": name,
            "type": "integer", "typeSchema": null, "notNull": false, "dimensions": 0,
            "default": null, "generated": null, "identity": null,
        })
    }

    fn names_of(snapshot: &Value, entity_type: &str) -> Vec<String> {
        let mut names: Vec<String> = snapshot["ddl"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["entityType"] == entity_type)
            .map(|row| match row.get("table").and_then(Value::as_str) {
                Some(table) => format!("{table}.{}", row["name"].as_str().unwrap()),
                None => row["name"].as_str().unwrap().to_string(),
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_table_rename_moves_its_columns_and_their_index() {
        let index = json!({
            "entityType": "indexes", "schema": "public", "table": "users", "name": "users_name_idx",
            "columns": [{"value": "name", "isExpression": false}], "concurrently": true,
        });
        let snapshots = vec![
            json!({
                "type": "base", "version": "9", "dialect": "postgres", "id": "a",
                "prevIds": [ORIGIN_UUID], "footprints": [],
                "ddl": [table("users"), column("users", "name"), index],
            }),
            diff(
                "b",
                &["a"],
                json!([
                    {"op": "rename", "key": "tables\u{0}[\"public\",\"users\"]", "set": {"name": "accounts"}},
                    {"op": "rename", "key": "columns\u{0}[\"public\",\"accounts\",\"name\"]", "set": {"name": "full_name"}},
                ]),
            ),
        ];

        let full = to_full_snapshots(snapshots).expect("reconstructs");

        let last = &full[1];
        assert_eq!(last["version"], POSTGRES_SNAPSHOT_VERSION);
        assert_eq!(names_of(last, "columns"), ["accounts.full_name"]);
        let index = last["ddl"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["entityType"] == "indexes")
            .unwrap();
        assert_eq!(index["table"], "accounts");
        assert_eq!(index["columns"][0]["value"], "full_name");
        assert!(
            index.get("concurrently").is_none(),
            "format 10 drops concurrently"
        );
    }

    #[test]
    fn a_merge_applies_the_other_branch() {
        let snapshots = vec![
            base("a", json!([table("users")])),
            diff(
                "b",
                &["a"],
                json!([{"op": "create", "row": column("users", "left")}]),
            ),
            diff(
                "c",
                &["a"],
                json!([{"op": "create", "row": table("posts")}]),
            ),
            diff(
                "d",
                &["c", "b"],
                json!([{"op": "drop", "key": "columns\u{0}[\"public\",\"users\",\"left\"]"}]),
            ),
        ];

        let full = to_full_snapshots(snapshots).expect("reconstructs");

        assert_eq!(names_of(&full[1], "tables"), ["users"]);
        assert_eq!(names_of(&full[3], "tables"), ["posts", "users"]);
        assert!(names_of(&full[3], "columns").is_empty());
    }

    #[test]
    fn a_rename_onto_a_taken_key_is_an_error() {
        let snapshots = vec![
            base("a", json!([table("users"), table("accounts")])),
            diff(
                "b",
                &["a"],
                json!([{"op": "rename", "key": "tables\u{0}[\"public\",\"users\"]", "set": {"name": "accounts"}}]),
            ),
        ];

        let error = to_full_snapshots(snapshots).expect_err("collision");

        assert!(
            error
                .to_string()
                .contains("another entity already has the key"),
            "{error}"
        );
    }

    #[test]
    fn a_diff_without_a_base_is_an_error() {
        let snapshots = vec![diff("b", &[ORIGIN_UUID], json!([]))];

        let error = to_full_snapshots(snapshots).expect_err("no base");

        assert!(
            error.to_string().contains("does not reach a base snapshot"),
            "{error}"
        );
    }

    #[test]
    fn unsupported_entities_are_an_error() {
        let procedure = json!({
            "entityType": "procedures", "schema": "public", "name": "f",
            "params": "", "body": "", "language": "sql",
        });

        let error = to_full_snapshots(vec![base("a", json!([procedure]))]).expect_err("procedures");

        assert!(error.to_string().contains("\"procedures\""), "{error}");
    }
}
