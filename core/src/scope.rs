//! Type-level query scope.
//!
//! A SELECT carries its FROM/JOIN sources as a type-level list (the *scope*).
//! Every expression carries the sources it reads as a type-level tree (its
//! [`Sources`](crate::expr::ExprSources::Sources)). Checking a clause is one
//! question: does every source in the expression's tree appear in the scope?
//!
//! The same walk also answers how NULL reaches the expression's value. A
//! source on the nullable side of an outer join behaves like a table whose
//! every column is nullable, and the tree records how each operator passes
//! NULL from its operands to its result.
//!
//! # Scope
//!
//! The scope is `Cons<Entry, Cons<Entry, ... Nil>>`, newest source first.
//! Each entry implements [`ScopeEntry`], which names the key that columns
//! use to refer to it and whether an outer join can leave it NULL.
//!
//! # Sources tree
//!
//! | Node | Meaning | NULL when |
//! |------|---------|-----------|
//! | `()` | reads no source | never (from outer joins) |
//! | [`Src<T>`] | a column of source `T` | `T` is outer-joined |
//! | `(A, B)` | both, NULL-propagating | `A` or `B` is NULL |
//! | [`Coalesce<A, B>`] | both, NULL-absorbing | `A` and `B` are NULL |
//! | [`NonNull`] / [`Null`] | an operand's declared nullability | `Null` |
//!
//! Operators whose result is never NULL (`IS NULL`, `COUNT`, `EXISTS`) wrap
//! their operands in [`ScopeOnly`], so the operands are still scope-checked
//! but cannot make the result NULL.

use core::marker::PhantomData;

use crate::expr::{NonNull, Null, NullAnd, NullOr, Nullability};
use crate::{Cons, Nil};

// =============================================================================
// Scope entries
// =============================================================================

/// A source that can appear in a query scope.
///
/// Tables and views are their own key. Aliased sources (`Table::alias::<Tag>()`,
/// derived tables, CTEs) are keyed by [`AliasKey<Tag>`], because the alias name
/// is what SQL resolves their columns against.
pub trait ScopeEntry {
    /// The identity columns use to refer to this source.
    type Key;
    /// [`Null`] when an outer join can leave this source NULL.
    type Nullable: Nullability;
    /// Sources this source reads itself: a derived table's subquery. They
    /// resolve against the enclosing query (or, for a `LATERAL` join, the
    /// sources joined before it). Tables and views read nothing (`()`).
    type Sources;
}

impl<T: ScopeEntry + ?Sized> ScopeEntry for &T {
    type Key = T::Key;
    type Nullable = T::Nullable;
    type Sources = T::Sources;
}

/// Key of a raw SQL source (`.from(sql)`): no typed column refers to it.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawSource;

impl<V: crate::SQLParam> ScopeEntry for crate::SQL<'_, V> {
    type Key = RawSource;
    type Nullable = NonNull;
    type Sources = ();
}

/// Key for a source referred to by an alias name.
#[derive(Debug, Clone, Copy, Default)]
pub struct AliasKey<Tag>(PhantomData<Tag>);

impl<Tag> ScopeEntry for AliasKey<Tag> {
    type Key = Self;
    type Nullable = NonNull;
    type Sources = ();
}

/// Scope entry for a source on the nullable side of an outer join.
///
/// `LEFT JOIN` wraps the joined source, `RIGHT JOIN` wraps every source
/// already in scope, and `FULL JOIN` wraps both.
#[derive(Debug, Clone, Copy, Default)]
pub struct OuterJoined<T>(PhantomData<T>);

impl<T: ScopeEntry> ScopeEntry for OuterJoined<T> {
    type Key = T::Key;
    type Nullable = Null;
    // Recorded when the source was joined.
    type Sources = ();
}

/// Type-level witness that a source is the head of a scope list.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeHere;

/// Type-level witness that a source is deeper in a scope list.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeThere<Prev>(PhantomData<Prev>);

/// Witness for a table or view key, found by name comparison.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeFound;

/// The scope has a source whose key is `Key`.
///
/// Tables and views are looked up by SQL name, so the first (innermost)
/// matching source wins: a correlated subquery that reads the same table as
/// its outer query resolves to its own copy, as in SQL. Aliased sources are
/// looked up by alias type.
#[diagnostic::on_unimplemented(
    message = "`{Key}` is not in this query's FROM/JOIN scope",
    label = "this expression reads a source that the query never joins",
    note = "add the source with .from(...) or a .join(...) before using its columns"
)]
pub trait ScopeContains<Key, Witness> {
    /// [`Null`] when the found source is on the nullable side of an outer join.
    type Nullable: Nullability;
}

