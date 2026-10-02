//! Compile-time checks that a query only reads tables it has joined.
//!
//! This module is internal machinery. Users never name these types; they see
//! the result as a compile error when a query reads a table it never added
//! with `.from(...)` or `.join(...)`:
//!
//! ```text
//! error[E0277]: `Posts` is not in this query's FROM/JOIN scope
//!    |
//! 47 |     db.select(users.id).from(users).r#where(eq(posts.views, 1)).all();
//!    |                                                                  ^^^ this expression reads a table that the query never joins
//! ```
//!
//! The error points at the terminal method (`.all()`, `.get()`, `.rows()`),
//! because that is where the whole query is checked.
//!
//! # How it works
//!
//! - **Scope.** A SELECT builder carries its FROM/JOIN sources in its marker
//!   type ([`Scoped`]) as a type-level list, newest first:
//!   `Cons<Posts, Cons<Users, Nil>>`. Each element is a [`ScopeEntry`].
//! - **Sources.** Every expression carries the sources it reads as a type
//!   (its [`Sources`](crate::expr::ExprSources::Sources)): `users.id` reads
//!   `Src<Users>`, and `eq(users.id, posts.author_id)` reads both.
//! - **Recording.** Each clause (JOIN ON, WHERE, GROUP BY, HAVING, ORDER BY)
//!   adds its expression's sources to the marker ([`HasScope::With`]), paired
//!   with the scope at that point ([`At`]). Joins update the scope through
//!   [`JoinStep`].
//! - **Checking.** The terminal method requires
//!   [`MarkerScopeValidFor`](crate::row::MarkerScopeValidFor), which asks
//!   [`SourcesIn`]: is every recorded source in the scope?
//!
//! The same walk also works out nullability. A source on the nullable side
//! of an outer join behaves like a table whose columns are all nullable, so
//! after `.left_join(posts)` a selected `posts.title` must be decoded as
//! `Option<String>`.
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
//! | [`At<Scope, S>`] | sources of another query or an earlier clause | never |
//!
//! Operators whose result is never NULL (`IS NULL`, `COUNT`, `EXISTS`) wrap
//! their operands in [`ScopeOnly`], so the operands are still scope-checked
//! but cannot make the result NULL.
//!
//! # Examples
//!
//! A source in scope passes the check:
//!
//! ```
//! use drizzle_core::{Cons, Nil};
//! use drizzle_core::expr::NonNull;
//! use drizzle_core::scope::{ScopeEntry, SourcesIn, Src, TableKey, name::{H1, H2}};
//!
//! struct Users;
//! impl ScopeEntry for Users {
//!     type Key = TableKey<Cons<H1, Nil>, Users>;
//!     type Nullable = NonNull;
//!     type Sources = ();
//! }
//!
//! fn in_scope<S: SourcesIn<Scope, P>, Scope, P>() {}
//!
//! in_scope::<Src<Users>, Cons<Users, Nil>, _>();
//! ```
//!
//! A source that was never joined does not:
//!
//! ```compile_fail
//! use drizzle_core::{Cons, Nil};
//! use drizzle_core::expr::NonNull;
//! use drizzle_core::scope::{ScopeEntry, SourcesIn, Src, TableKey, name::{H1, H2}};
//!
//! struct Users;
//! struct Posts;
//! impl ScopeEntry for Users {
//!     type Key = TableKey<Cons<H1, Nil>, Users>;
//!     type Nullable = NonNull;
//!     type Sources = ();
//! }
//! impl ScopeEntry for Posts {
//!     type Key = TableKey<Cons<H2, Nil>, Posts>;
//!     type Nullable = NonNull;
//!     type Sources = ();
//! }
//!
//! fn in_scope<S: SourcesIn<Scope, P>, Scope, P>() {}
//!
//! // error: `Posts` is not in this query's FROM/JOIN scope
//! in_scope::<Src<Posts>, Cons<Users, Nil>, _>();
//! ```

use core::marker::PhantomData;

use crate::expr::{NonNull, Null, Nullability};
use crate::{Cons, Nil};

// Bound-free `Clone`/`Copy`/`Default`/`Debug` for type-level markers, so user
// tag and table types need not implement them.
macro_rules! marker_impls {
    ($($name:ident<$($p:ident),+>),+ $(,)?) => {$(
        impl<$($p),+> Clone for $name<$($p),+> {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<$($p),+> Copy for $name<$($p),+> {}
        impl<$($p),+> Default for $name<$($p),+> {
            fn default() -> Self {
                Self(PhantomData)
            }
        }
        impl<$($p),+> core::fmt::Debug for $name<$($p),+> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(stringify!($name))
            }
        }
    )+};
}

