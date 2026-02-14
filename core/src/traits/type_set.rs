use core::marker::PhantomData;

/// Empty type-level set/list.
pub struct Nil;

/// Non-empty type-level set/list node.
pub struct Cons<Head, Tail>(PhantomData<(Head, Tail)>);

/// Marker trait for type-level sets/lists.
pub trait TypeSet {}

impl TypeSet for Nil {}
impl<Head, Tail> TypeSet for Cons<Head, Tail> where Tail: TypeSet {}

/// Type-level concatenation.
pub trait Concat<Rhs> {
    type Output: TypeSet;
}

impl<Rhs> Concat<Rhs> for Nil
where
    Rhs: TypeSet,
{
    type Output = Rhs;
}

impl<Head, Tail, Rhs> Concat<Rhs> for Cons<Head, Tail>
where
    Tail: Concat<Rhs> + TypeSet,
    Rhs: TypeSet,
{
    type Output = Cons<Head, <Tail as Concat<Rhs>>::Output>;
}
