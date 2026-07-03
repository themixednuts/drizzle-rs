//! `PostgreSQL` DDL collection — typed access to schema entities.
//!
//! The generic [`EntityCollection<T>`] storage backbone lives in
//! [`crate::collection`]; this file supplies the per-entity-type lookup
//! helpers (`one`, `for_table`, etc.) whose shape depends on each
//! Postgres entity's identity (`(schema, name)`, `(schema, table, name)`).

use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, PostgresEntity, PrimaryKey, Role,
    Schema, Sequence, Table, UniqueConstraint, View,
};
use crate::collection::EntityCollection;
use crate::traits::EntityKind;
use std::borrow::Cow;
use std::collections::HashMap;

// =============================================================================
// Per-entity-type lookup helpers
// =============================================================================

// Schema-specific operations
impl EntityCollection<Schema> {
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&Schema> {
        self.entities.iter().find(|s| s.name == name)
    }
}

// Enum-specific operations
impl EntityCollection<Enum> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&Enum> {
        self.entities
            .iter()
            .find(|e| e.schema == schema && e.name == name)
    }
}

// Sequence-specific operations
impl EntityCollection<Sequence> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&Sequence> {
        self.entities
            .iter()
            .find(|s| s.schema == schema && s.name == name)
    }
}

// Role-specific operations
impl EntityCollection<Role> {
    #[must_use]
    pub fn one(&self, name: &str) -> Option<&Role> {
        self.entities.iter().find(|r| r.name == name)
    }
}

