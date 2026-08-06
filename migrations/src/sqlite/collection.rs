//! `SQLite` DDL collection — typed access to schema entities.
//!
//! The generic [`EntityCollection<T>`] storage backbone lives in
//! [`crate::collection`]; this file supplies the per-entity-type lookup
//! helpers (`one`, `for_table`, `delete`) whose shape depends on each
//! SQLite entity's identity (single-name for `Table`/`Index`; `(table,
//! name)` for `Column`).

use super::ddl::{
    CheckConstraint, Column, ForeignKey, Index, PrimaryKey, SqliteEntity, Table, UniqueConstraint,
    View,
};
use crate::collection::EntityCollection;
use crate::traits::EntityKind;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

// =============================================================================
// Per-entity-type lookup helpers
// =============================================================================

// Table-specific operations
impl EntityCollection<Table> {
    /// Find a table by name
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&Table> {
        self.entities.iter().find(|t| t.name == name)
    }

    /// Delete a table by name
    pub fn delete(&mut self, name: &str) -> Option<Table> {
        if let Some(pos) = self.entities.iter().position(|t| t.name == name) {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }
}

// Column-specific operations
impl EntityCollection<Column> {
    /// Find a column by table and name
    #[must_use]
    pub fn one(&self, table: &str, name: &str) -> Option<&Column> {
        self.entities
            .iter()
            .find(|c| c.table == table && c.name == name)
    }

    /// List columns for a table
    #[must_use]
    pub fn for_table(&self, table: &str) -> Vec<&Column> {
        self.entities.iter().filter(|c| c.table == table).collect()
    }

    /// Delete a column by table and name
    pub fn delete(&mut self, table: &str, name: &str) -> Option<Column> {
        if let Some(pos) = self
            .entities
            .iter()
            .position(|c| c.table == table && c.name == name)
        {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }
}

// Index-specific operations
impl EntityCollection<Index> {
    /// Find an index by name
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&Index> {
        self.entities.iter().find(|i| i.name == name)
    }

    /// List indexes for a table
    #[must_use]
    pub fn for_table(&self, table: &str) -> Vec<&Index> {
        self.entities.iter().filter(|i| i.table == table).collect()
    }
}

// ForeignKey-specific operations
impl EntityCollection<ForeignKey> {
    /// Find a foreign key by name
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&ForeignKey> {
        self.entities.iter().find(|f| f.name == name)
    }

    /// List foreign keys for a table
    #[must_use]
    pub fn for_table(&self, table: &str) -> Vec<&ForeignKey> {
        self.entities.iter().filter(|f| f.table == table).collect()
    }
}

// PrimaryKey-specific operations
impl EntityCollection<PrimaryKey> {
    /// Find a primary key by table
    #[must_use]
    pub fn for_table(&self, table: &str) -> Option<&PrimaryKey> {
        self.entities.iter().find(|p| p.table == table)
    }
}

// UniqueConstraint-specific operations
impl EntityCollection<UniqueConstraint> {
    /// Find by name
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&UniqueConstraint> {
        self.entities.iter().find(|u| u.name == name)
    }

    /// List for a table
    #[must_use]
    pub fn for_table(&self, table: &str) -> Vec<&UniqueConstraint> {
        self.entities.iter().filter(|u| u.table == table).collect()
    }
}

// CheckConstraint-specific operations
impl EntityCollection<CheckConstraint> {
    /// Find by name
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&CheckConstraint> {
        self.entities.iter().find(|c| c.name == name)
    }

    /// List for a table
    #[must_use]
    pub fn for_table(&self, table: &str) -> Vec<&CheckConstraint> {
        self.entities.iter().filter(|c| c.table == table).collect()
    }
}

// View-specific operations
impl EntityCollection<View> {
    /// Find a view by name
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&View> {
        self.entities.iter().find(|v| v.name == name)
    }
}

// =============================================================================
// SQLite DDL - Main Collection Type
// =============================================================================

/// `SQLite` DDL collection - stores all schema entities
///
/// This is the main type for working with DDL entities.
/// It provides typed access to each entity type with collection operations.
#[derive(Debug, Clone, Default)]
pub struct SQLiteDDL {
    pub tables: EntityCollection<Table>,
    pub columns: EntityCollection<Column>,
    pub indexes: EntityCollection<Index>,
    pub fks: EntityCollection<ForeignKey>,
    pub pks: EntityCollection<PrimaryKey>,
    pub uniques: EntityCollection<UniqueConstraint>,
    pub checks: EntityCollection<CheckConstraint>,
    pub views: EntityCollection<View>,
}

impl SQLiteDDL {
    /// Create a new empty DDL collection
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create DDL from a list of entities
    #[must_use]
    pub fn from_entities(entities: Vec<SqliteEntity>) -> Self {
        let mut ddl = Self::new();
        for entity in entities {
            ddl.push_entity(entity);
        }
        ddl
    }

