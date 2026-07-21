//! `RelEntry<Rel, Data, Rest>` — type-level linked list for relation data storage.

use core::marker::PhantomData;

/// One loaded relation's data plus the remaining store chain.
///
/// Built during JSON column decode and consumed by [`super::BuildRow`].
#[derive(Debug, Clone)]
pub struct RelEntry<Rel, Data, Rest> {
    pub(crate) data: Data,
    pub(crate) rest: Rest,
    pub(crate) _rel: PhantomData<Rel>,
}

impl<Rel, Data, Rest> RelEntry<Rel, Data, Rest> {
    /// Creates a new `RelEntry`.
    pub(crate) const fn new(data: Data, rest: Rest) -> Self {
        Self {
            data,
            rest,
            _rel: PhantomData,
        }
    }

    /// Splits this entry into its relation data and the remaining chain.
    pub(crate) fn into_parts(self) -> (Data, Rest) {
        (self.data, self.rest)
    }
}