// Policy-specific operations
impl EntityCollection<Policy> {
    #[must_use]
    pub fn one(&self, schema: &str, table: &str, name: &str) -> Option<&Policy> {
        self.entities
            .iter()
            .find(|p| p.schema == schema && p.table == table && p.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&Policy> {
        self.entities
            .iter()
            .filter(|p| p.schema == schema && p.table == table)
            .collect()
    }
}

// Table-specific operations
impl EntityCollection<Table> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&Table> {
        self.entities
            .iter()
            .find(|t| t.schema == schema && t.name == name)
    }
}

// Column-specific operations
impl EntityCollection<Column> {
    #[must_use]
    pub fn one(&self, schema: &str, table: &str, name: &str) -> Option<&Column> {
        self.entities
            .iter()
            .find(|c| c.schema == schema && c.table == table && c.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&Column> {
        self.entities
            .iter()
            .filter(|c| c.schema == schema && c.table == table)
            .collect()
    }
}

// Index-specific operations
impl EntityCollection<Index> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&Index> {
        self.entities
            .iter()
            .find(|i| i.schema == schema && i.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&Index> {
        self.entities
            .iter()
            .filter(|i| i.schema == schema && i.table == table)
            .collect()
    }
}

// ForeignKey-specific operations
impl EntityCollection<ForeignKey> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&ForeignKey> {
        self.entities
            .iter()
            .find(|f| f.schema == schema && f.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&ForeignKey> {
        self.entities
            .iter()
            .filter(|f| f.schema == schema && f.table == table)
            .collect()
    }
}

// PrimaryKey-specific operations
impl EntityCollection<PrimaryKey> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&PrimaryKey> {
        self.entities
            .iter()
            .find(|p| p.schema == schema && p.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Option<&PrimaryKey> {
        self.entities
            .iter()
            .find(|p| p.schema == schema && p.table == table)
    }
}

// UniqueConstraint-specific operations
impl EntityCollection<UniqueConstraint> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&UniqueConstraint> {
        self.entities
            .iter()
            .find(|u| u.schema == schema && u.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&UniqueConstraint> {
        self.entities
            .iter()
            .filter(|u| u.schema == schema && u.table == table)
            .collect()
    }
}

// CheckConstraint-specific operations
impl EntityCollection<CheckConstraint> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&CheckConstraint> {
        self.entities
            .iter()
            .find(|c| c.schema == schema && c.name == name)
    }
    #[must_use]
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&CheckConstraint> {
        self.entities
            .iter()
            .filter(|c| c.schema == schema && c.table == table)
            .collect()
    }
}

// View-specific operations
impl EntityCollection<View> {
    #[must_use]
    pub fn one(&self, schema: &str, name: &str) -> Option<&View> {
        self.entities
            .iter()
            .find(|v| v.schema == schema && v.name == name)
    }
}

// =============================================================================
// PostgreSQL DDL - Main Collection Type
// =============================================================================

/// `PostgreSQL` DDL collection - stores all schema entities
#[derive(Debug, Clone, Default)]
pub struct PostgresDDL {
    pub schemas: EntityCollection<Schema>,
    pub enums: EntityCollection<Enum>,
    pub sequences: EntityCollection<Sequence>,
    pub roles: EntityCollection<Role>,
    pub policies: EntityCollection<Policy>,
    pub tables: EntityCollection<Table>,
    pub columns: EntityCollection<Column>,
    pub indexes: EntityCollection<Index>,
    pub fks: EntityCollection<ForeignKey>,
    pub pks: EntityCollection<PrimaryKey>,
    pub uniques: EntityCollection<UniqueConstraint>,
    pub checks: EntityCollection<CheckConstraint>,
    pub views: EntityCollection<View>,
}

impl PostgresDDL {
    /// Create a new empty DDL collection
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create DDL from a list of entities
    #[must_use]
    pub fn from_entities(entities: Vec<PostgresEntity>) -> Self {
        let mut ddl = Self::new();
        for entity in entities {
            ddl.push_entity(entity);
        }
        ddl
    }

    /// Push any entity type
    pub fn push_entity(&mut self, entity: PostgresEntity) {
        match entity {
            PostgresEntity::Schema(s) => self.schemas.push(s),
            PostgresEntity::Enum(e) => self.enums.push(e),
            PostgresEntity::Sequence(s) => self.sequences.push(s),
            PostgresEntity::Role(r) => self.roles.push(r),
            PostgresEntity::Policy(p) => self.policies.push(p),
            PostgresEntity::Table(t) => self.tables.push(t),
            PostgresEntity::Column(c) => self.columns.push(c),
            PostgresEntity::Index(i) => self.indexes.push(i),
            PostgresEntity::ForeignKey(f) => self.fks.push(f),
            PostgresEntity::PrimaryKey(p) => self.pks.push(p),
            PostgresEntity::UniqueConstraint(u) => self.uniques.push(u),
            PostgresEntity::CheckConstraint(c) => self.checks.push(c),
            PostgresEntity::View(v) => self.views.push(v),
            // Privileges are not yet tracked in the DDL collection.
            PostgresEntity::Privilege(_) => {}
        }
    }

    /// Convert to entity array for snapshot serialization
    #[must_use]
    pub fn to_entities(&self) -> Vec<PostgresEntity> {
        let mut entities = Vec::new();

        // Push in logical order
        for e in self.schemas.list() {
            entities.push(PostgresEntity::Schema(e.clone()));
        }
        for e in self.enums.list() {
            entities.push(PostgresEntity::Enum(e.clone()));
        }
        for e in self.sequences.list() {
            entities.push(PostgresEntity::Sequence(e.clone()));
        }
        for e in self.roles.list() {
            entities.push(PostgresEntity::Role(e.clone()));
        }

        for e in self.tables.list() {
            entities.push(PostgresEntity::Table(e.clone()));
        }

        for e in self.columns.list() {
            entities.push(PostgresEntity::Column(e.clone()));
        }
        for e in self.indexes.list() {
            entities.push(PostgresEntity::Index(e.clone()));
        }
        for e in self.fks.list() {
            entities.push(PostgresEntity::ForeignKey(e.clone()));
        }
        for e in self.pks.list() {
            entities.push(PostgresEntity::PrimaryKey(e.clone()));
        }
        for e in self.uniques.list() {
            entities.push(PostgresEntity::UniqueConstraint(e.clone()));
        }
        for e in self.checks.list() {
            entities.push(PostgresEntity::CheckConstraint(e.clone()));
        }
        for e in self.policies.list() {
            entities.push(PostgresEntity::Policy(e.clone()));
        }

        for e in self.views.list() {
            entities.push(PostgresEntity::View(e.clone()));
        }

        entities
    }

    /// Check if DDL is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tables.is_empty() && self.enums.is_empty() && self.views.is_empty()
    }
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
    pub name: String,
    /// For alter: changed fields with (from, to) values
    pub changes: HashMap<String, (String, String)>,
    /// Original entity (for drop/alter)
    pub left: Option<PostgresEntity>,
    /// New entity (for create/alter)
    pub right: Option<PostgresEntity>,
}

fn diff_top_level_entities(left: &PostgresDDL, right: &PostgresDDL, diffs: &mut Vec<EntityDiff>) {
    diff_entity_type(
        left.schemas.list(),
        right.schemas.list(),
        |e| e.name.to_string(),
        |e| PostgresEntity::Schema(e.clone()),
        EntityKind::Schema,
        diffs,
    );
    diff_entity_type(
        left.enums.list(),
        right.enums.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Enum(e.clone()),
        EntityKind::Enum,
        diffs,
    );
    diff_entity_type(
        left.sequences.list(),
        right.sequences.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Sequence(e.clone()),
        EntityKind::Sequence,
        diffs,
    );
    diff_entity_type(
        left.roles.list(),
        right.roles.list(),
        |e| e.name.to_string(),
        |e| PostgresEntity::Role(e.clone()),
        EntityKind::Role,
        diffs,
    );
    diff_entity_type_with(
        left.tables.list(),
        right.tables.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Table(e.clone()),
        EntityKind::Table,
        diffs,
        tables_equivalent,
    );
    diff_entity_type(
        left.views.list(),
        right.views.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::View(e.clone()),
        EntityKind::View,
        diffs,
    );
}

fn diff_table_entities(left: &PostgresDDL, right: &PostgresDDL, diffs: &mut Vec<EntityDiff>) {
    diff_entity_type_with(
        left.columns.list(),
        right.columns.list(),
        |e| format!("{}.{}.{}", e.schema, e.table, e.name),
        |e| PostgresEntity::Column(e.clone()),
        EntityKind::Column,
        diffs,
        columns_equivalent,
    );
    diff_entity_type(
        left.indexes.list(),
        right.indexes.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Index(e.clone()),
        EntityKind::Index,
        diffs,
    );
    diff_entity_type_with(
        left.fks.list(),
        right.fks.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::ForeignKey(e.clone()),
        EntityKind::ForeignKey,
        diffs,
        foreign_keys_equivalent,
    );
    diff_entity_type(
        left.pks.list(),
        right.pks.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::PrimaryKey(e.clone()),
        EntityKind::PrimaryKey,
        diffs,
    );
    diff_entity_type(
        left.uniques.list(),
        right.uniques.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::UniqueConstraint(e.clone()),
        EntityKind::UniqueConstraint,
        diffs,
    );
    diff_entity_type(
        left.checks.list(),
        right.checks.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::CheckConstraint(e.clone()),
        EntityKind::CheckConstraint,
        diffs,
    );
    diff_entity_type_with(
        left.policies.list(),
        right.policies.list(),
        |e| format!("{}.{}.{}", e.schema, e.table, e.name),
        |e| PostgresEntity::Policy(e.clone()),
        EntityKind::Policy,
        diffs,
        policies_equivalent,
    );
}

/// Compute diff between two DDL collections
#[must_use]
pub fn diff_ddl(left: &PostgresDDL, right: &PostgresDDL) -> Vec<EntityDiff> {
    let mut diffs = Vec::new();
    diff_top_level_entities(left, right, &mut diffs);
    diff_table_entities(left, right, &mut diffs);
    diffs
}

/// Helper to diff a single entity type
fn diff_entity_type<T: Clone + PartialEq>(
    left: &[T],
    right: &[T],
    key_fn: impl Fn(&T) -> String,
    to_entity: impl Fn(&T) -> PostgresEntity,
    kind: EntityKind,
    diffs: &mut Vec<EntityDiff>,
) {
    diff_entity_type_with(left, right, key_fn, to_entity, kind, diffs, PartialEq::eq);
}

fn diff_entity_type_with<T: Clone>(
    left: &[T],
    right: &[T],
    key_fn: impl Fn(&T) -> String,
    to_entity: impl Fn(&T) -> PostgresEntity,
    kind: EntityKind,
    diffs: &mut Vec<EntityDiff>,
    equivalent: impl Fn(&T, &T) -> bool,
) {
    let left_map: HashMap<String, &T> = left.iter().map(|e| (key_fn(e), e)).collect();
    let right_map: HashMap<String, &T> = right.iter().map(|e| (key_fn(e), e)).collect();

    // Find dropped
    for left_entity in left {
        let key = key_fn(left_entity);
        if !right_map.contains_key(&key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Drop,
                kind,
                name: key,
                changes: HashMap::new(),
                left: Some(to_entity(left_entity)),
                right: None,
            });
        }
    }

    // Find created
    for right_entity in right {
        let key = key_fn(right_entity);
        if !left_map.contains_key(&key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Create,
                kind,
                name: key,
                changes: HashMap::new(),
                left: None,
                right: Some(to_entity(right_entity)),
            });
        }
    }