    /// Push any entity type
    pub fn push_entity(&mut self, entity: SqliteEntity) {
        match entity {
            SqliteEntity::Table(t) => self.tables.push(t),
            SqliteEntity::Column(c) => self.columns.push(c),
            SqliteEntity::Index(i) => self.indexes.push(i),
            SqliteEntity::ForeignKey(f) => self.fks.push(f),
            SqliteEntity::PrimaryKey(p) => self.pks.push(p),
            SqliteEntity::UniqueConstraint(u) => self.uniques.push(u),
            SqliteEntity::CheckConstraint(c) => self.checks.push(c),
            SqliteEntity::View(v) => self.views.push(v),
        };
    }

    /// Convert to entity array for snapshot serialization
    #[must_use]
    pub fn to_entities(&self) -> Vec<SqliteEntity> {
        let mut entities = Vec::new();

        // Tables first
        for t in self.tables.list() {
            entities.push(SqliteEntity::Table(t.clone()));
        }
        // Then columns
        for c in self.columns.list() {
            entities.push(SqliteEntity::Column(c.clone()));
        }
        // Then other entities
        for i in self.indexes.list() {
            entities.push(SqliteEntity::Index(i.clone()));
        }
        for f in self.fks.list() {
            entities.push(SqliteEntity::ForeignKey(f.clone()));
        }
        for p in self.pks.list() {
            entities.push(SqliteEntity::PrimaryKey(p.clone()));
        }
        for u in self.uniques.list() {
            entities.push(SqliteEntity::UniqueConstraint(u.clone()));
        }
        for c in self.checks.list() {
            entities.push(SqliteEntity::CheckConstraint(c.clone()));
        }
        for v in self.views.list() {
            entities.push(SqliteEntity::View(v.clone()));
        }

        entities
    }

    /// Check if DDL is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tables.is_empty()
            && self.columns.is_empty()
            && self.indexes.is_empty()
            && self.fks.is_empty()
            && self.pks.is_empty()
            && self.uniques.is_empty()
            && self.checks.is_empty()
            && self.views.is_empty()
    }

    /// Get all entities for a specific table
    #[must_use]
    pub fn table_entities<'a>(&'a self, table_name: &str) -> TableEntities<'a> {
        TableEntities {
            columns: self.columns.for_table(table_name),
            indexes: self.indexes.for_table(table_name),
            fks: self.fks.for_table(table_name),
            pk: self.pks.for_table(table_name),
            uniques: self.uniques.for_table(table_name),
            checks: self.checks.for_table(table_name),
        }
    }
}

/// All entities belonging to a specific table
pub struct TableEntities<'a> {
    pub columns: Vec<&'a Column>,
    pub indexes: Vec<&'a Index>,
    pub fks: Vec<&'a ForeignKey>,
    pub pk: Option<&'a PrimaryKey>,
    pub uniques: Vec<&'a UniqueConstraint>,
    pub checks: Vec<&'a CheckConstraint>,
}

// =============================================================================
// Diff Types
// =============================================================================

// Re-export shared DiffType from traits module
pub use crate::traits::DiffType;

/// A diff statement for any entity
#[derive(Debug, Clone)]
pub struct EntityDiff {
    pub diff_type: DiffType,
    pub kind: EntityKind,
    pub table: Option<String>,
    pub name: String,
    /// For alter: changed fields with (from, to) values
    pub changes: HashMap<String, (String, String)>,
    /// Original entity (for drop/alter)
    pub left: Option<SqliteEntity>,
    /// New entity (for create/alter)
    pub right: Option<SqliteEntity>,
}