impl<Name, Table, Scope> ScopeContains<TableKey<Name, Table>, ScopeFound> for Scope
where
    Scope: FindTable<Name, Table>,
{
    type Nullable = Scope::Nullable;
}

impl<Tag, Head, Tail> ScopeContains<AliasKey<Tag>, ScopeHere> for Cons<Head, Tail>
where
    Head: ScopeEntry<Key = AliasKey<Tag>>,
{
    type Nullable = Head::Nullable;
}

impl<Tag, Head, Tail, Witness> ScopeContains<AliasKey<Tag>, ScopeThere<Witness>>
    for Cons<Head, Tail>
where
    Tail: ScopeContains<AliasKey<Tag>, Witness>,
{
    type Nullable = Tail::Nullable;
}

// =============================================================================
// Table keys: decidable comparison by SQL name
// =============================================================================

/// Key of a table or view: its SQL name as a type-level list of nibbles
/// (see [`name`]), plus the Rust type for diagnostics.
#[derive(Debug, Clone, Copy, Default)]
pub struct TableKey<Name, Table>(PhantomData<(Name, Table)>);

/// Type-level boolean `true`.
#[derive(Debug, Clone, Copy, Default)]
pub struct True;
/// Type-level boolean `false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct False;

/// Nibbles that spell a table's SQL name, two per byte.
pub mod name {
    use super::{False, True};

    macro_rules! nibbles {
        ($($n:ident),+) => {
            $(
                #[doc(hidden)]
                #[derive(Debug, Clone, Copy, Default)]
                pub struct $n;
            )+
        };
    }

    nibbles!(H0, H1, H2, H3, H4, H5, H6, H7, H8, H9, HA, HB, HC, HD, HE, HF);

    /// Nibble equality.
    pub trait NibEq<Other> {
        type Out;
    }

    macro_rules! nib_eq {
        () => {};
        ($head:ident $(, $rest:ident)*) => {
            impl NibEq<$head> for $head {
                type Out = True;
            }
            $(
                impl NibEq<$rest> for $head {
                    type Out = False;
                }
                impl NibEq<$head> for $rest {
                    type Out = False;
                }
            )*
            nib_eq!($($rest),*);
        };
    }

    nib_eq!(H0, H1, H2, H3, H4, H5, H6, H7, H8, H9, HA, HB, HC, HD, HE, HF);
}

/// Equality of two nibble lists.
#[doc(hidden)]
pub trait NameEq<Other> {
    type Out;
}

impl NameEq<Nil> for Nil {
    type Out = True;
}

impl<H, T> NameEq<Cons<H, T>> for Nil {
    type Out = False;
}

impl<H, T> NameEq<Nil> for Cons<H, T> {
    type Out = False;
}

impl<HA, TA, HB, TB> NameEq<Cons<HB, TB>> for Cons<HA, TA>
where
    HA: name::NibEq<HB>,
    HA::Out: NameEqRest<TA, TB>,
{
    type Out = <HA::Out as NameEqRest<TA, TB>>::Out;
}

/// Compares the rest of two names only while the prefixes match.
#[doc(hidden)]
pub trait NameEqRest<A, B> {
    type Out;
}

impl<A: NameEq<B>, B> NameEqRest<A, B> for True {
    type Out = A::Out;
}

impl<A, B> NameEqRest<A, B> for False {
    type Out = False;
}

/// Whether a scope entry's key is the table named `Name`.
#[doc(hidden)]
pub trait IsTable<Name> {
    type Out;
}

impl<Name, Other: NameEq<Name>, T> IsTable<Name> for TableKey<Other, T> {
    type Out = Other::Out;
}

impl<Name, Tag> IsTable<Name> for AliasKey<Tag> {
    type Out = False;
}

impl<Name> IsTable<Name> for RawSource {
    type Out = False;
}

/// Finds the first table named `Name` in a scope list.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Table}` is not in this query's FROM/JOIN scope",
    label = "this expression reads a table that the query never joins",
    note = "add the table with .from(...) or a .join(...) before using its columns"
)]
pub trait FindTable<Name, Table> {
    type Nullable: Nullability;
}

