use core::marker::PhantomData;

/// The empty type-level list.
///
/// Type-level lists (`Cons<A, Cons<B, Nil>>`) let traits work over a list of
/// types, such as a query's tables or a SELECT's columns.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nil;

/// A type-level list node: `Head` followed by the list `Tail`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cons<Head, Tail>(PhantomData<(Head, Tail)>);

/// A type-level list: [`Nil`] or a [`Cons`] ending in `Nil`.
pub trait TypeSet {}

impl TypeSet for Nil {}
impl<Head, Tail> TypeSet for Cons<Head, Tail> where Tail: TypeSet {}

/// Appends the list `Rhs` to the list `Self`.
pub trait Concat<Rhs> {
    /// The joined list.
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
