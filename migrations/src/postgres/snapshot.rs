//! PostgreSQL snapshot types matching drizzle-kit format

use crate::postgres::ddl::PostgresEntity;
use crate::postgres::grammar::{extract_nextval_sequence, is_serial_expression};
use crate::version::{ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// PostgreSQL schema snapshot (version 8 - drizzle-kit beta)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSnapshot {
    pub version: String,
    pub dialect: String,
    pub id: String,
    pub prev_ids: Vec<String>,
    pub ddl: Vec<PostgresEntity>,
    /// Renames tracking (for table/column renames between migrations)
    #[serde(default)]
    pub renames: Vec<String>,
}

impl Default for PostgresSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresSnapshot {
    pub fn new() -> Self {
        Self {
            version: POSTGRES_SNAPSHOT_VERSION.to_string(),
            dialect: "postgres".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            prev_ids: vec![ORIGIN_UUID.to_string()],
            ddl: Vec::new(),
            renames: Vec::new(),
        }
    }

    pub fn with_prev_ids(prev_ids: Vec<String>) -> Self {
        let mut snapshot = Self::new();
        snapshot.prev_ids = prev_ids;
        snapshot
    }

    pub fn add_entity(&mut self, entity: PostgresEntity) {
        self.ddl.push(entity);
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Return a new snapshot scoped to only the given tables.
    ///
    /// - Schema entities are kept only if referenced by a desired table.
    /// - Table-scoped entities (Column, Index, FK, PK, Unique, Check,
    ///   Policy) are kept only when their parent table is in the set.
    /// - Other global entities (Enum, Sequence, Role, View) pass through.
    ///
    /// The set contains `(schema, table_name)` pairs.
    pub fn scoped_to_tables(&self, tables: &HashSet<(String, String)>) -> Self {
        // Derive the set of schema names referenced by desired tables
        let schemas: HashSet<&str> = tables.iter().map(|(s, _)| s.as_str()).collect();

        let mut scoped = Self::new();
        for entity in &self.ddl {
            match entity {
                // Schema entities — keep only if referenced by desired tables
                PostgresEntity::Schema(s) => {
                    if schemas.contains(s.name.as_ref()) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                // Table-scoped entities — keep only if table is in desired set
                PostgresEntity::Table(t) => {
                    if tables.contains(&(t.schema.to_string(), t.name.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::Column(c) => {
                    if tables.contains(&(c.schema.to_string(), c.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::Index(i) => {
                    if tables.contains(&(i.schema.to_string(), i.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::ForeignKey(f) => {
                    if tables.contains(&(f.schema.to_string(), f.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::PrimaryKey(p) => {
                    if tables.contains(&(p.schema.to_string(), p.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::UniqueConstraint(u) => {
                    if tables.contains(&(u.schema.to_string(), u.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::CheckConstraint(c) => {
                    if tables.contains(&(c.schema.to_string(), c.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::Policy(p) => {
                    if tables.contains(&(p.schema.to_string(), p.table.to_string())) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                // Schema-scoped global entities — keep only if in relevant schemas
                PostgresEntity::Sequence(s) => {
                    if schemas.contains(s.schema.as_ref()) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::Enum(e) => {
                    if schemas.contains(e.schema.as_ref()) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                PostgresEntity::View(v) => {
                    if schemas.contains(v.schema.as_ref()) {
                        scoped.ddl.push(entity.clone());
                    }
                }
                // Truly global entities (Role, Privilege)
                _ => scoped.ddl.push(entity.clone()),
            }
        }
        scoped
    }

    /// Remove sequences that are owned by serial/bigserial columns.
    ///
    /// Serial columns auto-create sequences in PostgreSQL. These should not
    /// appear in snapshots used for diffing, otherwise the diff engine will
    /// try to DROP them (breaking the serial column) or CREATE duplicates.
    pub fn filter_serial_sequences(&mut self) {
        // Collect (schema, seq_name) pairs referenced by serial column defaults
        let serial_seqs: HashSet<(String, String)> = self
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Column(c) = e {
                    let default = c.default.as_deref()?;
                    if is_serial_expression(default, &c.schema) {
                        let name = extract_nextval_sequence(default)?;
                        return Some((c.schema.to_string(), name));
                    }
                }
                None
            })
            .collect();

        if !serial_seqs.is_empty() {
            self.ddl.retain(|e| {
                if let PostgresEntity::Sequence(s) = e {
                    !serial_seqs.contains(&(s.schema.to_string(), s.name.to_string()))
                } else {
                    true
                }
            });
        }
    }

    /// Normalize introspected columns for push comparison.
    ///
    /// - Converts `int4 + nextval()` back to `SERIAL` (and analogously for
    ///   int8→BIGSERIAL, int2→SMALLSERIAL) so the live snapshot matches the
    ///   desired snapshot that uses serial pseudo-types.
    /// - Strips `ordinal_position` from all columns (desired snapshots don't
    ///   have it but introspected ones do).
    pub fn normalize_columns_for_push(&mut self) {
        for entity in &mut self.ddl {
            if let PostgresEntity::Column(c) = entity {
                // Strip fields that only appear in introspection
                c.ordinal_position = None;
                // pg_catalog is the default for built-in types — clear it
                if c.type_schema.as_deref() == Some("pg_catalog") {
                    c.type_schema = None;
                }
                // Detect serial pattern: integer type + nextval() default
                if let Some(ref default) = c.default
                    && is_serial_expression(default, &c.schema)
                {
                    let serial_type = match c.sql_type.as_ref() {
                        "int4" | "integer" => Some("SERIAL"),
                        "int8" | "bigint" => Some("BIGSERIAL"),
                        "int2" | "smallint" => Some("SMALLSERIAL"),
                        _ => None,
                    };
                    if let Some(st) = serial_type {
                        c.sql_type = st.to_string().into();
                        c.default = None;
                    }
                }
            }
        }
    }

    /// Extract the set of `(schema, table_name)` pairs in this snapshot.
    pub fn table_names(&self) -> HashSet<(String, String)> {
        let mut tables = HashSet::new();
        for entity in &self.ddl {
            if let PostgresEntity::Table(t) = entity {
                tables.insert((t.schema.to_string(), t.name.to_string()));
            }
        }
        tables
    }

    /// Extract the unique schema names referenced by tables in this snapshot.
    pub fn schema_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .table_names()
            .into_iter()
            .map(|(s, _)| s)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        names.sort();
        names
    }

    /// Prepare a live (introspected) snapshot for push comparison against `desired`.
    ///
    /// Combines three normalization steps into a single call:
    /// 1. Scope to only tables present in `desired` (avoids DROP for unmanaged tables)
    /// 2. Filter serial-owned sequences (they're auto-managed by PostgreSQL)
    /// 3. Normalize columns (int4+nextval→SERIAL, strip ordinal_position)
    pub fn prepare_for_push(&self, desired: &Self) -> Self {
        let tables = desired.table_names();
        let mut scoped = self.scoped_to_tables(&tables);
        scoped.filter_serial_sequences();
        scoped.normalize_columns_for_push();
        scoped
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, json)
    }
}

// =============================================================================
// Legacy V7 Snapshot (drizzle-kit stable)
// =============================================================================

/// Schema metadata for tracking renames (Legacy)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Meta {
    #[serde(default)]
    pub schemas: HashMap<String, String>,
    #[serde(default)]
    pub tables: HashMap<String, String>,
    #[serde(default)]
    pub columns: HashMap<String, String>,
}

/// Legacy V7 Snapshot for reading compatibility
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSnapshotV7 {
    pub version: String,
    pub dialect: String,
    pub id: String,
    pub prev_id: String,
    // Using Value for legacy details to avoid redefining all legacy structs
    pub tables: HashMap<String, serde_json::Value>,
    pub enums: HashMap<String, serde_json::Value>,
    pub schemas: HashMap<String, serde_json::Value>,
    pub sequences: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub views: HashMap<String, serde_json::Value>,
    #[serde(rename = "_meta")]
    pub meta: Meta,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::ddl::{Column, Schema, Sequence, Table};

    fn make_table(schema: &str, name: &str) -> PostgresEntity {
        PostgresEntity::Table(Table {
            schema: schema.to_string().into(),
            name: name.to_string().into(),
            is_rls_enabled: None,
        })
    }

    fn make_column(schema: &str, table: &str, name: &str, sql_type: &str) -> Column {
        Column::new(
            schema.to_string(),
            table.to_string(),
            name.to_string(),
            sql_type.to_string(),
        )
    }

    fn make_sequence(schema: &str, name: &str) -> PostgresEntity {
        PostgresEntity::Sequence(Sequence {
            schema: schema.to_string().into(),
            name: name.to_string().into(),
            increment_by: None,
            min_value: None,
            max_value: None,
            start_with: None,
            cache_size: None,
            cycle: None,
        })
    }

    #[test]
    fn test_new_snapshot() {
        let snapshot = PostgresSnapshot::new();
        assert_eq!(snapshot.version, "8");
        assert_eq!(snapshot.dialect, "postgres");
        assert_eq!(snapshot.prev_ids, vec![ORIGIN_UUID]);
        assert!(snapshot.ddl.is_empty());
        assert!(snapshot.renames.is_empty());
    }

    #[test]
    fn test_add_entity() {
        let mut snapshot = PostgresSnapshot::new();

        let schema = Schema::new("public");
        snapshot.add_entity(PostgresEntity::Schema(schema));

        let table = Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
        };
        snapshot.add_entity(PostgresEntity::Table(table));

        assert_eq!(snapshot.ddl.len(), 2);
    }

    #[test]
    fn test_schema_names() {
        let mut snap = PostgresSnapshot::new();
        snap.add_entity(make_table("public", "users"));
        snap.add_entity(make_table("auth", "sessions"));
        snap.add_entity(make_table("public", "posts"));

        let names = snap.schema_names();
        assert_eq!(names, vec!["auth", "public"]);
    }

    #[test]
    fn test_schema_names_empty() {
        let snap = PostgresSnapshot::new();
        assert!(snap.schema_names().is_empty());
    }

    #[test]
    fn test_filter_serial_sequences() {
        let mut snap = PostgresSnapshot::new();
        snap.add_entity(make_table("public", "users"));
        // Serial column with nextval default
        let mut col = make_column("public", "users", "id", "int4");
        col.default = Some("nextval('users_id_seq'::regclass)".into());
        snap.add_entity(PostgresEntity::Column(col));
        // The auto-created sequence
        snap.add_entity(make_sequence("public", "users_id_seq"));
        // An unrelated sequence that should survive
        snap.add_entity(make_sequence("public", "custom_seq"));

        snap.filter_serial_sequences();

        let seq_names: Vec<&str> = snap
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Sequence(s) = e {
                    Some(s.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(seq_names, vec!["custom_seq"]);
    }

    #[test]
    fn test_normalize_columns_for_push() {
        let mut snap = PostgresSnapshot::new();
        // int4 + nextval → should become SERIAL
        let mut col = make_column("public", "users", "id", "int4");
        col.default = Some("nextval('users_id_seq'::regclass)".into());
        col.ordinal_position = Some(1);
        col.type_schema = Some("pg_catalog".into());
        snap.add_entity(PostgresEntity::Column(col));

        // bigint + nextval → BIGSERIAL
        let mut col2 = make_column("public", "users", "big_id", "bigint");
        col2.default = Some("nextval('users_big_id_seq'::regclass)".into());
        snap.add_entity(PostgresEntity::Column(col2));

        // Regular column — should be untouched (except ordinal/type_schema)
        let mut col3 = make_column("public", "users", "name", "text");
        col3.ordinal_position = Some(3);
        snap.add_entity(PostgresEntity::Column(col3));

        snap.normalize_columns_for_push();

        let columns: Vec<&Column> = snap
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Column(c) = e {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();

        // id: int4+nextval → SERIAL, no default, no ordinal, no type_schema
        assert_eq!(columns[0].sql_type.as_ref(), "SERIAL");
        assert!(columns[0].default.is_none());
        assert!(columns[0].ordinal_position.is_none());
        assert!(columns[0].type_schema.is_none());

        // big_id: bigint+nextval → BIGSERIAL
        assert_eq!(columns[1].sql_type.as_ref(), "BIGSERIAL");
        assert!(columns[1].default.is_none());

        // name: unchanged type, ordinal stripped
        assert_eq!(columns[2].sql_type.as_ref(), "text");
        assert!(columns[2].ordinal_position.is_none());
    }

    #[test]
    fn test_prepare_for_push() {
        // Live snapshot has extra tables/sequences
        let mut live = PostgresSnapshot::new();
        live.add_entity(PostgresEntity::Schema(Schema::new("public")));
        live.add_entity(make_table("public", "users"));
        live.add_entity(make_table("public", "unmanaged"));
        let mut col = make_column("public", "users", "id", "int4");
        col.default = Some("nextval('users_id_seq'::regclass)".into());
        col.ordinal_position = Some(1);
        col.type_schema = Some("pg_catalog".into());
        live.add_entity(PostgresEntity::Column(col));
        live.add_entity(make_sequence("public", "users_id_seq"));

        // Desired only has "users"
        let mut desired = PostgresSnapshot::new();
        desired.add_entity(make_table("public", "users"));

        let result = live.prepare_for_push(&desired);

        // "unmanaged" table should be filtered out
        let table_names: Vec<&str> = result
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Table(t) = e {
                    Some(t.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(table_names, vec!["users"]);

        // Serial sequence should be filtered
        let seq_count = result
            .ddl
            .iter()
            .filter(|e| matches!(e, PostgresEntity::Sequence(_)))
            .count();
        assert_eq!(seq_count, 0);

        // Column should be normalized to SERIAL
        let col = result
            .ddl
            .iter()
            .find_map(|e| {
                if let PostgresEntity::Column(c) = e {
                    Some(c)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(col.sql_type.as_ref(), "SERIAL");
        assert!(col.default.is_none());
        assert!(col.ordinal_position.is_none());
        assert!(col.type_schema.is_none());
    }

    #[test]
    fn test_scoped_to_tables_keeps_relevant_entities() {
        let mut snap = PostgresSnapshot::new();
        snap.add_entity(PostgresEntity::Schema(Schema::new("public")));
        snap.add_entity(PostgresEntity::Schema(Schema::new("other")));
        snap.add_entity(make_table("public", "users"));
        snap.add_entity(make_table("other", "logs"));
        snap.add_entity(make_sequence("public", "my_seq"));
        snap.add_entity(make_sequence("other", "other_seq"));

        let tables: HashSet<(String, String)> =
            [("public".to_string(), "users".to_string())].into();
        let scoped = snap.scoped_to_tables(&tables);

        // Only "public" schema, "users" table, and "public" sequence
        let schemas: Vec<&str> = scoped
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Schema(s) = e {
                    Some(s.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(schemas, vec!["public"]);

        let tables: Vec<&str> = scoped
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Table(t) = e {
                    Some(t.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tables, vec!["users"]);

        let seqs: Vec<&str> = scoped
            .ddl
            .iter()
            .filter_map(|e| {
                if let PostgresEntity::Sequence(s) = e {
                    Some(s.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(seqs, vec!["my_seq"]);
    }
}
