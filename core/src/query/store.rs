//! `RelEntry<Rel, Data, Rest>` — type-level linked list for relation data storage.

use core::marker::PhantomData;

/// Stores one relation's data plus the rest of the chain.
///
/// A `QueryRow` with two included relations has store type:
/// ```text
/// RelEntry<RelPosts, Vec<SelectPost>, RelEntry<RelInvitedBy, Option<SelectUser>, ()>>
/// ```
#[derive(Debug, Clone)]
pub struct RelEntry<Rel, Data, Rest> {
    pub(crate) data: Data,
    pub(crate) rest: Rest,
    pub(crate) _rel: PhantomData<Rel>,
}

impl<Rel, Data, Rest> RelEntry<Rel, Data, Rest> {
    /// Creates a new `RelEntry`.
    pub(crate) fn new(data: Data, rest: Rest) -> Self {
        Self {
            data,
            rest,
            _rel: PhantomData,
        }
    }
}