marker_impls!(
    AliasKey<Tag>,
    OuterJoined<T>,
    ScopeThere<Prev>,
    TableKey<Name, Table>,
    Scoped<Marker, Scope, Used>,
    Src<T>,
    Coalesce<A, B>,
    At<Scope, S>,
    Lateral<Kind>,
);

// =============================================================================
// Scope entries
// =============================================================================

/// Something that can be listed in FROM or JOIN: a table, view, alias,
/// derived table, CTE, or raw SQL.
///
/// Table, view, and alias macros implement this for you. A column's
/// [`Sources`](crate::expr::ExprSources::Sources) names its source as
/// [`Src<T>`], and the scope check looks up `T::Key` in the query's scope.
///
/// Tables and views are keyed by their SQL name ([`TableKey`]). Aliased
/// sources (`Table::alias::<Tag>()`, derived tables, CTEs) are keyed by
/// [`AliasKey<Tag>`], because SQL resolves their columns by alias name.
pub trait ScopeEntry {
    /// How columns refer to this source: a [`TableKey`] or an [`AliasKey`].
    type Key;
    /// [`Null`] when an outer join can leave this source NULL.
    type Nullable: Nullability;
    /// Sources this source reads itself, such as the tables a derived
    /// table's subquery reads from an outer query. They resolve against the
    /// enclosing query (or, for a `LATERAL` join, the sources joined before
    /// it). Tables and views read nothing (`()`).
    type Sources;
}

impl<T: ScopeEntry + ?Sized> ScopeEntry for &T {
    type Key = T::Key;
    type Nullable = T::Nullable;
    type Sources = T::Sources;
}

/// Key of a raw SQL source (`.from(sql)`). No typed column can refer to it.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawSource;

impl<V: crate::SQLParam> ScopeEntry for crate::SQL<'_, V> {
    type Key = RawSource;
    type Nullable = NonNull;
    type Sources = ();
}

/// Key of a source that SQL refers to by an alias name. `Tag` is the alias's
/// type-level name.
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
pub struct OuterJoined<T>(PhantomData<T>);

impl<T: ScopeEntry> ScopeEntry for OuterJoined<T> {
    type Key = T::Key;
    type Nullable = Null;
    // Recorded when the source was joined.
    type Sources = ();
}

/// Proof that an item is the first element of a type-level list.
///
/// The compiler infers these proof types; users never write them.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeHere;

/// Proof that an item is further down a type-level list, at the position
/// `Prev` proves for the tail.
pub struct ScopeThere<Prev>(PhantomData<Prev>);

/// Proof that a table or view was found in a scope by name comparison.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeFound;

/// The scope `Self` contains a source whose key is `Key`.
///
/// Tables and views are looked up by SQL name, so the first (innermost)
/// matching source wins: a correlated subquery that reads the same table as
/// its outer query resolves to its own copy, as in SQL. Aliased sources are
/// looked up by alias type.
///
/// When this fails, the compiler reports "`X` is not in this query's
/// FROM/JOIN scope". `Witness` is a proof type the compiler infers.
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
    Scope: FindTable<Table>,
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

/// Key of a table or view: its SQL name spelled as a type-level list of
/// nibbles (see [`name`]), plus the Rust type for error messages.
///
/// Rust's trait system cannot tell that two different types are *not*
/// equal, but it can compare two nibble lists digit by digit. Comparing by
/// name is what lets the lookup skip non-matching tables.
pub struct TableKey<Name, Table>(PhantomData<(Name, Table)>);

/// Type-level boolean `true`.
#[derive(Debug, Clone, Copy, Default)]
pub struct True;
/// Type-level boolean `false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct False;

/// Type-level hex digits that spell a table's SQL name, two per byte (high
/// nibble first). `"posts"` is `Cons<H7, Cons<H0, Cons<H6, Cons<HF, ...>>>>`.
///
/// The table macros generate these lists; users never write them.
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

    nibbles!(
        H0, H1, H2, H3, H4, H5, H6, H7, H8, H9, HA, HB, HC, HD, HE, HF
    );

    /// Type-level equality of two nibbles: `Out` is [`True`] or [`False`].
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

    nib_eq!(
        H0, H1, H2, H3, H4, H5, H6, H7, H8, H9, HA, HB, HC, HD, HE, HF
    );
}

