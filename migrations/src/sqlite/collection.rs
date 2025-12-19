//! SQLite DDL collection - entity storage and management
//!
//! This implements the DDL collection pattern from drizzle-kit beta,
//! providing typed access to schema entities with push/list/one/update/delete operations.

use super::ddl::{
    CheckConstraint, Column, ForeignKey, Index, PrimaryKey, SqliteEntity, Table, UniqueConstraint,
    View,
};
use crate::traits::EntityKind;
use std::collections::HashMap;

// =============================================================================
// Entity Collection - Typed Operations
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

    /// Update entities matching a filter with a transformation function
    pub fn update_where<F, P>(&mut self, predicate: P, mut transform: F)
    where
        F: FnMut(&mut T),
        P: Fn(&T) -> bool,
    {
        for entity in self.entities.iter_mut() {
            if predicate(entity) {
                transform(entity);
            }
        }
    }

    /// Update all entities with a transformation function
    pub fn update_all<F>(&mut self, mut transform: F)
    where
        F: FnMut(&mut T),
    {
        for entity in self.entities.iter_mut() {
            transform(entity);
        }
    }
}

// Table-specific operations
impl EntityCollection<Table> {
    /// Find a table by name
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
    pub fn one(&self, table: &str, name: &str) -> Option<&Column> {
        self.entities
            .iter()
            .find(|c| c.table == table && c.name == name)
    }

    /// List columns for a table
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
    pub fn one(&self, name: &str) -> Option<&Index> {
        self.entities.iter().find(|i| i.name == name)
    }

    /// List indexes for a table
    pub fn for_table(&self, table: &str) -> Vec<&Index> {
        self.entities.iter().filter(|i| i.table == table).collect()
    }
}

// ForeignKey-specific operations
impl EntityCollection<ForeignKey> {
    /// Find a foreign key by name
    pub fn one(&self, name: &str) -> Option<&ForeignKey> {
        self.entities.iter().find(|f| f.name == name)
    }

    /// List foreign keys for a table
    pub fn for_table(&self, table: &str) -> Vec<&ForeignKey> {
        self.entities.iter().filter(|f| f.table == table).collect()
    }
}

// PrimaryKey-specific operations
impl EntityCollection<PrimaryKey> {
    /// Find a primary key by table
    pub fn for_table(&self, table: &str) -> Option<&PrimaryKey> {
        self.entities.iter().find(|p| p.table == table)
    }
}

// UniqueConstraint-specific operations
impl EntityCollection<UniqueConstraint> {
    /// Find by name
    pub fn one(&self, name: &str) -> Option<&UniqueConstraint> {
        self.entities.iter().find(|u| u.name == name)
    }

    /// List for a table
    pub fn for_table(&self, table: &str) -> Vec<&UniqueConstraint> {
        self.entities.iter().filter(|u| u.table == table).collect()
    }
}

// CheckConstraint-specific operations
impl EntityCollection<CheckConstraint> {
    /// Find by name
    pub fn one(&self, name: &str) -> Option<&CheckConstraint> {
        self.entities.iter().find(|c| c.name == name)
    }

    /// List for a table
    pub fn for_table(&self, table: &str) -> Vec<&CheckConstraint> {
        self.entities.iter().filter(|c| c.table == table).collect()
    }
}

// View-specific operations
impl EntityCollection<View> {
    /// Find a view by name
    pub fn one(&self, name: &str) -> Option<&View> {
        self.entities.iter().find(|v| v.name == name)
    }
}

// =============================================================================
// SQLite DDL - Main Collection Type
// =============================================================================

/// SQLite DDL collection - stores all schema entities
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Create DDL from a list of entities
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
    pub fn is_empty(&self) -> bool {
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
pub fn diff_ddl(left: &SQLiteDDL, right: &SQLiteDDL) -> Vec<EntityDiff> {
    let mut diffs = Vec::new();

    // Diff tables
    diff_entity_type(
        left.tables.list(),
        right.tables.list(),
        |t| t.name.to_string(),
        |t| SqliteEntity::Table(t.clone()),
        EntityKind::Table,
        &mut diffs,
    );

    // Diff columns
    diff_entity_type(
        left.columns.list(),
        right.columns.list(),
        |c| format!("{}:{}", c.table, c.name),
        |c| SqliteEntity::Column(c.clone()),
        EntityKind::Column,
        &mut diffs,
    );

    // Diff indexes
    diff_entity_type(
        left.indexes.list(),
        right.indexes.list(),
        |i| i.name.to_string(),
        |i| SqliteEntity::Index(i.clone()),
        EntityKind::Index,
        &mut diffs,
    );

    // Diff foreign keys
    diff_entity_type(
        left.fks.list(),
        right.fks.list(),
        |f| f.name.to_string(),
        |f| SqliteEntity::ForeignKey(f.clone()),
        EntityKind::ForeignKey,
        &mut diffs,
    );

    // Diff primary keys
    diff_entity_type(
        left.pks.list(),
        right.pks.list(),
        |p| p.table.to_string(),
        |p| SqliteEntity::PrimaryKey(p.clone()),
        EntityKind::PrimaryKey,
        &mut diffs,
    );

    // Diff unique constraints
    diff_entity_type(
        left.uniques.list(),
        right.uniques.list(),
        |u| u.name.to_string(),
        |u| SqliteEntity::UniqueConstraint(u.clone()),
        EntityKind::UniqueConstraint,
        &mut diffs,
    );

    // Diff check constraints
    diff_entity_type(
        left.checks.list(),
        right.checks.list(),
        |c| c.name.to_string(),
        |c| SqliteEntity::CheckConstraint(c.clone()),
        EntityKind::CheckConstraint,
        &mut diffs,
    );

    // Diff views
    diff_entity_type(
        left.views.list(),
        right.views.list(),
        |v| v.name.to_string(),
        |v| SqliteEntity::View(v.clone()),
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
    kind: EntityKind,
    diffs: &mut Vec<EntityDiff>,
) {
    let left_map: HashMap<String, &T> = left.iter().map(|e| (key_fn(e), e)).collect();
    let right_map: HashMap<String, &T> = right.iter().map(|e| (key_fn(e), e)).collect();

    // Find dropped (in left but not in right)
    for (key, left_entity) in &left_map {
        if !right_map.contains_key(key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Drop,
                kind,
                table: None,
                name: key.clone(),
                changes: HashMap::new(),
                left: Some(to_entity(left_entity)),
                right: None,
            });
        }
    }

    // Find created (in right but not in left)
    for (key, right_entity) in &right_map {
        if !left_map.contains_key(key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Create,
                kind,
                table: None,
                name: key.clone(),
                changes: HashMap::new(),
                left: None,
                right: Some(to_entity(right_entity)),
            });
        }
    }

    // Find altered (in both, but different)
    for (key, left_entity) in &left_map {
        if let Some(right_entity) = right_map.get(key) {
            if *left_entity != *right_entity {
                diffs.push(EntityDiff {
                    diff_type: DiffType::Alter,
                    kind,
                    table: None,
                    name: key.clone(),
                    changes: HashMap::new(), // Field-level comparison available via left/right entities
                    left: Some(to_entity(left_entity)),
                    right: Some(to_entity(right_entity)),
                });
            }
        }
    }
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
}
