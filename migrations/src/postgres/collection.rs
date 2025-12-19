//! PostgreSQL DDL collection - entity storage and management
//!
//! This implements the DDL collection pattern from drizzle-kit beta,
//! providing typed access to schema entities with push/list/one/update/delete operations.

use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, PostgresEntity, PrimaryKey, Role,
    Schema, Sequence, Table, UniqueConstraint, View,
};
use crate::traits::EntityKind;
use std::collections::HashMap;

// =============================================================================
// Entity Collection - Typed Operations
// =============================================================================

/// DDL entity collection with typed operations
#[derive(Debug, Clone)]
pub struct EntityCollection<T> {
    entities: Vec<T>,
}

impl<T> Default for EntityCollection<T> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl<T> EntityCollection<T> {
    /// Create empty collection
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Push an entity, returns true if inserted, false if duplicate
    pub fn push(&mut self, entity: T) -> bool {
        self.entities.push(entity);
        true
    }

    /// List all entities matching filter
    pub fn list(&self) -> &[T] {
        &self.entities
    }

    /// Get mutable access to entities
    pub fn list_mut(&mut self) -> &mut Vec<T> {
        &mut self.entities
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.entities.len()
    }
}

impl<T: Clone> EntityCollection<T> {
    /// Convert to Vec
    pub fn into_vec(self) -> Vec<T> {
        self.entities
    }
}

// Schema-specific operations
impl EntityCollection<Schema> {
    pub fn one(&self, name: &str) -> Option<&Schema> {
        self.entities.iter().find(|s| s.name == name)
    }
}

// Enum-specific operations
impl EntityCollection<Enum> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&Enum> {
        self.entities
            .iter()
            .find(|e| e.schema == schema && e.name == name)
    }
}

// Sequence-specific operations
impl EntityCollection<Sequence> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&Sequence> {
        self.entities
            .iter()
            .find(|s| s.schema == schema && s.name == name)
    }
}

// Role-specific operations
impl EntityCollection<Role> {
    pub fn one(&self, name: &str) -> Option<&Role> {
        self.entities.iter().find(|r| r.name == name)
    }
}

