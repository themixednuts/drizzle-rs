//! Shared generic entity collection for DDL storage.
//!
//! [`EntityCollection<T>`] is a thin `Vec<T>` wrapper that backs both the
//! SQLite and Postgres DDL pipelines. The generic operations (push, list,
//! is_empty, len, mutable access, ...) live here once; per-dialect `impl
//! EntityCollection<DialectEntity>` blocks in `sqlite/collection.rs` and
//! `postgres/collection.rs` add typed lookup helpers (`one(name)`,
//! `for_table(table)`, etc.) whose shape depends on the dialect's entity
//! identity (single-name vs (schema, name) vs (schema, table, name)).
//!
//! ## Why a Vec wrapper, not an indexed map
//!
//! The DDL serializer needs to emit entities in insertion order (the order
//! the user declared them). A Vec preserves that for free; a HashMap would
//! need a parallel ordering structure. Duplicate keys are also allowed
//! during partial state — `push` always succeeds, callers de-duplicate
//! explicitly when they need uniqueness.

// =============================================================================
// Entity Collection - Typed Operations
// =============================================================================

/// Generic DDL entity collection with typed operations.
///
/// See module docs for the design rationale. Per-dialect `impl
/// EntityCollection<…>` blocks supplying entity-aware lookups live in
/// `sqlite/collection.rs` and `postgres/collection.rs`.
#[derive(Debug, Clone)]
pub struct EntityCollection<T> {
    /// Crate-private so per-dialect `impl EntityCollection<DialectEntity>`
    /// blocks (in `sqlite/collection.rs` and `postgres/collection.rs`) can
    /// supply entity-aware lookup helpers without going through accessor
    /// methods. Not part of the public API.
    pub(crate) entities: Vec<T>,
}

impl<T> Default for EntityCollection<T> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl<T> EntityCollection<T> {
    /// Create empty collection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Push an entity. Always succeeds — duplicate detection is the
    /// caller's responsibility (see module docs).
    pub fn push(&mut self, entity: T) {
        self.entities.push(entity);
    }

    /// List all entities in insertion order.
    #[must_use]
    pub fn list(&self) -> &[T] {
        &self.entities
    }

    /// Mutable access to the underlying `Vec`.
    pub const fn list_mut(&mut self) -> &mut Vec<T> {
        &mut self.entities
    }

    /// Check if empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Number of entities currently in the collection.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entities.len()
    }
}

impl<T> Extend<T> for EntityCollection<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.entities.extend(iter);
    }
}

impl<T: Clone> EntityCollection<T> {
    /// Consume the collection and return the underlying `Vec`.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.entities
    }

    /// Update entities matching `predicate` with `transform`.
    pub fn update_where<F, P>(&mut self, predicate: P, mut transform: F)
    where
        F: FnMut(&mut T),
        P: Fn(&T) -> bool,
    {
        for entity in &mut self.entities {
            if predicate(entity) {
                transform(entity);
            }
        }
    }

    /// Update every entity with `transform`.
    pub fn update_all<F>(&mut self, mut transform: F)
    where
        F: FnMut(&mut T),
    {
        for entity in &mut self.entities {
            transform(entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EntityCollection;

    #[test]
    fn extend_preserves_insertion_order() {
        let mut entities = EntityCollection::new();
        entities.push(1);
        entities.extend([2, 3]);

        assert_eq!(entities.list(), &[1, 2, 3]);
    }
}