/// Type-level equality of two nibble lists: `Out` is [`True`] or [`False`].
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

/// The SQL name of a table key.
#[doc(hidden)]
pub trait TableName {
    type Name;
}

impl<Name, Table> TableName for TableKey<Name, Table> {
    type Name = Name;
}

/// Whether a scope entry's key names the same table as `Table`.
#[doc(hidden)]
pub trait IsTable<Table> {
    type Out;
}

impl<Table, Other, T> IsTable<Table> for TableKey<Other, T>
where
    Table: ScopeEntry,
    Table::Key: TableName,
    Other: NameEq<<Table::Key as TableName>::Name>,
{
    type Out = Other::Out;
}

impl<Table, Tag> IsTable<Table> for AliasKey<Tag> {
    type Out = False;
}

impl<Table> IsTable<Table> for RawSource {
    type Out = False;
}

/// Finds the first source in a scope list that is the table `Table`,
/// compared by SQL name.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Table}` is not in this query's FROM/JOIN scope",
    label = "this expression reads a table that the query never joins",
    note = "add the table with .from(...) or a .join(...) before using its columns"
)]
pub trait FindTable<Table> {
    type Nullable: Nullability;
}

impl<Table, Head, Tail> FindTable<Table> for Cons<Head, Tail>
where
    Head: ScopeEntry,
    Head::Key: IsTable<Table>,
    <Head::Key as IsTable<Table>>::Out: FoundOr<Head::Nullable, Tail, Table>,
{
    type Nullable =
        <<Head::Key as IsTable<Table>>::Out as FoundOr<Head::Nullable, Tail, Table>>::Nullable;
}

/// Continues a [`FindTable`] search: [`True`] stops at the head, [`False`]
/// searches the tail.
#[doc(hidden)]
pub trait FoundOr<Nullable, Rest, Table> {
    type Nullable: Nullability;
}

impl<N: Nullability, Rest, Table> FoundOr<N, Rest, Table> for True {
    type Nullable = N;
}

impl<N, Rest, Table> FoundOr<N, Rest, Table> for False
where
    Rest: FindTable<Table>,
{
    type Nullable = Rest::Nullable;
}

/// SELECT marker that also carries the query's scope and the sources its
/// clauses read.
///
/// `.from(...)` wraps the select marker (`SelectStar`, `SelectCols`, ...) in
/// this type, and each join and clause updates it.
///
/// - `Marker`: how rows decode (see [`crate::row`]).
/// - `Scope`: the FROM/JOIN sources, newest first.
/// - `Used`: a sources tree of every clause added so far (JOIN ON, WHERE,
///   GROUP BY, HAVING, ORDER BY), each wrapped in [`At`] with the scope it
///   was written against.
///
/// `Used` is checked by `.all()`, `.get()` and `.rows()` (through
/// [`MarkerScopeValidFor`](crate::row::MarkerScopeValidFor)). When the query
/// is used as a subquery, `Used` is checked against the outer query instead,
/// so a correlated subquery can read its outer query's tables.
pub struct Scoped<Marker, Scope, Used = ()>(PhantomData<(Marker, Scope, Used)>);