    // Find altered
    for left_entity in left {
        let key = key_fn(left_entity);
        if let Some(right_entity) = right_map.get(&key)
            && !equivalent(left_entity, right_entity)
        {
            diffs.push(EntityDiff {
                diff_type: DiffType::Alter,
                kind,
                name: key,
                changes: HashMap::new(), // Rely on left/right for details
                left: Some(to_entity(left_entity)),
                right: Some(to_entity(right_entity)),
            });
        }
    }
}

fn tables_equivalent(left: &Table, right: &Table) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.is_rls_enabled = Some(left.is_rls_enabled.unwrap_or(false));
    right.is_rls_enabled = Some(right.is_rls_enabled.unwrap_or(false));
    left.is_unlogged = Some(left.is_unlogged.unwrap_or(false));
    right.is_unlogged = Some(right.is_unlogged.unwrap_or(false));
    left.is_temporary = Some(left.is_temporary.unwrap_or(false));
    right.is_temporary = Some(right.is_temporary.unwrap_or(false));
    left == right
}

fn columns_equivalent(left: &Column, right: &Column) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.sql_type = Cow::Owned(normalize_column_type_for_compare(&left));
    right.sql_type = Cow::Owned(normalize_column_type_for_compare(&right));
    left.dimensions = None;
    right.dimensions = None;
    left.ordinal_position = None;
    right.ordinal_position = None;
    left == right
}

