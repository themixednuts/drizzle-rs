//! `QueryRow<Base, Store>` — nested relation JSON decode target.

use core::ops::Deref;

/// A base select model paired with a decoded relation store.
///
/// Used while deserializing nested relation JSON columns. Public query APIs
/// assemble this into generated `*With*` row structs via [`super::BuildRow`].
#[derive(Debug, Clone)]
pub struct QueryRow<Base, Store = ()> {
    base: Base,
    #[doc(hidden)]
    pub store: Store,
}

impl<Base, Store> QueryRow<Base, Store> {
    /// Creates a new `QueryRow` with the given base model and relation store.
    pub const fn new(base: Base, store: Store) -> Self {
        Self { base, store }
    }

    /// Returns a reference to the base model.
    pub const fn base(&self) -> &Base {
        &self.base
    }

    /// Consumes the `QueryRow` and returns the base model and store.
    pub fn into_parts(self) -> (Base, Store) {
        (self.base, self.store)
    }
}

impl<Base, Store> Deref for QueryRow<Base, Store> {
    type Target = Base;

    fn deref(&self) -> &Base {
        &self.base
    }
}