/// Compute diff between two DDL collections
#[must_use]
pub fn diff_ddl(left: &SQLiteDDL, right: &SQLiteDDL) -> Vec<EntityDiff> {
    let mut diffs = Vec::new();

    // Diff tables (no table_fn needed since these ARE tables)
    diff_entity_type(
        left.tables.list(),
        right.tables.list(),
        |t| t.name.to_string(),
        |t| SqliteEntity::Table(t.clone()),
        None,
        EntityKind::Table,
        &mut diffs,
    );

    // Diff columns - extract table name from column.
    // Inline INTEGER PRIMARY KEY columns need context for the NOT NULL
    // reconciliation (emitters skip NOT NULL, PRAGMA reports 0, snapshots say
    // true), so collect them from both sides first.
    let integer_pks: HashSet<(String, String)> = inline_integer_pk_columns(left)
        .into_iter()
        .chain(inline_integer_pk_columns(right))
        .collect();
    diff_entity_type_with(
        left.columns.list(),
        right.columns.list(),
        |c| format!("{}:{}", c.table, c.name),
        |c| SqliteEntity::Column(c.clone()),
        Some(&|c: &Column| c.table.to_string()),
        EntityKind::Column,
        |l, r| columns_equivalent(l, r, &integer_pks),
        &mut diffs,
    );

    // Diff indexes - extract table name from index
    diff_entity_type(
        left.indexes.list(),
        right.indexes.list(),
        |i| i.name.to_string(),
        |i| SqliteEntity::Index(i.clone()),
        Some(&|i: &Index| i.table.to_string()),
        EntityKind::Index,
        &mut diffs,
    );

    // Diff foreign keys - keyed structurally (table/columns/target), NOT by
    // name: PRAGMA foreign_key_list cannot recover real FK names, so keying or
    // comparing by name would guarantee drop/create churn on every push.
    // Names are still carried on the entities and used for rendering.
    diff_entity_type_with(
        left.fks.list(),
        right.fks.list(),
        fk_structural_key,
        |f| SqliteEntity::ForeignKey(f.clone()),
        Some(&|f: &ForeignKey| f.table.to_string()),
        EntityKind::ForeignKey,
        foreign_keys_equivalent,
        &mut diffs,
    );

    // Diff primary keys - keyed by table, compared by column set (names are
    // synthesized during introspection and irrelevant for equivalence).
    diff_entity_type_with(
        left.pks.list(),
        right.pks.list(),
        |p| p.table.to_string(),
        |p| SqliteEntity::PrimaryKey(p.clone()),
        Some(&|p: &PrimaryKey| p.table.to_string()),
        EntityKind::PrimaryKey,
        primary_keys_equivalent,
        &mut diffs,
    );

    // Diff unique constraints - extract table name from unique
    diff_entity_type(
        left.uniques.list(),
        right.uniques.list(),
        |u| u.name.to_string(),
        |u| SqliteEntity::UniqueConstraint(u.clone()),
        Some(&|u: &UniqueConstraint| u.table.to_string()),
        EntityKind::UniqueConstraint,
        &mut diffs,
    );

    // Diff check constraints - extract table name from check
    diff_entity_type(
        left.checks.list(),
        right.checks.list(),
        |c| c.name.to_string(),
        |c| SqliteEntity::CheckConstraint(c.clone()),
        Some(&|c: &CheckConstraint| c.table.to_string()),
        EntityKind::CheckConstraint,
        &mut diffs,
    );

    // Diff views (no table_fn needed since views are standalone)
    diff_entity_type(
        left.views.list(),
        right.views.list(),
        |v| v.name.to_string(),
        |v| SqliteEntity::View(v.clone()),
        None,
        EntityKind::View,
        &mut diffs,
    );

    diffs
}

/// Helper to diff a single entity type
fn diff_entity_type<T: Clone + PartialEq>(
    left: &[T],
    right: &[T],
    key_fn: impl Fn(&T) -> String,
    to_entity: impl Fn(&T) -> SqliteEntity,
    table_fn: Option<&dyn Fn(&T) -> String>,
    kind: EntityKind,
    diffs: &mut Vec<EntityDiff>,
) {
    diff_entity_type_with(
        left,
        right,
        key_fn,
        to_entity,
        table_fn,
        kind,
        PartialEq::eq,
        diffs,
    );
}