fn foreign_keys_equivalent(left: &ForeignKey, right: &ForeignKey) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.on_delete = normalize_fk_action(left.on_delete.as_deref());
    left.on_update = normalize_fk_action(left.on_update.as_deref());
    right.on_delete = normalize_fk_action(right.on_delete.as_deref());
    right.on_update = normalize_fk_action(right.on_update.as_deref());
    left == right
}

fn policies_equivalent(left: &Policy, right: &Policy) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    normalize_policy(&mut left);
    normalize_policy(&mut right);
    left == right
}

fn normalize_fk_action(action: Option<&str>) -> Option<Cow<'static, str>> {
    match action {
        None => None,
        Some(action) if action.eq_ignore_ascii_case("NO ACTION") => None,
        Some(action) => Some(Cow::Owned(action.to_ascii_uppercase())),
    }
}

fn normalize_policy(policy: &mut Policy) {
    policy.as_clause = Some(Cow::Owned(
        policy
            .as_clause
            .as_deref()
            .unwrap_or("PERMISSIVE")
            .to_ascii_uppercase(),
    ));
    policy.for_clause = Some(Cow::Owned(
        policy
            .for_clause
            .as_deref()
            .unwrap_or("ALL")
            .to_ascii_uppercase(),
    ));
    if let Some(roles) = policy.to.as_mut() {
        for role in roles {
            if role.eq_ignore_ascii_case("public") {
                *role = Cow::Borrowed("PUBLIC");
            }
        }
    }
}

