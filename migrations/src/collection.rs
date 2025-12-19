//! Generic entity collection for DDL storage
//!
//! This module provides a shared `Collection<E>` type that works with any
//! entity implementing the `Entity` trait. Used by both SQLite and PostgreSQL.

use crate::traits::{DiffType, Entity, EntityKey, EntityKind};
use std::collections::HashMap;

// =============================================================================
// Generic Entity Collection
// =============================================================================

/// Generic collection for any DDL entity type.
///
/// Provides O(1) lookup via an internal index, along with standard
/// collection operations like push, list, delete.
#[derive(Debug, Clone)]
pub struct Collection<E: Entity> {
    entities: Vec<E>,
    /// Index from entity key to position for fast lookups
    index: HashMap<EntityKey, usize>,
}

impl<E: Entity> Default for Collection<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Entity> Collection<E> {
    /// Create an empty collection
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Push an entity, returns true if inserted, false if duplicate key
    pub fn push(&mut self, entity: E) -> bool {
        let key = entity.key();
        if self.index.contains_key(&key) {
            return false;
        }
        let idx = self.entities.len();
        self.entities.push(entity);
        self.index.insert(key, idx);
        true
    }

    /// Get an entity by its key
    pub fn get(&self, key: &EntityKey) -> Option<&E> {
        self.index.get(key).map(|&idx| &self.entities[idx])
    }

    /// Check if an entity with the given key exists
    pub fn contains(&self, key: &EntityKey) -> bool {
        self.index.contains_key(key)
    }

    /// Delete an entity by key, returns the removed entity if found
    pub fn delete(&mut self, key: &EntityKey) -> Option<E> {
        if let Some(&idx) = self.index.get(key) {
            self.index.remove(key);
            // Swap remove and update index of moved element
            let removed = self.entities.swap_remove(idx);
            if idx < self.entities.len() {
                // Update index for the element that was swapped in
                let swapped_key = self.entities[idx].key();
                self.index.insert(swapped_key, idx);
            }
            Some(removed)
        } else {
            None
        }
    }

    /// List all entities
    pub fn list(&self) -> &[E] {
        &self.entities
    }

    /// Get mutable access to entities (invalidates index!)
    /// Use with caution - prefer update_where for safe mutations
    pub fn list_mut(&mut self) -> &mut Vec<E> {
        &mut self.entities
    }

    /// Rebuild the index (call after list_mut modifications)
    pub fn rebuild_index(&mut self) {
        self.index.clear();
        for (idx, entity) in self.entities.iter().enumerate() {
            self.index.insert(entity.key(), idx);
        }
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get the count of entities
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Iterate over entities
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.entities.iter()
    }

    /// Convert to Vec, consuming the collection
    pub fn into_vec(self) -> Vec<E> {
        self.entities
    }

    /// Update entities matching a predicate
    pub fn update_where<P, F>(&mut self, predicate: P, mut transform: F)
    where
        P: Fn(&E) -> bool,
        F: FnMut(&mut E),
    {
        for entity in self.entities.iter_mut() {
            if predicate(entity) {
                transform(entity);
            }
        }
        // Rebuild index in case keys changed
        self.rebuild_index();
    }

    /// Filter entities matching a predicate
    pub fn filter<P>(&self, predicate: P) -> Vec<&E>
    where
        P: Fn(&E) -> bool,
    {
        self.entities.iter().filter(|e| predicate(*e)).collect()
    }
}

// =============================================================================
// Entity Diff
// =============================================================================

/// A diff entry for any entity type
#[derive(Debug, Clone)]
pub struct EntityDiff<E: Entity> {
    /// The type of diff operation
    pub diff_type: DiffType,
    /// The entity key
    pub key: EntityKey,
    /// Original entity (for Drop/Alter)
    pub left: Option<E>,
    /// New entity (for Create/Alter)
    pub right: Option<E>,
}

impl<E: Entity> EntityDiff<E> {
    /// Get the entity kind
    pub fn kind(&self) -> EntityKind {
        E::KIND
    }
}