#[allow(clippy::too_many_arguments)]
fn diff_entity_type_with<T: Clone>(
    left: &[T],
    right: &[T],
    key_fn: impl Fn(&T) -> String,
    to_entity: impl Fn(&T) -> SqliteEntity,
    table_fn: Option<&dyn Fn(&T) -> String>,
    kind: EntityKind,
    equivalent: impl Fn(&T, &T) -> bool,
    diffs: &mut Vec<EntityDiff>,
) {
    let left_map: HashMap<String, &T> = left.iter().map(|e| (key_fn(e), e)).collect();
    let right_map: HashMap<String, &T> = right.iter().map(|e| (key_fn(e), e)).collect();

    // Find dropped (in left but not in right)
    for left_entity in left {
        let key = key_fn(left_entity);
        if !right_map.contains_key(&key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Drop,
                kind,
                table: table_fn.map(|f| f(left_entity)),
                name: key,
                changes: HashMap::new(),
                left: Some(to_entity(left_entity)),
                right: None,
            });
        }
    }

    // Find created (in right but not in left)
    for right_entity in right {
        let key = key_fn(right_entity);
        if !left_map.contains_key(&key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Create,
                kind,
                table: table_fn.map(|f| f(right_entity)),
                name: key,
                changes: HashMap::new(),
                left: None,
                right: Some(to_entity(right_entity)),
            });
        }
    }

    // Find altered (in both, but different)
    for left_entity in left {
        let key = key_fn(left_entity);
        if let Some(right_entity) = right_map.get(&key)
            && !equivalent(left_entity, right_entity)
        {
            diffs.push(EntityDiff {
                diff_type: DiffType::Alter,
                kind,
                table: table_fn.map(|f| f(right_entity)),
                name: key,
                changes: HashMap::new(), // Field-level comparison available via left/right entities
                left: Some(to_entity(left_entity)),
                right: Some(to_entity(right_entity)),
            });
        }
    }
}

// =============================================================================
// Equivalence normalization
//
// Introspected DDL and macro/snapshot DDL systematically differ in ways that
// don't change the rendered schema (ordinal positions, literal quoting styles,
// synthesized constraint names, ...). These helpers normalize both sides
// before comparing so that a push round-trip is a no-op. Rendering always
// uses the original, un-normalized entities.
// =============================================================================

/// Strips one layer of outer balanced parentheses, if fully wrapped.
fn strip_outer_parens(expr: &str) -> &str {
    let expr = expr.trim();
    let bytes = expr.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'(' || bytes[bytes.len() - 1] != b')' {
        return expr;
    }
    let mut depth = 0i32;
    for (i, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    if i == expr.len() - 1 {
                        return expr[1..expr.len() - 1].trim();
                    }
                    return expr;
                }
            }
            _ => {}
        }
    }
    expr
}

/// Normalizes a DEFAULT literal for comparison: strips one paren layer, then
/// one layer of matching quotes (`'x'` ≡ `"x"` ≡ `x`, with doubled-quote
/// unescaping), then canonicalizes numeric literals (`0.0` ≡ `0`).
fn normalize_default_literal(default: &str) -> String {
    let s = strip_outer_parens(default);
    let bytes = s.as_bytes();
    let unquoted = if bytes.len() >= 2 && bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'' {
        s[1..s.len() - 1].replace("''", "'")
    } else if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        s[1..s.len() - 1].replace("\"\"", "\"")
    } else {
        s.to_string()
    };

    if let Ok(int) = unquoted.parse::<i128>() {
        return int.to_string();
    }
    if let Ok(float) = unquoted.parse::<f64>()
        && float.is_finite()
    {
        return float.to_string();
    }
    unquoted
}

/// Collects `(table, column)` pairs that render as inline `INTEGER PRIMARY
/// KEY` (single-column PK entity, or a lone column-level PK flag).
fn inline_integer_pk_columns(ddl: &SQLiteDDL) -> HashSet<(String, String)> {
    let mut out = HashSet::new();

    let mut candidates: Vec<(String, String)> = ddl
        .pks
        .list()
        .iter()
        .filter(|pk| pk.columns.len() == 1)
        .map(|pk| (pk.table.to_string(), pk.columns[0].to_string()))
        .collect();

    // Column-level flags: only a single flag column per table renders inline.
    let mut flag_pks: HashMap<String, Vec<String>> = HashMap::new();
    for c in ddl.columns.list() {
        if c.primary_key == Some(true) {
            flag_pks
                .entry(c.table.to_string())
                .or_default()
                .push(c.name.to_string());
        }
    }
    for (table, cols) in flag_pks {
        if let [col] = cols.as_slice() {
            candidates.push((table, col.clone()));
        }
    }

    for (table, col) in candidates {
        let is_integer = ddl
            .columns
            .one(&table, &col)
            .is_some_and(|c| c.sql_type.to_ascii_lowercase().starts_with("int"));
        if is_integer {
            out.insert((table, col));
        }
    }
    out
}