fn collapse_sql_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_pg_type_for_compare(sql_type: &str) -> String {
    let mut ty = collapse_sql_whitespace(&sql_type.trim().to_ascii_lowercase());
    let mut dimensions = String::new();

    while let Some(stripped) = ty.strip_suffix("[]") {
        dimensions.push_str("[]");
        ty = stripped.trim_end().to_string();
    }

    if let Some(stripped) = ty.strip_prefix('_') {
        dimensions.push_str("[]");
        ty = stripped.to_string();
    }

    let params = ty
        .find('(')
        .map(|idx| ty[idx..].to_string())
        .unwrap_or_default();

    let canonical = match ty.as_str() {
        "int" | "int4" | "integer" => "integer".to_string(),
        "int2" | "smallint" => "smallint".to_string(),
        "int8" | "bigint" => "bigint".to_string(),
        "bool" | "boolean" => "boolean".to_string(),
        "timestamptz" | "timestamp with time zone" => "timestamp with time zone".to_string(),
        "timestamp" | "timestamp without time zone" => "timestamp".to_string(),
        "timetz" | "time with time zone" => "time with time zone".to_string(),
        "time" | "time without time zone" => "time".to_string(),
        _ if ty.starts_with("varchar") || ty.starts_with("character varying") => {
            format!("character varying{params}")
        }
        _ => match super::grammar::PgTypeCategory::from_sql_type(&ty) {
            super::grammar::PgTypeCategory::SmallInt => "smallint".to_string(),
            super::grammar::PgTypeCategory::Integer => "integer".to_string(),
            super::grammar::PgTypeCategory::BigInt => "bigint".to_string(),
            super::grammar::PgTypeCategory::Boolean => "boolean".to_string(),
            super::grammar::PgTypeCategory::Text => "text".to_string(),
            super::grammar::PgTypeCategory::Varchar => format!("character varying{params}"),
            super::grammar::PgTypeCategory::TimestampTz => "timestamp with time zone".to_string(),
            super::grammar::PgTypeCategory::Timestamp => "timestamp".to_string(),
            super::grammar::PgTypeCategory::TimeTz => "time with time zone".to_string(),
            super::grammar::PgTypeCategory::Time => "time".to_string(),
            _ => ty,
        },
    };

    format!("{canonical}{dimensions}")
}

fn normalize_column_type_for_compare(column: &Column) -> String {
    let mut sql_type = column.sql_type.to_string();
    if let Some(dimensions) = column.dimensions
        && dimensions > 0
    {
        for _ in 0..dimensions {
            sql_type.push_str("[]");
        }
    }
    normalize_pg_type_for_compare(&sql_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column_with_type(sql_type: &str) -> Column {
        Column::new("public", "users", "value", sql_type.to_string())
    }

    #[test]
    fn postgres_type_aliases_compare_equal() {
        let cases = [
            ("int4", "INTEGER"),
            ("varchar(255)", "character varying(255)"),
            ("timestamptz", "TIMESTAMP WITH TIME ZONE"),
            ("bool", "BOOLEAN"),
        ];

        for (left_type, right_type) in cases {
            let left = PostgresDDL::from_entities(vec![
                PostgresEntity::Table(Table::new("public", "users")),
                PostgresEntity::Column(column_with_type(left_type)),
            ]);
            let right = PostgresDDL::from_entities(vec![
                PostgresEntity::Table(Table::new("public", "users")),
                PostgresEntity::Column(column_with_type(right_type)),
            ]);

            let diffs = diff_ddl(&left, &right);
            assert!(
                diffs.is_empty(),
                "expected {left_type:?} and {right_type:?} to compare equal, got {diffs:?}"
            );
        }
    }

    #[test]
    fn foreign_key_no_action_compares_equal_to_omitted_actions() {
        let mut left = ForeignKey::from_strings(
            "public".to_string(),
            "posts".to_string(),
            "posts_user_fk".to_string(),
            vec!["user_id".to_string()],
            "public".to_string(),
            "users".to_string(),
            vec!["id".to_string()],
        );
        left.on_delete = Some(Cow::Borrowed("NO ACTION"));
        left.on_update = Some(Cow::Borrowed("no action"));

        let right = ForeignKey::from_strings(
            "public".to_string(),
            "posts".to_string(),
            "posts_user_fk".to_string(),
            vec!["user_id".to_string()],
            "public".to_string(),
            "users".to_string(),
            vec!["id".to_string()],
        );

        assert!(foreign_keys_equivalent(&left, &right));
    }

    #[test]
    fn public_policy_roles_compare_equal_case_insensitively() {
        let mut left = Policy::new("public", "users", "users_policy");
        left.to = Some(vec![Cow::Borrowed("public")]);

        let mut right = Policy::new("public", "users", "users_policy");
        right.as_clause = Some(Cow::Borrowed("PERMISSIVE"));
        right.for_clause = Some(Cow::Borrowed("ALL"));
        right.to = Some(vec![Cow::Borrowed("PUBLIC")]);

        assert!(policies_equivalent(&left, &right));
    }
}
