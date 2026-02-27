//! Type-level witness search for relation data in a `RelEntry` chain.

use core::marker::PhantomData;

use super::store::RelEntry;

/// Witness: the relation was found at the head of the chain.
pub struct Here;

/// Witness: the relation was found deeper in the chain.
pub struct There<W>(PhantomData<W>);

/// Type-level search for relation `Rel` in a `RelEntry` chain.
///
/// The witness `W` is inferred by the compiler (unique because each
/// relation ZST appears at most once in the chain).
pub trait FindRel<Rel, Witness> {
    /// The data type stored for this relation.
    type Data;
    /// Returns a reference to the stored data.
    fn get(&self) -> &Self::Data;
}

// Base case: found at head of chain.
impl<Rel, Data, Rest> FindRel<Rel, Here> for RelEntry<Rel, Data, Rest> {
    type Data = Data;

    fn get(&self) -> &Data {
        &self.data
    }
}

// Recursive case: search in the tail.
impl<Rel, Other, Data, Rest, W> FindRel<Rel, There<W>> for RelEntry<Other, Data, Rest>
where
    Rest: FindRel<Rel, W>,
{
    type Data = <Rest as FindRel<Rel, W>>::Data;

    fn get(&self) -> &Self::Data {
        self.rest.get()
    }
}