fn columns_equivalent(
    left: &Column,
    right: &Column,
    integer_pks: &HashSet<(String, String)>,
) -> bool {
    let normalize = |column: &Column| -> Column {
        let mut c = column.clone();
        c.sql_type = Cow::Owned(c.sql_type.to_ascii_lowercase());
        // (a) ordinal position is introspection metadata, not schema shape
        c.ordinal_position = None;
        // Explicit `Some(false)` flags are equivalent to omitted flags
        if c.primary_key == Some(false) {
            c.primary_key = None;
        }
        if c.unique == Some(false) {
            c.unique = None;
        }
        if c.autoincrement == Some(false) {
            c.autoincrement = None;
        }
        // (b) inline INTEGER PRIMARY KEY: emitters skip NOT NULL and PRAGMA
        // reports 0 while snapshots say true — both render identically, so
        // pin not_null for comparison purposes.
        if integer_pks.contains(&(c.table.to_string(), c.name.to_string())) {
            c.not_null = true;
        }
        // (c) default literal quoting/numeric normalization
        if let Some(default) = c.default.as_ref() {
            c.default = Some(Cow::Owned(normalize_default_literal(default)));
        }
        // Generated expressions: macro producers store `(expr)`, introspection
        // stores bare `expr` — strip one paren layer from both sides.
        if let Some(generated) = c.generated.as_mut() {
            generated.expression =
                Cow::Owned(strip_outer_parens(&generated.expression).to_string());
        }
        c
    };

    normalize(left) == normalize(right)
}

fn normalize_fk_action(action: &Option<Cow<'static, str>>) -> Option<Cow<'static, str>> {
    match action.as_deref() {
        None => None,
        Some(action) if action.eq_ignore_ascii_case("NO ACTION") => None,
        Some(action) => Some(Cow::Owned(action.to_ascii_uppercase())),
    }
}

/// Structural identity for a foreign key (used as the diff key): PRAGMA cannot
/// recover FK constraint names, so identity is the (table, columns, target
/// table, target columns) shape.
fn fk_structural_key(fk: &ForeignKey) -> String {
    let cols: Vec<&str> = fk.columns.iter().map(AsRef::as_ref).collect();
    let cols_to: Vec<&str> = fk.columns_to.iter().map(AsRef::as_ref).collect();
    format!(
        "{}({})->{}({})",
        fk.table,
        cols.join(","),
        fk.table_to,
        cols_to.join(",")
    )
}

fn foreign_keys_equivalent(left: &ForeignKey, right: &ForeignKey) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.on_delete = normalize_fk_action(&left.on_delete);
    left.on_update = normalize_fk_action(&left.on_update);
    right.on_delete = normalize_fk_action(&right.on_delete);
    right.on_update = normalize_fk_action(&right.on_update);
    // (e) names cannot be recovered from PRAGMA — equivalence is structural
    left.name = Cow::Borrowed("");
    right.name = Cow::Borrowed("");
    left.name_explicit = false;
    right.name_explicit = false;
    left == right
}