impl<Name, Table, Head, Tail> FindTable<Name, Table> for Cons<Head, Tail>
where
    Head: ScopeEntry,
    Head::Key: IsTable<Name>,
    <Head::Key as IsTable<Name>>::Out: FoundOr<Head::Nullable, Tail, Name, Table>,
{
    type Nullable =
        <<Head::Key as IsTable<Name>>::Out as FoundOr<Head::Nullable, Tail, Name, Table>>::Nullable;
}

/// `True`: the head matched; `False`: keep searching the tail.
#[doc(hidden)]
pub trait FoundOr<Nullable, Rest, Name, Table> {
    type Nullable: Nullability;
}

impl<N: Nullability, Rest, Name, Table> FoundOr<N, Rest, Name, Table> for True {
    type Nullable = N;
}

impl<N, Rest, Name, Table> FoundOr<N, Rest, Name, Table> for False
where
    Rest: FindTable<Name, Table>,
{
    type Nullable = Rest::Nullable;
}

/// Marker wrapper that carries the sources of a SELECT.
///
/// - `Scope`: the FROM/JOIN sources, newest first.
/// - `Used`: a sources tree of every clause added so far (JOIN ON, WHERE,
///   GROUP BY, HAVING, ORDER BY), each wrapped in [`At`] with the scope it
///   was written against. It is checked when the query runs, or against the
///   enclosing query when this one is used as a subquery, so correlated
///   subqueries can read their outer query's sources.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scoped<Marker, Scope, Used = ()>(PhantomData<(Marker, Scope, Used)>);

/// Exposes the scope of a SELECT marker and records clause sources on it.
pub trait HasScope {
    /// The FROM/JOIN sources.
    type Scope;
    /// Sources of every clause added so far.
    type Used;
    /// This marker after a clause reading `Sources` against the current scope.
    type With<Sources>;
}

impl<Marker, Scope, Used> HasScope for Scoped<Marker, Scope, Used> {
    type Scope = Scope;
    type Used = Used;
    type With<Sources> = Scoped<Marker, Scope, (Used, At<Scope, Sources>)>;
}

// =============================================================================
// Sources tree
// =============================================================================

/// Sources-tree leaf: a column of the source `T` (a [`ScopeEntry`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct Src<T>(PhantomData<T>);

/// Sources-tree node that is NULL only when both sides are NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct Coalesce<A, B>(PhantomData<(A, B)>);

/// Sources that are scope-checked but never make the result NULL.
pub type ScopeOnly<S> = Coalesce<S, NonNull>;

/// Every source in a sources tree is in `Scope`.
///
/// `Nullable` is [`Null`] when an outer join can make the expression NULL
/// even though its declared nullability says otherwise.
pub trait SourcesIn<Scope, Proof> {
    type Nullable: Nullability;
}

impl<Scope> SourcesIn<Scope, ()> for () {
    type Nullable = NonNull;
}

impl<Scope> SourcesIn<Scope, ()> for NonNull {
    type Nullable = NonNull;
}

impl<Scope> SourcesIn<Scope, ()> for Null {
    type Nullable = Null;
}

impl<Scope, T, Witness> SourcesIn<Scope, Witness> for Src<T>
where
    T: ScopeEntry,
    Scope: ScopeContains<T::Key, Witness>,
{
    type Nullable = Scope::Nullable;
}

impl<Scope, A, B, ProofA, ProofB> SourcesIn<Scope, (ProofA, ProofB)> for (A, B)
where
    A: SourcesIn<Scope, ProofA>,
    B: SourcesIn<Scope, ProofB>,
    A::Nullable: NullOr<B::Nullable>,
{
    type Nullable = <A::Nullable as NullOr<B::Nullable>>::Output;
}

impl<Scope, A, B, ProofA, ProofB> SourcesIn<Scope, (ProofA, ProofB)> for Coalesce<A, B>
where
    A: SourcesIn<Scope, ProofA>,
    B: SourcesIn<Scope, ProofB>,
    A::Nullable: NullAnd<B::Nullable>,
{
    type Nullable = <A::Nullable as NullAnd<B::Nullable>>::Output;
}

