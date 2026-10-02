//! [`EntityCollection`], the ordered list that holds DDL entities.
//!
//! Each dialect's DDL model (`SQLiteDDL`, `PostgresDDL`, `MySQLDDL`) stores
//! its tables, columns, indexes, and so on in one `EntityCollection` per
//! entity kind. The generic operations live here; the `sqlite`, `postgres`,
//! and `mysql` `collection` modules add typed lookups (`one(...)`,
//! `for_table(...)`, ...) keyed the way each dialect identifies an entity.
//!
//! ## Why a Vec wrapper, not an indexed map
//!
//! The serializer emits entities in insertion order (the order the user
//! declared them), which a `Vec` preserves for free. Duplicate keys are
//! allowed: [`push`](EntityCollection::push) always succeeds, and callers
//! de-duplicate when they need to.

// =============================================================================
// Entity Collection - Typed Operations
// =============================================================================

/// An insertion-ordered list of DDL entities of one kind.
///
/// See the [module docs](self) for why this is a `Vec` and not a map.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::EntityCollection;
///
/// let mut names = EntityCollection::new();
/// names.push("users");
/// names.extend(["posts", "comments"]);
/// assert_eq!(names.list(), &["users", "posts", "comments"]);
/// ```
#[derive(Debug, Clone)]
pub struct EntityCollection<T> {
    // Crate-visible so the per-dialect `impl EntityCollection<...>` blocks
    // can reach the Vec directly. Not part of the public API.
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
    /// Creates an empty collection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Appends an entity. Does not check for duplicates.
    pub fn push(&mut self, entity: T) {
        self.entities.push(entity);
    }

    /// Returns all entities in insertion order.
    #[must_use]
    pub fn list(&self) -> &[T] {
        &self.entities
    }

    /// Returns the underlying `Vec` for in-place edits.
    pub const fn list_mut(&mut self) -> &mut Vec<T> {
        &mut self.entities
    }

    /// Returns `true` if there are no entities.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns the number of entities.
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
    /// Returns the underlying `Vec`.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.entities
    }

    /// Runs `transform` on every entity for which `predicate` returns `true`.
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

    /// Runs `transform` on every entity.
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