/// (d) primary keys compare by column set (order-insensitive), not name.
fn primary_keys_equivalent(left: &PrimaryKey, right: &PrimaryKey) -> bool {
    if left.table != right.table {
        return false;
    }
    let mut left_cols: Vec<&str> = left.columns.iter().map(AsRef::as_ref).collect();
    let mut right_cols: Vec<&str> = right.columns.iter().map(AsRef::as_ref).collect();
    left_cols.sort_unstable();
    right_cols.sort_unstable();
    left_cols == right_cols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ddl_collection_push() {
        let mut ddl = SQLiteDDL::new();

        ddl.tables.push(Table::new("users"));
        ddl.columns.push(Column::new("users", "id", "integer"));
        ddl.columns.push(Column::new("users", "name", "text"));

        assert_eq!(ddl.tables.len(), 1);
        assert_eq!(ddl.columns.len(), 2);
        assert_eq!(ddl.columns.for_table("users").len(), 2);
    }

    #[test]
    fn test_ddl_to_entities() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.columns
            .push(Column::new("users", "id", "integer").not_null());

        let entities = ddl.to_entities();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_diff_create() {
        let left = SQLiteDDL::new();
        let mut right = SQLiteDDL::new();
        right.tables.push(Table::new("users"));

        let diffs = diff_ddl(&left, &right);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Create);
        assert_eq!(diffs[0].kind, EntityKind::Table);
    }

    #[test]
    fn test_diff_drop() {
        let mut left = SQLiteDDL::new();
        left.tables.push(Table::new("users"));
        let right = SQLiteDDL::new();

        let diffs = diff_ddl(&left, &right);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Drop);
    }

    #[test]
    fn ordinal_position_is_ignored_in_column_equivalence() {
        let mut introspected = SQLiteDDL::new();
        introspected.tables.push(Table::new("t"));
        let mut col = Column::new("t", "name", "text").not_null();
        col.ordinal_position = Some(3);
        introspected.columns.push(col);

        let mut snapshot = SQLiteDDL::new();
        snapshot.tables.push(Table::new("t"));
        snapshot
            .columns
            .push(Column::new("t", "name", "TEXT").not_null());

        let diffs = diff_ddl(&introspected, &snapshot);
        assert!(diffs.is_empty(), "unexpected diffs: {diffs:#?}");
    }

    #[test]
    fn integer_pk_not_null_mismatch_is_reconciled() {
        use crate::sqlite::ddl::PrimaryKey;

        // Introspected: PRAGMA reports notnull = 0 for INTEGER PRIMARY KEY
        let mut introspected = SQLiteDDL::new();
        introspected.tables.push(Table::new("t"));
        introspected.columns.push(Column::new("t", "id", "integer"));
        introspected.pks.push(PrimaryKey::from_strings(
            "t".to_string(),
            "t_pk".to_string(),
            vec!["id".to_string()],
        ));

        // Snapshot: macro marks PK fields NOT NULL
        let mut snapshot = SQLiteDDL::new();
        snapshot.tables.push(Table::new("t"));
        snapshot
            .columns
            .push(Column::new("t", "id", "INTEGER").not_null());
        snapshot.pks.push(PrimaryKey::from_strings(
            "t".to_string(),
            "t_pk".to_string(),
            vec!["id".to_string()],
        ));

        let diffs = diff_ddl(&introspected, &snapshot);
        assert!(diffs.is_empty(), "unexpected diffs: {diffs:#?}");
    }

    #[test]
    fn default_literal_quoting_is_normalized() {
        for (left_default, right_default) in [
            ("'hello'", "hello"),
            ("\"hello\"", "'hello'"),
            ("'it''s'", "it's"),
            ("0.0", "0"),
            ("(42)", "42"),
        ] {
            let mut left = SQLiteDDL::new();
            left.tables.push(Table::new("t"));
            left.columns
                .push(Column::new("t", "c", "text").default_value(left_default.to_string()));

            let mut right = SQLiteDDL::new();
            right.tables.push(Table::new("t"));
            right
                .columns
                .push(Column::new("t", "c", "text").default_value(right_default.to_string()));

            let diffs = diff_ddl(&left, &right);
            assert!(
                diffs.is_empty(),
                "{left_default:?} vs {right_default:?} should be equivalent: {diffs:#?}"
            );
        }

        // Different values must still diff.
        let mut left = SQLiteDDL::new();
        left.tables.push(Table::new("t"));
        left.columns
            .push(Column::new("t", "c", "text").default_value("'a'"));
        let mut right = SQLiteDDL::new();
        right.tables.push(Table::new("t"));
        right
            .columns
            .push(Column::new("t", "c", "text").default_value("'b'"));
        assert_eq!(diff_ddl(&left, &right).len(), 1);
    }

    #[test]
    fn generated_expression_parens_are_normalized() {
        use crate::sqlite::ddl::{Generated, GeneratedType};

        let make = |expr: &str| {
            let mut ddl = SQLiteDDL::new();
            ddl.tables.push(Table::new("t"));
            let mut col = Column::new("t", "g", "text");
            col.generated = Some(Generated {
                expression: expr.to_string().into(),
                gen_type: GeneratedType::Virtual,
            });
            ddl.columns.push(col);
            ddl
        };

        // Macro-produced `(expr)` vs introspected `expr`
        let diffs = diff_ddl(&make("(length(name))"), &make("length(name)"));
        assert!(diffs.is_empty(), "unexpected diffs: {diffs:#?}");

        // Different expressions still diff
        assert_eq!(
            diff_ddl(&make("(length(name))"), &make("length(other)")).len(),
            1
        );
    }

    #[test]
    fn primary_keys_compare_by_column_set_not_name() {
        use crate::sqlite::ddl::PrimaryKey;

        let make = |name: &str, cols: Vec<&str>| {
            let mut ddl = SQLiteDDL::new();
            ddl.tables.push(Table::new("t"));
            ddl.columns
                .push(Column::new("t", "a", "integer").not_null());
            ddl.columns
                .push(Column::new("t", "b", "integer").not_null());
            ddl.pks.push(PrimaryKey::from_strings(
                "t".to_string(),
                name.to_string(),
                cols.into_iter().map(str::to_string).collect(),
            ));
            ddl
        };

        // Same column set, different name and order: equivalent
        let diffs = diff_ddl(
            &make("t_pk", vec!["a", "b"]),
            &make("custom", vec!["b", "a"]),
        );
        assert!(diffs.is_empty(), "unexpected diffs: {diffs:#?}");

        // Different column set: alter
        assert_eq!(
            diff_ddl(&make("t_pk", vec!["a", "b"]), &make("t_pk", vec!["a"])).len(),
            1
        );
    }

    #[test]
    fn foreign_keys_compare_structurally_not_by_name() {
        let make = |name: &str| {
            let mut ddl = SQLiteDDL::new();
            ddl.tables.push(Table::new("child"));
            ddl.tables.push(Table::new("parent"));
            ddl.columns
                .push(Column::new("child", "parent_id", "integer").not_null());
            ddl.fks.push(ForeignKey::from_strings(
                "child".to_string(),
                name.to_string(),
                vec!["parent_id".to_string()],
                "parent".to_string(),
                vec!["id".to_string()],
            ));
            ddl
        };

        // Same structure, different names (introspected names are synthesized):
        // no churn — neither drop/create (structural key) nor alter (structural
        // equivalence).
        let diffs = diff_ddl(&make("fk_child_parent_id_parent_id_fk"), &make("my_fk"));
        assert!(diffs.is_empty(), "unexpected diffs: {diffs:#?}");

        // Different action: alter (single diff, not drop+create)
        let mut with_cascade = make("a");
        with_cascade.fks.list_mut()[0].on_delete = Some("CASCADE".into());
        let diffs = diff_ddl(&make("b"), &with_cascade);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Alter);
    }

    #[test]
    fn introspected_types_and_no_action_fks_match_macro_snapshots() {
        let mut introspected = SQLiteDDL::new();
        introspected.tables.push(Table::new("child"));
        introspected.tables.push(Table::new("parent"));
        introspected
            .columns
            .push(Column::new("child", "id", "integer").not_null());
        introspected
            .columns
            .push(Column::new("child", "parent_id", "integer").not_null());
        introspected.fks.push(
            ForeignKey::from_strings(
                "child".to_string(),
                "child_parent_id_fk".to_string(),
                vec!["parent_id".to_string()],
                "parent".to_string(),
                vec!["id".to_string()],
            )
            .on_delete("NO ACTION")
            .on_update("no action"),
        );

        let mut macro_snapshot = SQLiteDDL::new();
        macro_snapshot.tables.push(Table::new("child"));
        macro_snapshot.tables.push(Table::new("parent"));
        macro_snapshot
            .columns
            .push(Column::new("child", "id", "INTEGER").not_null());
        macro_snapshot
            .columns
            .push(Column::new("child", "parent_id", "INTEGER").not_null());
        macro_snapshot.fks.push(ForeignKey::from_strings(
            "child".to_string(),
            "child_parent_id_fk".to_string(),
            vec!["parent_id".to_string()],
            "parent".to_string(),
            vec!["id".to_string()],
        ));

        let diffs = diff_ddl(&introspected, &macro_snapshot);
        assert!(diffs.is_empty(), "unexpected diffs: {diffs:#?}");
    }
}