/// Sources read by a clause of another query (or an earlier join step),
/// resolved against `Scope` first and then the enclosing scope.
///
/// Never NULL by itself: a subquery's inner sources do not make the outer
/// expression NULL (the subquery operator decides that).
#[derive(Debug, Clone, Copy, Default)]
pub struct At<Scope, S>(PhantomData<(Scope, S)>);

impl<Outer, Scope, S, Proof> SourcesIn<Outer, Proof> for At<Scope, S>
where
    Scope: crate::Concat<Outer>,
    S: SourcesIn<<Scope as crate::Concat<Outer>>::Output, Proof>,
{
    type Nullable = NonNull;
}

/// Sources a SELECT reads when it is used as a subquery expression.
///
/// A scoped query resolves its clauses and projection against its own
/// FROM/JOIN scope first ([`At`]); whatever does not resolve there must be a
/// source of the enclosing query (a correlated reference).
pub trait SelectSources {
    type Sources;
}

impl SelectSources for crate::row::SelectStar {
    type Sources = ();
}

impl SelectSources for crate::row::SelectExpr {
    type Sources = ();
}

impl<R> SelectSources for crate::row::SelectAs<R> {
    type Sources = ();
}

impl<Cols> SelectSources for crate::row::SelectCols<Cols>
where
    Cols: crate::row::SelectedExpressionList,
    Cols::Expressions: crate::expr::ExprSources,
{
    type Sources = <Cols::Expressions as crate::expr::ExprSources>::Sources;
}

impl<M: SelectSources, Scope, Used> SelectSources for Scoped<M, Scope, Used> {
    type Sources = (Used, At<Scope, M::Sources>);
}

/// The marker of a compound query (`UNION`, `INTERSECT`, `EXCEPT`) built
/// from a query with marker `Self` and an operand with marker `Other`.
///
/// The compound decodes like the left query; the operand's sources are kept
/// so the compound is scope-checked as a whole.
pub trait SetOperand<Other> {
    type Combined;
}

impl<M, Scope, Used, Other: SelectSources> SetOperand<Other> for Scoped<M, Scope, Used> {
    type Combined = Scoped<M, Scope, (Used, Other::Sources)>;
}

impl<Other> SetOperand<Other> for crate::row::SelectStar {
    type Combined = Self;
}

impl<Other> SetOperand<Other> for crate::row::SelectExpr {
    type Combined = Self;
}

impl<Cols, Other> SetOperand<Other> for crate::row::SelectCols<Cols> {
    type Combined = Self;
}

impl<R, Other> SetOperand<Other> for crate::row::SelectAs<R> {
    type Combined = Self;
}

/// Sources of a COALESCE-style operand: its declared nullability plus the
/// nullability its sources pick up from outer joins.
pub type Arg<N, S> = (N, S);

// =============================================================================
// Generic type lists
// =============================================================================

/// Exact type membership in a `Cons` list.
///
/// Used for column lists (GROUP BY keys, INSERT target columns), where the
/// elements are compared by type rather than by scope key.
pub trait ListContains<T, Witness> {}

impl<Head, Tail> ListContains<Head, ScopeHere> for Cons<Head, Tail> {}

impl<Head, Tail, T, Witness> ListContains<T, ScopeThere<Witness>> for Cons<Head, Tail> where
    Tail: ListContains<T, Witness>
{
}

/// Every element of `Required` is in `Self`.
pub trait ListIncludes<Required, Proof> {}

impl<List> ListIncludes<Nil, ()> for List {}

impl<List, Head, Tail, HeadProof, TailProof> ListIncludes<Cons<Head, Tail>, (HeadProof, TailProof)>
    for List
where
    List: ListContains<Head, HeadProof> + ListIncludes<Tail, TailProof>,
{
}

// =============================================================================
// Joins
// =============================================================================

mod join_kind_private {
    pub trait Sealed {}
}

/// A join kind marker: [`InnerJoin`], [`LeftJoin`], [`RightJoin`], [`FullJoin`].
pub trait JoinKind: join_kind_private::Sealed {}

/// `JOIN` / `INNER JOIN` / `CROSS JOIN`.
#[derive(Debug, Clone, Copy, Default)]
pub struct InnerJoin;
/// `LEFT [OUTER] JOIN`: the joined source can be NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct LeftJoin;
/// `RIGHT [OUTER] JOIN`: every source already in scope can be NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct RightJoin;
/// `FULL [OUTER] JOIN`: every source can be NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct FullJoin;