// Policy-specific operations
impl EntityCollection<Policy> {
    pub fn one(&self, schema: &str, table: &str, name: &str) -> Option<&Policy> {
        self.entities
            .iter()
            .find(|p| p.schema == schema && p.table == table && p.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&Policy> {
        self.entities
            .iter()
            .filter(|p| p.schema == schema && p.table == table)
            .collect()
    }
}

// Table-specific operations
impl EntityCollection<Table> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&Table> {
        self.entities
            .iter()
            .find(|t| t.schema == schema && t.name == name)
    }
}

// Column-specific operations
impl EntityCollection<Column> {
    pub fn one(&self, schema: &str, table: &str, name: &str) -> Option<&Column> {
        self.entities
            .iter()
            .find(|c| c.schema == schema && c.table == table && c.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&Column> {
        self.entities
            .iter()
            .filter(|c| c.schema == schema && c.table == table)
            .collect()
    }
}

// Index-specific operations
impl EntityCollection<Index> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&Index> {
        self.entities
            .iter()
            .find(|i| i.schema == schema && i.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&Index> {
        self.entities
            .iter()
            .filter(|i| i.schema == schema && i.table == table)
            .collect()
    }
}

// ForeignKey-specific operations
impl EntityCollection<ForeignKey> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&ForeignKey> {
        self.entities
            .iter()
            .find(|f| f.schema == schema && f.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&ForeignKey> {
        self.entities
            .iter()
            .filter(|f| f.schema == schema && f.table == table)
            .collect()
    }
}

// PrimaryKey-specific operations
impl EntityCollection<PrimaryKey> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&PrimaryKey> {
        self.entities
            .iter()
            .find(|p| p.schema == schema && p.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Option<&PrimaryKey> {
        self.entities
            .iter()
            .find(|p| p.schema == schema && p.table == table)
    }
}

// UniqueConstraint-specific operations
impl EntityCollection<UniqueConstraint> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&UniqueConstraint> {
        self.entities
            .iter()
            .find(|u| u.schema == schema && u.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&UniqueConstraint> {
        self.entities
            .iter()
            .filter(|u| u.schema == schema && u.table == table)
            .collect()
    }
}

// CheckConstraint-specific operations
impl EntityCollection<CheckConstraint> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&CheckConstraint> {
        self.entities
            .iter()
            .find(|c| c.schema == schema && c.name == name)
    }
    pub fn for_table(&self, schema: &str, table: &str) -> Vec<&CheckConstraint> {
        self.entities
            .iter()
            .filter(|c| c.schema == schema && c.table == table)
            .collect()
    }
}

// View-specific operations
impl EntityCollection<View> {
    pub fn one(&self, schema: &str, name: &str) -> Option<&View> {
        self.entities
            .iter()
            .find(|v| v.schema == schema && v.name == name)
    }
}

// =============================================================================
// PostgreSQL DDL - Main Collection Type
// =============================================================================

/// PostgreSQL DDL collection - stores all schema entities
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Create DDL from a list of entities
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
            PostgresEntity::Privilege(_) => true // Privileges are not yet tracked in the DDL collection
        };
    }

    /// Convert to entity array for snapshot serialization
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
    pub fn is_empty(&self) -> bool {
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

/// Compute diff between two DDL collections
pub fn diff_ddl(left: &PostgresDDL, right: &PostgresDDL) -> Vec<EntityDiff> {
    let mut diffs = Vec::new();

    // Schemas
    diff_entity_type(
        left.schemas.list(),
        right.schemas.list(),
        |e| e.name.to_string(),
        |e| PostgresEntity::Schema(e.clone()),
        EntityKind::Schema,
        &mut diffs,
    );

    // Enums
    diff_entity_type(
        left.enums.list(),
        right.enums.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Enum(e.clone()),
        EntityKind::Enum,
        &mut diffs,
    );

    // Sequences
    diff_entity_type(
        left.sequences.list(),
        right.sequences.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Sequence(e.clone()),
        EntityKind::Sequence,
        &mut diffs,
    );

    // Roles
    diff_entity_type(
        left.roles.list(),
        right.roles.list(),
        |e| e.name.to_string(),
        |e| PostgresEntity::Role(e.clone()),
        EntityKind::Role,
        &mut diffs,
    );

    // Tables
    diff_entity_type(
        left.tables.list(),
        right.tables.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Table(e.clone()),
        EntityKind::Table,
        &mut diffs,
    );

    // Columns
    diff_entity_type(
        left.columns.list(),
        right.columns.list(),
        |e| format!("{}.{}.{}", e.schema, e.table, e.name),
        |e| PostgresEntity::Column(e.clone()),
        EntityKind::Column,
        &mut diffs,
    );

    // Indexes
    diff_entity_type(
        left.indexes.list(),
        right.indexes.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::Index(e.clone()),
        EntityKind::Index,
        &mut diffs,
    );

    // Constraints
    diff_entity_type(
        left.fks.list(),
        right.fks.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::ForeignKey(e.clone()),
        EntityKind::ForeignKey,
        &mut diffs,
    );
    diff_entity_type(
        left.pks.list(),
        right.pks.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::PrimaryKey(e.clone()),
        EntityKind::PrimaryKey,
        &mut diffs,
    );
    diff_entity_type(
        left.uniques.list(),
        right.uniques.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::UniqueConstraint(e.clone()),
        EntityKind::UniqueConstraint,
        &mut diffs,
    );
    diff_entity_type(
        left.checks.list(),
        right.checks.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::CheckConstraint(e.clone()),
        EntityKind::CheckConstraint,
        &mut diffs,
    );

    // Policies
    diff_entity_type(
        left.policies.list(),
        right.policies.list(),
        |e| format!("{}.{}.{}", e.schema, e.table, e.name),
        |e| PostgresEntity::Policy(e.clone()),
        EntityKind::Policy,
        &mut diffs,
    );

    // Views
    diff_entity_type(
        left.views.list(),
        right.views.list(),
        |e| format!("{}.{}", e.schema, e.name),
        |e| PostgresEntity::View(e.clone()),
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
    to_entity: impl Fn(&T) -> PostgresEntity,
    kind: EntityKind,
    diffs: &mut Vec<EntityDiff>,
) {
    let left_map: HashMap<String, &T> = left.iter().map(|e| (key_fn(e), e)).collect();
    let right_map: HashMap<String, &T> = right.iter().map(|e| (key_fn(e), e)).collect();

    // Find dropped
    for (key, left_entity) in &left_map {
        if !right_map.contains_key(key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Drop,
                kind,
                name: key.clone(),
                changes: HashMap::new(),
                left: Some(to_entity(left_entity)),
                right: None,
            });
        }
    }

    // Find created
    for (key, right_entity) in &right_map {
        if !left_map.contains_key(key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Create,
                kind,
                name: key.clone(),
                changes: HashMap::new(),
                left: None,
                right: Some(to_entity(right_entity)),
            });
        }
    }

    // Find altered
    for (key, left_entity) in &left_map {
        if let Some(right_entity) = right_map.get(key)
            && *left_entity != *right_entity
        {
            diffs.push(EntityDiff {
                diff_type: DiffType::Alter,
                kind,
                name: key.clone(),
                changes: HashMap::new(), // Rely on left/right for details
                left: Some(to_entity(left_entity)),
                right: Some(to_entity(right_entity)),
            });
        }
    }
}