/// Compute diff between two collections of the same entity type
pub fn diff_collections<E: Entity>(
    left: &Collection<E>,
    right: &Collection<E>,
) -> Vec<EntityDiff<E>> {
    let mut diffs = Vec::new();

    // Find dropped (in left but not in right)
    for entity in left.iter() {
        let key = entity.key();
        if !right.contains(&key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Drop,
                key,
                left: Some(entity.clone()),
                right: None,
            });
        }
    }

    // Find created (in right but not in left)
    for entity in right.iter() {
        let key = entity.key();
        if !left.contains(&key) {
            diffs.push(EntityDiff {
                diff_type: DiffType::Create,
                key,
                left: None,
                right: Some(entity.clone()),
            });
        }
    }

    // Find altered (in both but different)
    for left_entity in left.iter() {
        let key = left_entity.key();
        if let Some(right_entity) = right.get(&key)
            && left_entity != right_entity
        {
            diffs.push(EntityDiff {
                diff_type: DiffType::Alter,
                key,
                left: Some(left_entity.clone()),
                right: Some(right_entity.clone()),
            });
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test entity
    #[derive(Clone, Debug, PartialEq)]
    struct TestEntity {
        name: String,
        value: i32,
    }

    impl Entity for TestEntity {
        const KIND: EntityKind = EntityKind::Table;

        fn key(&self) -> EntityKey {
            EntityKey::simple(&self.name)
        }
    }

    #[test]
    fn test_collection_push_and_get() {
        let mut col: Collection<TestEntity> = Collection::new();

        let e1 = TestEntity {
            name: "foo".into(),
            value: 1,
        };
        assert!(col.push(e1.clone()));

        // Duplicate should fail
        let e1_dup = TestEntity {
            name: "foo".into(),
            value: 2,
        };
        assert!(!col.push(e1_dup));

        // Get should return original
        let key = EntityKey::simple("foo");
        let got = col.get(&key).unwrap();
        assert_eq!(got.value, 1);
    }

    #[test]
    fn test_collection_delete() {
        let mut col: Collection<TestEntity> = Collection::new();
        col.push(TestEntity {
            name: "a".into(),
            value: 1,
        });
        col.push(TestEntity {
            name: "b".into(),
            value: 2,
        });
        col.push(TestEntity {
            name: "c".into(),
            value: 3,
        });

        let key_b = EntityKey::simple("b");
        let removed = col.delete(&key_b);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "b");

        assert_eq!(col.len(), 2);
        assert!(!col.contains(&key_b));

        // Other elements still accessible
        assert!(col.contains(&EntityKey::simple("a")));
        assert!(col.contains(&EntityKey::simple("c")));
    }

    #[test]
    fn test_diff_collections() {
        let mut left: Collection<TestEntity> = Collection::new();
        left.push(TestEntity {
            name: "keep".into(),
            value: 1,
        });
        left.push(TestEntity {
            name: "drop".into(),
            value: 2,
        });
        left.push(TestEntity {
            name: "alter".into(),
            value: 3,
        });

        let mut right: Collection<TestEntity> = Collection::new();
        right.push(TestEntity {
            name: "keep".into(),
            value: 1,
        });
        right.push(TestEntity {
            name: "create".into(),
            value: 4,
        });
        right.push(TestEntity {
            name: "alter".into(),
            value: 99,
        }); // Changed value

        let diffs = diff_collections(&left, &right);

        assert_eq!(diffs.len(), 3);

        let dropped: Vec<_> = diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop)
            .collect();
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].left.as_ref().unwrap().name, "drop");

        let created: Vec<_> = diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create)
            .collect();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].right.as_ref().unwrap().name, "create");

        let altered: Vec<_> = diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Alter)
            .collect();
        assert_eq!(altered.len(), 1);
        assert_eq!(altered[0].left.as_ref().unwrap().value, 3);
        assert_eq!(altered[0].right.as_ref().unwrap().value, 99);
    }
}