/// `[INNER|LEFT|CROSS] JOIN LATERAL`: the joined subquery may read the
/// sources joined before it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lateral<Kind>(PhantomData<Kind>);

impl join_kind_private::Sealed for InnerJoin {}
impl<Kind: JoinKind> join_kind_private::Sealed for Lateral<Kind> {}
impl<Kind: JoinKind> JoinKind for Lateral<Kind> {}
impl join_kind_private::Sealed for LeftJoin {}
impl join_kind_private::Sealed for RightJoin {}
impl join_kind_private::Sealed for FullJoin {}
impl JoinKind for InnerJoin {}
impl JoinKind for LeftJoin {}
impl JoinKind for RightJoin {}
impl JoinKind for FullJoin {}

/// Wraps every entry of a scope list in [`OuterJoined`].
#[doc(hidden)]
pub trait OuterJoinScope {
    type Out;
}

impl OuterJoinScope for Nil {
    type Out = Self;
}

impl<Head, Tail: OuterJoinScope> OuterJoinScope for Cons<Head, Tail> {
    type Out = Cons<OuterJoined<Head>, Tail::Out>;
}

/// How a select marker's row type changes when `Joined` is joined.
///
/// `SELECT *` grows the row by the joined model (wrapped in `Option` on the
/// nullable side); every other marker keeps its row.
#[doc(hidden)]
pub trait JoinRow<Row, Joined, Kind> {
    type Row;
}

/// The marker and row type after joining `Joined` with join kind `Kind`.
///
/// `On` is the sources tree of the join's `ON` condition. It is recorded
/// against the scope that includes `Joined`, which is exactly what the
/// condition may reference (plus the enclosing query, for a correlated
/// subquery). The joined source's own [`ScopeEntry::Sources`] resolve against
/// the enclosing query only, or against the new scope for a [`Lateral`] join.
pub trait JoinStep<Row, Joined, Kind, On = ()> {
    /// The new marker, with `Joined` pushed into its scope.
    type Marker;
    /// The new inferred row type.
    type Row;
}

/// Records a join's sources on a marker whose scope became `NewScope`.
type Joined<M, NewScope, Used, Free, On> = Scoped<M, NewScope, ((Used, Free), At<NewScope, On>)>;

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, InnerJoin, On> for Scoped<M, Scope, Used>
where
    M: JoinRow<Row, J, InnerJoin>,
    J: ScopeEntry,
{
    type Marker = Joined<M, Cons<J, Scope>, Used, J::Sources, On>;
    type Row = M::Row;
}

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, LeftJoin, On> for Scoped<M, Scope, Used>
where
    M: JoinRow<Row, J, LeftJoin>,
    J: ScopeEntry,
{
    type Marker = Joined<M, Cons<OuterJoined<J>, Scope>, Used, J::Sources, On>;
    type Row = M::Row;
}

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, RightJoin, On> for Scoped<M, Scope, Used>
where
    M: JoinRow<Row, J, RightJoin>,
    J: ScopeEntry,
    Scope: OuterJoinScope,
{
    type Marker = Joined<M, Cons<J, Scope::Out>, Used, J::Sources, On>;
    type Row = M::Row;
}

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, FullJoin, On> for Scoped<M, Scope, Used>
where
    M: JoinRow<Row, J, FullJoin>,
    J: ScopeEntry,
    Scope: OuterJoinScope,
{
    type Marker = Joined<M, Cons<OuterJoined<J>, Scope::Out>, Used, J::Sources, On>;
    type Row = M::Row;
}

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, Lateral<InnerJoin>, On>
    for Scoped<M, Scope, Used>
where
    M: JoinRow<Row, J, InnerJoin>,
    J: ScopeEntry,
{
    type Marker = Joined<M, Cons<J, Scope>, Used, (), (On, J::Sources)>;
    type Row = M::Row;
}

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, Lateral<LeftJoin>, On> for Scoped<M, Scope, Used>
where
    M: JoinRow<Row, J, LeftJoin>,
    J: ScopeEntry,
{
    type Marker = Joined<M, Cons<OuterJoined<J>, Scope>, Used, (), (On, J::Sources)>;
    type Row = M::Row;
}

/// The marker after `.from(source)`.
pub type FromMarker<M, Source> = Scoped<M, Cons<Source, Nil>, <Source as ScopeEntry>::Sources>;