/// Reads the scope of a SELECT marker and records clause sources on it.
///
/// Builder methods such as `.r#where(expr)` use `M::With<E::Sources>` as the
/// new marker type, so the clause is checked later with the rest of the query.
pub trait HasScope {
    /// The FROM/JOIN sources.
    type Scope;
    /// Sources of every clause added so far.
    type Used;
    /// This marker after adding a clause that reads `Sources`, recorded
    /// against the current scope.
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

/// Sources-tree leaf: the expression reads a column of the source `T` (a
/// [`ScopeEntry`]).
pub struct Src<T>(PhantomData<T>);

/// Sources-tree node that is NULL only when both sides are NULL, as in
/// `COALESCE(a, b)`.
pub struct Coalesce<A, B>(PhantomData<(A, B)>);

/// Sources that are scope-checked but never make the result NULL (used by
/// `IS NULL`, `COUNT`, `EXISTS`).
pub type ScopeOnly<S> = Coalesce<S, NonNull>;

/// Every source in the sources tree `Self` is in `Scope`.
///
/// This is the core scope check. It fails, with "`X` is not in this query's
/// FROM/JOIN scope", when a [`Src<T>`] in the tree names a source that
/// `Scope` does not contain. `Proof` is a witness type the compiler infers.
pub trait SourcesIn<Scope, Proof> {
    /// [`Null`] when an outer join can make the expression NULL even though
    /// its declared nullability says otherwise.
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
{
    type Nullable = <A::Nullable as Nullability>::Or<B::Nullable>;
}

impl<Scope, A, B, ProofA, ProofB> SourcesIn<Scope, (ProofA, ProofB)> for Coalesce<A, B>
where
    A: SourcesIn<Scope, ProofA>,
    B: SourcesIn<Scope, ProofB>,
{
    type Nullable = <A::Nullable as Nullability>::And<B::Nullable>;
}

/// Sources `S` read by a clause written against `Scope`.
///
/// They resolve against `Scope` first and then against the enclosing scope.
/// This is how a subquery's clauses can read the outer query's tables.
///
/// Never NULL by itself: a subquery's inner sources do not make the outer
/// expression NULL (the subquery operator decides that).
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
    /// The sources tree the outer query must contain.
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
/// The compound decodes like the left query. The operand's sources are kept
/// so the compound is scope-checked as a whole.
pub trait SetOperand<Other> {
    /// The marker of the compound query.
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

/// Sources of a COALESCE-style operand: its declared nullability `N` plus
/// its sources `S`, which may add nullability from outer joins.
pub type Arg<N, S> = (N, S);

// =============================================================================
// Generic type lists
// =============================================================================

/// The `Cons` list `Self` contains the type `T`.
///
/// Used for column lists (GROUP BY keys, INSERT target columns), where the
/// elements are compared by type rather than by scope key. `Witness` is a
/// proof type the compiler infers ([`ScopeHere`] / [`ScopeThere`]).
pub trait ListContains<T, Witness> {}

impl<Head, Tail> ListContains<Head, ScopeHere> for Cons<Head, Tail> {}

impl<Head, Tail, T, Witness> ListContains<T, ScopeThere<Witness>> for Cons<Head, Tail> where
    Tail: ListContains<T, Witness>
{
}

/// Every element of the `Cons` list `Required` is in the list `Self`.
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

/// Join kind for [`JoinStep`]: `JOIN`, `INNER JOIN` or `CROSS JOIN`. No
/// source becomes nullable.
#[derive(Debug, Clone, Copy, Default)]
pub struct InnerJoin;
/// Join kind for [`JoinStep`]: `LEFT [OUTER] JOIN`. The joined source can
/// be NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct LeftJoin;
/// Join kind for [`JoinStep`]: `RIGHT [OUTER] JOIN`. Every source already
/// in scope can be NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct RightJoin;
/// Join kind for [`JoinStep`]: `FULL [OUTER] JOIN`. Every source can be
/// NULL.
#[derive(Debug, Clone, Copy, Default)]
pub struct FullJoin;

/// Join kind for [`JoinStep`]: `[INNER|LEFT|CROSS] JOIN LATERAL` (`Kind` is
/// [`InnerJoin`] or [`LeftJoin`]). The joined subquery may read the sources
/// joined before it.
pub struct Lateral<Kind>(PhantomData<Kind>);

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
/// nullable side). Every other marker keeps its row.
#[doc(hidden)]
pub trait JoinRow<Row, Joined, Kind> {
    type Row;
}

/// The marker and row type after joining `Joined` with join kind `Kind`
/// ([`InnerJoin`], [`LeftJoin`], [`RightJoin`], [`FullJoin`] or
/// [`Lateral`]).
///
/// Join builder methods use this to compute their return type. It pushes
/// `Joined` onto the scope, wrapping the sources an outer join can leave
/// NULL in [`OuterJoined`].
///
/// `On` is the sources tree of the join's `ON` condition. It is recorded
/// against the scope that includes `Joined`, which is exactly what the
/// condition may reference (plus the enclosing query, for a correlated
/// subquery). The joined source's own [`ScopeEntry::Sources`] resolve against
/// the enclosing query only, or against the new scope for a [`Lateral`] join.
/// Nothing is checked here; the check happens at the terminal method.
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

impl<M, Scope, Used, Row, J, On> JoinStep<Row, J, Lateral<InnerJoin>, On> for Scoped<M, Scope, Used>
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

/// The marker after `.from(source)`: `M` wrapped in [`Scoped`] with `Source`
/// as the only scope entry.
pub type FromMarker<M, Source> = Scoped<M, Cons<Source, Nil>, <Source as ScopeEntry>::Sources>;
