//! Row type inference and row decoding for SELECT queries.
//!
//! These traits work out the Rust type a query returns from what it selects,
//! its table, and its joins, so `.all()` and `.get()` need no turbofish. They
//! also decode database rows into that type.
//!
//! Most of this is internal machinery driven by the builders and the table
//! macros. Users meet it through compile errors at `.all()` / `.get()` /
//! `.rows()`, for example when a column from a `LEFT JOIN` is decoded as `T`
//! instead of `Option<T>`.
//!
//! # How a row type is built
//!
//! ```text
//! .select(cols)  -> marker  (SelectStar | SelectCols<C> | SelectExpr | SelectAs<R>)   via IntoSelectTarget
//! .from(table)   -> R       (marker + table -> row type)                              via ResolveRow
//! .join(t2)      -> R'      (marker + R + joined table -> new row type)               via JoinStep
//! .all()         -> Vec<R>  (scope, GROUP BY and decode checks, then FromDrizzleRow)
//! ```
//!
//! # Checks at the terminal method
//!
//! - [`MarkerScopeValidFor`]: every table the query reads was added with
//!   `.from(...)` or a join (see [`crate::scope`]).
//! - [`MarkerAggValidFor`]: with GROUP BY, every non-aggregate selected
//!   column is grouped.
//! - [`MarkerColumnCountValid`]: the decode target matches the selected
//!   columns, with `Option` wherever an outer join can produce NULL.
//! - [`StrictDecodeMarker`]: a raw `sql!` projection has an explicit type.

// Driver-specific leaf FromDrizzleRow implementations
#[cfg(feature = "libsql")]
mod libsql;
#[cfg(any(feature = "tokio-postgres", feature = "postgres-sync"))]
mod postgres;
#[cfg(feature = "rusqlite")]
mod rusqlite;
// Shared blanket impls for SQLite-flavored drivers whose cells are tagged
// unions (rusqlite, libsql, turso). The driver-specific files above just
// impl `SqliteValueRow` for their row type; everything else lives here.
#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
pub(crate) mod sqlite_value;
#[cfg(feature = "turso")]
mod turso;

use core::marker::PhantomData;

use crate::error::DrizzleError;
use crate::{Cons, Nil};

// =============================================================================
// Select Target Markers
// =============================================================================

/// Select marker for `SELECT *`: the row type is the table's select model and
/// grows with each join.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectStar;

/// Select marker for explicit columns: the row type is a tuple of the
/// columns' value types and does not change with joins.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectCols<Cols>(PhantomData<Cols>);

/// Select marker for raw SQL or an untyped expression: the caller must name
/// the row type.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectExpr;

/// Select marker for a user-chosen row model `R`, such as a `FromRow` struct
/// passed as `.select(MyRow::Select)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectAs<R>(PhantomData<R>);

pub use crate::scope::{
    HasScope, OuterJoined, ScopeContains, ScopeEntry, ScopeHere, ScopeThere, Scoped,
};

/// The query only reads tables it has added with `.from(...)` or a join.
///
/// `.all()`, `.get()` and `.rows()` require this bound. It holds when:
///
/// - every source read by a clause (JOIN ON, WHERE, GROUP BY, HAVING,
///   ORDER BY) is in the query's scope;
/// - every explicitly selected column comes from a source in scope;
/// - a `FromRow` selector's tables are in scope, and every field it reads
///   from the nullable side of an outer join is an `Option`.
///
/// A failure is reported through the trait that failed underneath, most
/// often "`X` is not in this query's FROM/JOIN scope" (see
/// [`crate::scope`]). `Proof` is a witness type the compiler infers.
///
/// # Compile-time checks
///
/// A `FromRow` model reading `Users` passes when `Users` is in scope:
///
/// ```
/// use drizzle_core::{Cons, Nil, Scoped, SelectAs, SelectTableFields, TableFields};
/// use drizzle_core::scope::{ScopeEntry, TableKey, name::{H1, H2}};
/// use drizzle_core::expr::NonNull;
/// use drizzle_core::row::MarkerScopeValidFor;
///
/// struct Users;
/// impl ScopeEntry for Users {
///     type Key = TableKey<Cons<H1, Nil>, Users>;
///     type Nullable = NonNull;
///     type Sources = ();
/// }
/// struct Model;
/// impl SelectTableFields for Model {
///     type TableFields = Cons<TableFields<Users, Cons<i32, Nil>>, Nil>;
/// }
///
/// fn needs_valid<M: MarkerScopeValidFor<P>, P>() {}
///
/// fn main() {
///     needs_valid::<Scoped<SelectAs<Model>, Cons<Users, Nil>>, _>();
/// }
/// ```
///
/// It fails when the query only selects from `Posts`:
///
/// ```compile_fail
/// use drizzle_core::{Cons, Nil, Scoped, SelectAs, SelectTableFields, TableFields};
/// use drizzle_core::scope::{ScopeEntry, TableKey, name::{H1, H2}};
/// use drizzle_core::expr::NonNull;
/// use drizzle_core::row::MarkerScopeValidFor;
///
/// struct Users;
/// struct Posts;
/// impl ScopeEntry for Users {
///     type Key = TableKey<Cons<H1, Nil>, Users>;
///     type Nullable = NonNull;
///     type Sources = ();
/// }
/// impl ScopeEntry for Posts {
///     type Key = TableKey<Cons<H2, Nil>, Posts>;
///     type Nullable = NonNull;
///     type Sources = ();
/// }
/// struct Model;
/// impl SelectTableFields for Model {
///     type TableFields = Cons<TableFields<Users, Cons<i32, Nil>>, Nil>;
/// }
///
/// fn needs_valid<M: MarkerScopeValidFor<P>, P>() {}
///
/// fn main() {
///     needs_valid::<Scoped<SelectAs<Model>, Cons<Posts, Nil>>, _>();
/// }
/// ```
pub trait MarkerScopeValidFor<Proof> {}

impl<Scope, Used, Proof> MarkerScopeValidFor<Proof> for Scoped<SelectStar, Scope, Used> where
    Used: SourcesIn<Nil, Proof>
{
}

impl<Scope, Used, Proof> MarkerScopeValidFor<Proof> for Scoped<SelectExpr, Scope, Used> where
    Used: SourcesIn<Nil, Proof>
{
}

impl<R, Scope, Used, UsedProof, FieldsProof> MarkerScopeValidFor<(UsedProof, FieldsProof)>
    for Scoped<SelectAs<R>, Scope, Used>
where
    Used: SourcesIn<Nil, UsedProof>,
    R: SelectTableFields,
    R::TableFields: TableFieldsIn<Scope, FieldsProof>,
{
}

impl<Cols, Scope, Used, UsedProof, ColsProof> MarkerScopeValidFor<(UsedProof, ColsProof)>
    for Scoped<SelectCols<Cols>, Scope, Used>
where
    Used: SourcesIn<Nil, UsedProof>,
    Cols: SelectedExpressionList,
    Cols::Expressions: ProjectionIn<Scope, ColsProof>,
{
}

use crate::scope::SourcesIn;

/// Marks a decoded column that can be NULL although its declared value type
/// is not an `Option`: a column from the nullable side of an outer join, or a
/// comparison whose operand can be NULL.
///
/// `MaybeNull<T>` only matches `Option<T>` in a decode target (and
/// `MaybeNull<Option<T>>` matches `Option<T>`), never a bare `T`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MaybeNull<T>(PhantomData<T>);

/// Checks an explicit SELECT list against the query scope.
///
/// Each expression's sources must be in `Scope`. The resulting column list
/// widens a column to [`MaybeNull`] when an outer join can make it NULL.
#[doc(hidden)]
pub trait ProjectionIn<Scope, Proof> {
    /// Column types the decode target is checked against.
    type Columns: crate::TypeSet;
}

impl<Scope> ProjectionIn<Scope, ()> for Nil {
    type Columns = Self;
}

impl<Head, Tail, Scope, HeadProof, TailProof> ProjectionIn<Scope, (HeadProof, TailProof)>
    for Cons<Head, Tail>
where
    Head: crate::expr::ExprSources + ExprValueType,
    Head::Sources: SourcesIn<Scope, HeadProof>,
    Tail: ProjectionIn<Scope, TailProof>,
{
    type Columns = Cons<
        <<Head::Sources as SourcesIn<Scope, HeadProof>>::Nullable as crate::expr::Nullability>::Decoded<
            Head::ValueType,
        >,
        Tail::Columns,
    >;
}

/// Field types of a `FromRow` selector, grouped by the source each reads.
///
/// Generated by `#[derive(SQLiteFromRow)]`, `#[derive(PostgresFromRow)]` and
/// `#[derive(MySQLFromRow)]`. [`MarkerScopeValidFor`] requires every listed
/// source to be in scope, and `Option` on every field read from the nullable
/// side of an outer join.
pub trait SelectTableFields {
    /// `Cons<TableFields<Source, Cons<Field, ...>>, ...>`.
    type TableFields;
}

/// Field types (a `Cons` list) that a `FromRow` selector reads from `Source`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TableFields<Source, Fields>(PhantomData<(Source, Fields)>);

/// The field list `Self` can hold values from a source with nullability
/// `Nullable`: any fields for [`NonNull`](crate::expr::NonNull), only
/// `Option` fields for [`Null`](crate::expr::Null).
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "a field read from the nullable side of an outer join must be an `Option`",
    label = "LEFT/RIGHT/FULL JOIN can return NULL for every column of this table",
    note = "change the field type to `Option<_>`, or use an inner join"
)]
pub trait FieldsAcceptNullability<Nullable> {}

impl<Fields> FieldsAcceptNullability<crate::expr::NonNull> for Fields {}

impl FieldsAcceptNullability<crate::expr::Null> for Nil {}

impl<T, Tail> FieldsAcceptNullability<crate::expr::Null> for Cons<Option<T>, Tail> where
    Tail: FieldsAcceptNullability<crate::expr::Null>
{
}

/// Checks a [`SelectTableFields`] list against the query scope.
#[doc(hidden)]
pub trait TableFieldsIn<Scope, Proof> {}

impl<Scope> TableFieldsIn<Scope, ()> for Nil {}

impl<Source, Fields, Tail, Scope, Witness, TailProof> TableFieldsIn<Scope, (Witness, TailProof)>
    for Cons<TableFields<Source, Fields>, Tail>
where
    Source: ScopeEntry,
    Scope: ScopeContains<Source::Key, Witness>,
    Fields: FieldsAcceptNullability<<Scope as ScopeContains<Source::Key, Witness>>::Nullable>,
    Tail: TableFieldsIn<Scope, TailProof>,
{
}

// =============================================================================
// Aggregate status validation for SELECT lists
// =============================================================================

/// Combined aggregate status of a tuple of expressions.
///
/// A 1-tuple has its element's status. Longer tuples fold the statuses with
/// [`CombineAggStatus`](crate::expr::CombineAggStatus).
pub trait AggStatus {
    /// The combined status.
    type Status;
}

// 1-tuple base case
impl<E: crate::expr::HasAggStatus> AggStatus for (E,) {
    type Status = E::Status;
}

// Generate AggStatus for 2..N tuples.
// The `with_col_sizes_*` macros call this incrementally as:
//   callback!(T0; 0)  callback!(T0, T1; 0, 1)  callback!(T0, T1, T2; 0, 1, 2)  ...
// We skip the 1-tuple (handled above) and implement 2+ tuples.
macro_rules! impl_tuple_agg_status {
    // 1-tuple: skip (already implemented above)
    ($E0:ident; $i0:tt) => {};
    // 2-tuple
    ($E0:ident, $E1:ident; $i0:tt, $i1:tt) => {
        impl<$E0, $E1> AggStatus for ($E0, $E1)
        where
            $E0: crate::expr::HasAggStatus,
            $E1: crate::expr::HasAggStatus,
            <$E0 as crate::expr::HasAggStatus>::Status:
                crate::expr::CombineAggStatus<<$E1 as crate::expr::HasAggStatus>::Status>,
        {
            type Status = <<$E0 as crate::expr::HasAggStatus>::Status as
                crate::expr::CombineAggStatus<<$E1 as crate::expr::HasAggStatus>::Status>>::Output;
        }
    };
    // 3+ tuples: fold head element's status with the rest-tuple's status
    ($E0:ident, $E1:ident, $($rest:ident),+; $i0:tt, $i1:tt, $($ri:tt),+) => {
        impl<$E0, $E1, $($rest),+> AggStatus for ($E0, $E1, $($rest),+)
        where
            $E0: crate::expr::HasAggStatus,
            ($E1, $($rest),+): AggStatus,
            <$E0 as crate::expr::HasAggStatus>::Status:
                crate::expr::CombineAggStatus<<($E1, $($rest),+) as AggStatus>::Status>,
        {
            type Status = <<$E0 as crate::expr::HasAggStatus>::Status as
                crate::expr::CombineAggStatus<<($E1, $($rest),+) as AggStatus>::Status>>::Output;
        }
    };
}

with_col_sizes_8!(impl_tuple_agg_status);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_tuple_agg_status);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_tuple_agg_status);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_tuple_agg_status);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_tuple_agg_status);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_tuple_agg_status);

// =============================================================================
// GROUP BY column tracking
// =============================================================================

/// Something that can be passed to `.group_by()`: a column or a tuple of
/// columns.
///
/// The table macros implement this for each column; tuples are implemented
/// here. `Columns` lists the grouped columns so the SELECT list can be
/// checked against them ([`MarkerAggValidFor`]).
pub trait IntoGroupBy<'a, V: crate::SQLParam + 'a>:
    crate::ToSQL<'a, V> + crate::expr::ExprSources
{
    /// Grouped columns as a type-level list (`Cons<Col1, Cons<Col2, Nil>>`),
    /// or [`PkGroup<T>`] when grouping by a table's single primary key.
    type Columns;
}

// Single column → Cons<Self, Nil>
// (Implemented by proc macros for each column ZST)

/// GROUP BY marker: the group key is `Table`'s single-column primary key.
///
/// The table macros produce this when `.group_by(col)` is called with a
/// table's only primary-key column. Every other column of that table depends
/// on its primary key (SQL:1999 functional dependency), so any column of
/// `Table` may be selected without being listed in GROUP BY. Columns of
/// other tables (for example joined ones) must still be aggregated.
///
/// Grouping by the bare primary key also lets the database read groups in
/// key order instead of sorting the whole result, which helps
/// `GROUP BY ... ORDER BY pk LIMIT n`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PkGroup<Table>(PhantomData<Table>);

// Tuple impls: (Col1, Col2) → Cons<Col1, Cons<Col2, Nil>>
macro_rules! impl_into_group_by_tuple {
    // 1-tuple: skip (single column uses direct impl)
    ($T0:ident; $i0:tt) => {};
    // 2-tuple
    ($T0:ident, $T1:ident; $i0:tt, $i1:tt) => {
        impl<'a, V: crate::SQLParam + 'a, $T0, $T1> IntoGroupBy<'a, V> for ($T0, $T1)
        where
            $T0: crate::ToSQL<'a, V> + crate::expr::ExprSources,
            $T1: crate::ToSQL<'a, V> + crate::expr::ExprSources,
        {
            type Columns = Cons<$T0, Cons<$T1, Nil>>;
        }
    };
    // 3+ tuples
    ($T0:ident, $T1:ident, $($rest:ident),+; $i0:tt, $i1:tt, $($ri:tt),+) => {
        impl<'a, V: crate::SQLParam + 'a, $T0, $T1, $($rest),+> IntoGroupBy<'a, V> for ($T0, $T1, $($rest),+)
        where
            $T0: crate::ToSQL<'a, V> + crate::expr::ExprSources,
            $T1: crate::ToSQL<'a, V> + crate::expr::ExprSources,
            $($rest: crate::ToSQL<'a, V> + crate::expr::ExprSources,)+
        {
            type Columns = impl_into_group_by_tuple!(@cons $T0, $T1, $($rest),+);
        }
    };
    // Helper: build nested Cons type
    (@cons $T:ident) => { Cons<$T, Nil> };
    (@cons $T:ident, $($rest:ident),+) => { Cons<$T, impl_into_group_by_tuple!(@cons $($rest),+)> };
}

with_col_sizes_8!(impl_into_group_by_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_into_group_by_tuple);

// =============================================================================
// Scalar column validation against grouped columns
// =============================================================================

/// Every non-aggregate column in a selected tuple is in the `Grouped` list.
/// Aggregate columns are skipped.
///
/// `Proof` is a witness type the compiler infers (like
/// [`ListContains`](crate::scope::ListContains)).
#[diagnostic::on_unimplemented(
    message = "non-aggregate column in SELECT is not in GROUP BY",
    label = "this column must appear in .group_by(...) or be wrapped in an aggregate function",
    note = "when using GROUP BY, every non-aggregate column in SELECT must be listed in GROUP BY, \
            unless the group key is the column's table primary key (`.group_by(table.pk)`)"
)]
pub trait ScalarColumnsIn<Grouped, Proof> {}

/// GROUP BY proof: the column is an aggregate, so it need not be grouped.
pub struct AggSkip;

/// GROUP BY proof: the scalar column is in the grouped list at the position
/// `W` proves.
pub struct ScalarCheck<W>(core::marker::PhantomData<W>);

/// GROUP BY proof: the scalar column is allowed because the group key is its
/// table's primary key (functional dependency).
pub struct PkDependent;

// 1-tuple
impl<E, Grouped, Proof> ScalarColumnsIn<Grouped, (Proof,)> for (E,) where
    E: SingleColGroupCheck<Grouped, Proof>
{
}

/// GROUP BY check for one selected column: aggregates pass, scalars must be
/// grouped.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is selected next to an aggregate but is not grouped",
    label = "this query mixes aggregated and ungrouped columns",
    note = "add the column to `.group_by(...)`, or wrap it in an aggregate such as `max(...)`"
)]
pub trait SingleColGroupCheck<Grouped, Proof> {}

/// The column an expression is matched as when checking GROUP BY.
///
/// An aliased column (`AliasedExpr<Col>`) matches as `Col`; a bare column or
/// any other expression matches as itself.
pub trait GroupByIdentity {
    /// The type looked up in the grouped column list.
    type Identity;
}

// Default: identity is self (bare column ZSTs)
// (implemented by proc macros for each column ZST)

// AliasedExpr unwraps to inner
impl<E: GroupByIdentity> GroupByIdentity for crate::expr::AliasedExpr<E> {
    type Identity = E::Identity;
}

// SQLExpr: identity is self (for aggregate expressions, this won't be checked anyway)
impl<V: crate::SQLParam, T, N, A, S> GroupByIdentity for crate::expr::SQLExpr<'_, V, T, N, A, S>
where
    T: crate::types::DataType,
    N: crate::expr::Nullability,
    A: crate::expr::AggregateKind,
{
    type Identity = Self;
}

// ColumnBinOp: identity is self (complex expressions won't match GROUP BY)
impl<Lhs, Rhs, Op, D, SQLType, Nullable> GroupByIdentity
    for crate::expr::ColumnBinOp<Lhs, Rhs, Op, D, SQLType, Nullable>
{
    type Identity = Self;
}

// ColumnNeg: identity is self
impl<T, D, SQLType, Nullable> GroupByIdentity for crate::expr::ColumnNeg<T, D, SQLType, Nullable> {
    type Identity = Self;
}

// Aggregate expressions → always OK
impl<E, Grouped> SingleColGroupCheck<Grouped, AggSkip> for E where
    E: crate::expr::HasAggStatus<Status = crate::expr::AllAgg>
{
}

// Scalar expressions → base column identity must be in Grouped list
impl<E, Grouped, W> SingleColGroupCheck<Grouped, ScalarCheck<W>> for E
where
    E: crate::expr::HasAggStatus<Status = crate::expr::AllScalar> + GroupByIdentity,
    Grouped: crate::scope::ListContains<E::Identity, W>,
{
}

// Scalar expressions under a primary-key group → the whole row of that table
// is functionally dependent on the group key, so any column of the grouped
// table passes. No overlap with the `ScalarCheck` impl above: `PkGroup<T>`
// never implements `ListContains`.
impl<E, T> SingleColGroupCheck<PkGroup<T>, PkDependent> for E
where
    E: crate::expr::HasAggStatus<Status = crate::expr::AllScalar> + GroupByIdentity,
    E::Identity: crate::traits::ColumnOf<T>,
{
}

// N-tuple: check head element, recurse on tail
// Uses (HeadProof, TailProof) witness structure, like ListIncludes.

// 2-tuple
impl<T0, T1, Grouped, P0, P1> ScalarColumnsIn<Grouped, (P0, P1)> for (T0, T1)
where
    T0: SingleColGroupCheck<Grouped, P0>,
    T1: SingleColGroupCheck<Grouped, P1>,
{
}

// 3+ tuples: check head, recurse on (T1, T2, ...)
macro_rules! impl_scalar_columns_in {
    // 1-tuple: skip (handled above directly)
    ($T0:ident; $i0:tt) => {};
    // 2-tuple: skip (handled above directly)
    ($T0:ident, $T1:ident; $i0:tt, $i1:tt) => {};
    // 3+ tuples: head + tail recursion
    ($T0:ident, $($rest:ident),+; $i0:tt, $($ri:tt),+) => {
        impl<$T0, $($rest),+, Grouped, HeadProof, TailProof>
            ScalarColumnsIn<Grouped, (HeadProof, TailProof)>
            for ($T0, $($rest),+)
        where
            $T0: SingleColGroupCheck<Grouped, HeadProof>,
            ($($rest,)+): ScalarColumnsIn<Grouped, TailProof>,
        {}
    };
}

with_col_sizes_8!(impl_scalar_columns_in);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_scalar_columns_in);

// =============================================================================
// MarkerAggValidFor — top-level bound on terminal methods
// =============================================================================

/// The SELECT list is valid for the query's GROUP BY.
///
/// `.all()`, `.get()` and `.rows()` require this bound.
///
/// - `Grouped = ()` (no GROUP BY): any mix of columns is accepted.
/// - `Grouped = Cons<...>`: each non-aggregate column of an explicit SELECT
///   list must be in the list.
/// - `Grouped = PkGroup<T>`: each non-aggregate column must belong to `T`.
///
/// `SELECT *`, raw SQL and `FromRow` selections are not checked.
#[diagnostic::on_unimplemented(
    message = "non-aggregate column in SELECT is not in GROUP BY",
    label = "add this column to .group_by(...) or wrap it in an aggregate function"
)]
pub trait MarkerAggValidFor<Grouped, Proof = ()> {}

// No GROUP BY (Grouped = ()) → always valid, any mix is fine
impl<Mk> MarkerAggValidFor<()> for Mk {}

// SelectStar with GROUP BY: can't check at compile time, always passes
impl<Scope, Used, Head, Tail> MarkerAggValidFor<Cons<Head, Tail>>
    for Scoped<SelectStar, Scope, Used>
{
}

// SelectExpr with GROUP BY: can't check, always passes
impl<Scope, Used, Head, Tail> MarkerAggValidFor<Cons<Head, Tail>>
    for Scoped<SelectExpr, Scope, Used>
{
}

// SelectAs with GROUP BY: user-specified type, always passes
impl<Scope, Used, R, Head, Tail> MarkerAggValidFor<Cons<Head, Tail>>
    for Scoped<SelectAs<R>, Scope, Used>
{
}

// SelectCols with GROUP BY: check each scalar column is in the Grouped list
impl<Scope, Used, Cols, Head, Tail, Proof> MarkerAggValidFor<Cons<Head, Tail>, Proof>
    for Scoped<SelectCols<Cols>, Scope, Used>
where
    Cols: ScalarColumnsIn<Cons<Head, Tail>, Proof>,
{
}

// GROUP BY a table's primary key (`Grouped = PkGroup<T>`): same shape as the
// Cons impls above, but scalar columns are checked for membership in the
// grouped table instead of the grouped column list.
impl<Scope, Used, T> MarkerAggValidFor<PkGroup<T>> for Scoped<SelectStar, Scope, Used> {}

impl<Scope, Used, T> MarkerAggValidFor<PkGroup<T>> for Scoped<SelectExpr, Scope, Used> {}

impl<Scope, Used, R, T> MarkerAggValidFor<PkGroup<T>> for Scoped<SelectAs<R>, Scope, Used> {}

impl<Scope, Used, Cols, T, Proof> MarkerAggValidFor<PkGroup<T>, Proof>
    for Scoped<SelectCols<Cols>, Scope, Used>
where
    Cols: ScalarColumnsIn<PkGroup<T>, Proof>,
{
}

// =============================================================================
// Marker column-count validation for strict decode paths
// =============================================================================

/// The columns a decode target reads, as a type-level list.
///
/// Each column is one `Cons<T, ...>` node, where `T` is the Rust type it
/// decodes into. Tuples concatenate their elements' lists.
pub trait RowColumnList<Row: ?Sized> {
    /// `Cons<T0, Cons<T1, ... Nil>>`.
    type Columns: crate::TypeSet;
}

/// The value types of a selected column tuple, as a type-level list.
pub trait SelectedColumnList {
    /// `Cons<T0, Cons<T1, ... Nil>>`.
    type Columns: crate::TypeSet;
}

/// Type-level expression list for an explicit SELECT projection.
///
/// Unlike [`SelectedColumnList`], this preserves each expression type so a
/// dialect can validate SQL types and nullability at a later boundary such as
/// `INSERT ... SELECT`.
#[doc(hidden)]
pub trait SelectedExpressionList {
    type Expressions: crate::TypeSet;
}

trait SameType<T> {}
impl<T> SameType<T> for T {}

#[diagnostic::on_unimplemented(
    message = "selected column decodes as `{Expected}`, but the decode target uses `{Actual}`",
    label = "the decode target does not match the selected columns",
    note = "a selected `MaybeNull<T>` can be NULL (an outer join or a nullable operand) \
            and must be decoded as `Option<T>`"
)]
trait ColumnTypeCompatible<Row: ?Sized, Expected, Actual> {}

impl<Row: ?Sized, T> ColumnTypeCompatible<Row, T, T> for () {}

// A column from the nullable side of an outer join decodes only into `Option`.
// The accepted inner types live on a separate trait so a decode error lists
// one candidate here instead of every widened variant.
impl<Row: ?Sized, Expected, Actual> ColumnTypeCompatible<Row, MaybeNull<Expected>, Option<Actual>>
    for ()
where
    (): MaybeNullCompatible<Row, Expected, Actual>,
{
}

/// Inner-type rule for [`MaybeNull`] columns: `Expected` is the column's
/// own decoded type, `Actual` the type inside the target's `Option`.
trait MaybeNullCompatible<Row: ?Sized, Expected, Actual> {}

impl<Row: ?Sized, T> MaybeNullCompatible<Row, T, T> for () {}

impl<Row: ?Sized, T> MaybeNullCompatible<Row, Option<T>, T> for () {}

trait TypeListCompatible<Row: ?Sized, ActualList> {}

impl<Row: ?Sized> TypeListCompatible<Row, Self> for crate::Nil {}

impl<Row: ?Sized, EH, ET, AH, AT> TypeListCompatible<Row, crate::Cons<AH, AT>>
    for crate::Cons<EH, ET>
where
    (): ColumnTypeCompatible<Row, EH, AH>,
    ET: TypeListCompatible<Row, AT>,
{
}

trait SqliteDecodeRow {}

#[cfg(feature = "rusqlite")]
impl SqliteDecodeRow for ::rusqlite::Row<'_> {}

#[cfg(feature = "libsql")]
impl SqliteDecodeRow for ::libsql::Row {}

#[cfg(feature = "turso")]
impl SqliteDecodeRow for ::turso::Row {}

macro_rules! impl_sqlite_integer_decode_compat {
    ($expected:ty => $($actual:ty),+ $(,)?) => {
        $(
            impl<Row> ColumnTypeCompatible<Row, $expected, $actual> for ()
            where
                Row: SqliteDecodeRow,
            {
            }
        )+
    };
}

impl_sqlite_integer_decode_compat!(
    i64 => i8, i16, i32, isize, u8, u16, u32, u64, usize, bool
);

macro_rules! impl_sqlite_join_nullable_integer_decode_compat {
    ($($actual:ty),+ $(,)?) => {
        $(
            impl<Row> MaybeNullCompatible<Row, i64, $actual> for ()
            where
                Row: SqliteDecodeRow,
            {
            }

            impl<Row> MaybeNullCompatible<Row, Option<i64>, $actual> for ()
            where
                Row: SqliteDecodeRow,
            {
            }
        )+
    };
}

impl_sqlite_join_nullable_integer_decode_compat!(
    i8, i16, i32, isize, u8, u16, u32, u64, usize, bool
);

impl_sqlite_integer_decode_compat!(
    Option<i64> =>
        Option<i8>,
        Option<i16>,
        Option<i32>,
        Option<isize>,
        Option<u8>,
        Option<u16>,
        Option<u32>,
        Option<u64>,
        Option<usize>,
        Option<bool>
);

macro_rules! impl_row_column_list_one {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<Row: ?Sized> RowColumnList<Row> for $ty {
                type Columns = crate::Cons<$ty, crate::Nil>;
            }
        )+
    };
}

impl_row_column_list_one!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    f32,
    f64,
    bool,
    crate::prelude::String,
    crate::prelude::Vec<u8>
);

impl<Row: ?Sized> RowColumnList<Row> for () {
    type Columns = crate::Cons<(), crate::Nil>;
}

#[cfg(feature = "uuid")]
impl<Row: ?Sized> RowColumnList<Row> for uuid::Uuid {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::NaiveDate {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::NaiveTime {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::NaiveDateTime {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::DateTime<chrono::Utc> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "serde")]
impl<Row: ?Sized> RowColumnList<Row> for serde_json::Value {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "rust-decimal")]
impl<Row: ?Sized> RowColumnList<Row> for rust_decimal::Decimal {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::Duration {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "time")]
impl<Row: ?Sized> RowColumnList<Row> for time::Date {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "time")]
impl<Row: ?Sized> RowColumnList<Row> for time::Time {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "time")]
impl<Row: ?Sized> RowColumnList<Row> for time::PrimitiveDateTime {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "time")]
impl<Row: ?Sized> RowColumnList<Row> for time::OffsetDateTime {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "time")]
impl<Row: ?Sized> RowColumnList<Row> for time::Duration {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "jiff")]
impl<Row: ?Sized> RowColumnList<Row> for jiff::civil::Date {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "jiff")]
impl<Row: ?Sized> RowColumnList<Row> for jiff::civil::Time {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "jiff")]
impl<Row: ?Sized> RowColumnList<Row> for jiff::civil::DateTime {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "jiff")]
impl<Row: ?Sized> RowColumnList<Row> for jiff::Timestamp {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "cidr")]
impl<Row: ?Sized> RowColumnList<Row> for cidr::IpInet {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "cidr")]
impl<Row: ?Sized> RowColumnList<Row> for cidr::IpCidr {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "geo-types")]
impl<Row: ?Sized> RowColumnList<Row> for geo_types::Point<f64> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "geo-types")]
impl<Row: ?Sized> RowColumnList<Row> for geo_types::LineString<f64> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "geo-types")]
impl<Row: ?Sized> RowColumnList<Row> for geo_types::Rect<f64> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "bit-vec")]
impl<Row: ?Sized> RowColumnList<Row> for bit_vec::BitVec {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "arrayvec")]
impl<Row: ?Sized, const N: usize> RowColumnList<Row> for arrayvec::ArrayString<N> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "arrayvec")]
impl<Row: ?Sized, T, const N: usize> RowColumnList<Row> for arrayvec::ArrayVec<T, N> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

impl<Row: ?Sized> RowColumnList<Row> for compact_str::CompactString {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "bytes")]
impl<Row: ?Sized> RowColumnList<Row> for bytes::Bytes {
    type Columns = crate::Cons<Self, crate::Nil>;
}

#[cfg(feature = "bytes")]
impl<Row: ?Sized> RowColumnList<Row> for bytes::BytesMut {
    type Columns = crate::Cons<Self, crate::Nil>;
}

impl<Row: ?Sized, A: smallvec::Array> RowColumnList<Row> for smallvec::SmallVec<A> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

impl<Row: ?Sized, T> RowColumnList<Row> for Option<T> {
    type Columns = crate::Cons<Self, crate::Nil>;
}

/// Helper: split last element from a type list and generate `RowColumnList` impl.
/// Called by `impl_rcl_tuple` after separating first from rest.
macro_rules! impl_rcl_body {
    // 1-tuple: just delegate
    ([$A:ident] []) => {
        impl<Row: ?Sized, $A: RowColumnList<Row>> RowColumnList<Row> for ($A,) {
            type Columns = <$A as RowColumnList<Row>>::Columns;
        }
    };
    // N-tuple: delegate to (N-1)-tuple, concat last
    ([$($all:ident),+] [$($prev:ident),+; $last:ident]) => {
        impl<Row: ?Sized, $($all),+> RowColumnList<Row> for ($($all,)+)
        where
            $last: RowColumnList<Row>,
            ($($prev,)+): RowColumnList<Row>,
            <($($prev,)+) as RowColumnList<Row>>::Columns:
                crate::Concat<<$last as RowColumnList<Row>>::Columns>,
        {
            type Columns = <<($($prev,)+) as RowColumnList<Row>>::Columns as crate::Concat<
                <$last as RowColumnList<Row>>::Columns,
            >>::Output;
        }
    };
}

/// Callback for `with_type_sizes_*!`: receives all types, separates last via
/// recursive accumulator, then delegates to `impl_rcl_body`.
macro_rules! impl_rcl_tuple {
    ($($T:ident),+) => {
        impl_rcl_split!([$($T),+] [] $($T),+);
    };
}

/// Recursive accumulator to split `[all] [prev...] remaining...`
macro_rules! impl_rcl_split {
    // 1-tuple: no prev, single element
    ([$A:ident] [] $only:ident) => {
        impl_rcl_body!([$A] []);
    };
    // Base: one element left in remaining = it's the last
    ([$($all:ident),+] [$($prev:ident),+] $last:ident) => {
        impl_rcl_body!([$($all),+] [$($prev),+; $last]);
    };
    // Recurse from empty prev
    ([$($all:ident),+] [] $head:ident, $($rest:ident),+) => {
        impl_rcl_split!([$($all),+] [$head] $($rest),+);
    };
    // Recurse with non-empty prev
    ([$($all:ident),+] [$($prev:ident),+] $head:ident, $($rest:ident),+) => {
        impl_rcl_split!([$($all),+] [$($prev),+, $head] $($rest),+);
    };
}

with_type_sizes_8!(impl_rcl_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_type_sizes_16!(impl_rcl_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_type_sizes_32!(impl_rcl_tuple);

/// The decode target `Actual` matches what the query selects.
///
/// `.all()`, `.get()` and `.rows()` require this bound. `Inferred` is the
/// query's inferred row type and `Actual` the type the caller decodes into.
///
/// - Explicit columns: each column of `Actual` must have the selected
///   column's type, and must be `Option` when an outer join or a nullable
///   operand can make it NULL.
/// - Joined `SELECT *`: `Actual` must keep the inferred shape (see
///   [`JoinedStarRow`]).
/// - Raw SQL: `Actual` must equal `Inferred`.
/// - Single-table `SELECT *` and `FromRow` selections are not checked here.
///
/// `Proof` is the same witness the terminal method infers for
/// [`MarkerScopeValidFor`], so outer-join nullability found while checking
/// scope also decides which decoded columns must be `Option`.
#[diagnostic::on_unimplemented(
    message = "selected shape does not match decode target `{Actual}`",
    label = "this decode target is not type-compatible with .select(...) output",
    note = "use typed expressions or derive FromRow for explicit remapping when selecting custom expressions"
)]
pub trait MarkerColumnCountValid<Row: ?Sized, Inferred, Actual, Proof = ()> {}

/// The select marker can be decoded by `.all()` / `.get()` without an
/// explicit row type.
///
/// A raw `select(sql!(...))` is excluded on purpose: give it a type with a
/// typed expression (`raw_non_null`, `raw_nullable`) or select into a
/// `FromRow` struct instead.
#[diagnostic::on_unimplemented(
    message = "raw select expressions require explicit typing in strict decode",
    label = "`select(sql!(...)).all()/get()` is not allowed in strict mode",
    note = "use typed wrappers like `raw_non_null`/`raw_nullable` or derive FromRow"
)]
pub trait StrictDecodeMarker {}

impl StrictDecodeMarker for SelectStar {}
impl<Cols> StrictDecodeMarker for SelectCols<Cols> {}
impl<R> StrictDecodeMarker for SelectAs<R> {}
impl<M, Scope, Used> StrictDecodeMarker for Scoped<M, Scope, Used> where M: StrictDecodeMarker {}

impl<Row: ?Sized, Inferred, Actual, Proof> MarkerColumnCountValid<Row, Inferred, Actual, Proof>
    for SelectStar
{
}

impl<Row: ?Sized, Cols, Inferred, Actual, Proof>
    MarkerColumnCountValid<Row, Inferred, Actual, Proof> for SelectCols<Cols>
where
    Cols: SelectedColumnList,
    Actual: RowColumnList<Row>,
    <Cols as SelectedColumnList>::Columns:
        TypeListCompatible<Row, <Actual as RowColumnList<Row>>::Columns>,
{
}

impl<Row: ?Sized, Inferred, Actual, Proof> MarkerColumnCountValid<Row, Inferred, Actual, Proof>
    for SelectExpr
where
    Inferred: SameType<Actual>,
{
}

impl<Row: ?Sized, R, Inferred, Actual, Proof> MarkerColumnCountValid<Row, Inferred, Actual, Proof>
    for SelectAs<R>
{
}

/// Single-source `SELECT *`: the table model may be decoded into any row type.
impl<Row: ?Sized, Table, Used, Inferred, Actual, Proof>
    MarkerColumnCountValid<Row, Inferred, Actual, Proof>
    for Scoped<SelectStar, Cons<Table, Nil>, Used>
{
}

/// Joined `SELECT *`: the decode target must keep the inferred row shape, with
/// `Option` on every source that an outer join can leave NULL.
impl<Row: ?Sized, First, Second, Rest, Used, Inferred, Actual, Proof>
    MarkerColumnCountValid<Row, Inferred, Actual, Proof>
    for Scoped<SelectStar, Cons<First, Cons<Second, Rest>>, Used>
where
    Inferred: JoinedStarRow<Actual>,
{
}

impl<Row: ?Sized, Cols, Scope, Used, Inferred, Actual, UsedProof, ColsProof>
    MarkerColumnCountValid<Row, Inferred, Actual, (UsedProof, ColsProof)>
    for Scoped<SelectCols<Cols>, Scope, Used>
where
    Cols: SelectedExpressionList,
    Cols::Expressions: ProjectionIn<Scope, ColsProof>,
    Actual: RowColumnList<Row>,
    <Cols::Expressions as ProjectionIn<Scope, ColsProof>>::Columns:
        TypeListCompatible<Row, <Actual as RowColumnList<Row>>::Columns>,
{
}

impl<Row: ?Sized, Scope, Used, Inferred, Actual, Proof>
    MarkerColumnCountValid<Row, Inferred, Actual, Proof> for Scoped<SelectExpr, Scope, Used>
where
    Inferred: SameType<Actual>,
{
}

/// `FromRow` selectors are checked by [`MarkerScopeValidFor`].
impl<Row: ?Sized, R, Scope, Used, Inferred, Actual, Proof>
    MarkerColumnCountValid<Row, Inferred, Actual, Proof> for Scoped<SelectAs<R>, Scope, Used>
{
}

/// Decode target for a joined `SELECT *` row.
///
/// Joins nest the inferred row as `(previous, joined)`. Each half must be
/// decoded as its inferred type; a half may additionally be widened to
/// `Option`, but a half the join already made `Option` cannot be narrowed.
#[diagnostic::on_unimplemented(
    message = "joined `SELECT *` rows decode as `{Self}`, not `{Actual}`",
    label = "the decode target does not match the joined row type",
    note = "LEFT/RIGHT/FULL JOIN sources can be NULL: decode them as `Option<_>`, \
            e.g. `(SelectUsers, Option<SelectPosts>)` after `.left_join(posts)`"
)]
pub trait JoinedStarRow<Actual> {}

impl<A, B, ActualA, ActualB> JoinedStarRow<(ActualA, ActualB)> for (A, B)
where
    A: JoinedStarPart<ActualA>,
    B: JoinedStarPart<ActualB>,
{
}

/// One half of a joined `SELECT *` row: exact, or widened to `Option`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "this half of the joined `SELECT *` row decodes as `{Self}`, not `{Actual}`",
    label = "the decode target does not match the joined row type",
    note = "LEFT/RIGHT/FULL JOIN sources can be NULL: decode them as `Option<_>`"
)]
pub trait JoinedStarPart<Actual> {}

impl<T> JoinedStarPart<T> for T {}

impl<T> JoinedStarPart<Option<T>> for T {}

/// Decodes one result row into `R` the way the select marker `Self` asks.
///
/// `SelectAs<R>` uses `R: TryFrom<RowRef>` (the `FromRow` derive). Every
/// other marker uses [`FromDrizzleRow`].
pub trait DecodeSelectedRef<RowRef, R> {
    /// Decodes `row` into `R`.
    ///
    /// # Errors
    ///
    /// Returns an error if the row cannot be decoded into the expected type
    /// (missing columns, type mismatch, or downstream conversion failure).
    fn decode(row: RowRef) -> Result<R, DrizzleError>;
}

impl<RowRef, R> DecodeSelectedRef<RowRef, R> for SelectAs<R>
where
    R: TryFrom<RowRef>,
    <R as TryFrom<RowRef>>::Error: Into<DrizzleError>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::try_from(row).map_err(Into::into)
    }
}

impl<RowRef, R, M, Scope, Used> DecodeSelectedRef<RowRef, R> for Scoped<M, Scope, Used>
where
    M: DecodeSelectedRef<RowRef, R>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        M::decode(row)
    }
}

impl<RowRef, Row: ?Sized, R> DecodeSelectedRef<RowRef, R> for SelectStar
where
    RowRef: core::ops::Deref<Target = Row>,
    R: FromDrizzleRow<Row>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::from_row(&*row)
    }
}

impl<RowRef, Row: ?Sized, Cols, R> DecodeSelectedRef<RowRef, R> for SelectCols<Cols>
where
    RowRef: core::ops::Deref<Target = Row>,
    R: FromDrizzleRow<Row>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::from_row(&*row)
    }
}

impl<RowRef, Row: ?Sized, R> DecodeSelectedRef<RowRef, R> for SelectExpr
where
    RowRef: core::ops::Deref<Target = Row>,
    R: FromDrizzleRow<Row>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::from_row(&*row)
    }
}

// =============================================================================
// FromDrizzleRow — offset-based row extraction
// =============================================================================

/// Reads a Rust value from a database row, starting at a column offset.
///
/// Unlike `TryFrom<Row>`, reading from an offset lets one joined row be split
/// across several model types. Tuples compose: `(A, B)` reads `A` at
/// `offset`, then `B` at `offset + A::COLUMN_COUNT`.
///
/// Implemented for scalar types per driver, for `Option<T>`, for tuples, and
/// by the `FromRow` derives and table macros for models.
#[diagnostic::on_unimplemented(
    message = "cannot deserialize `{Self}` from a database row",
    label = "this type does not implement FromDrizzleRow",
    note = "derive #[SQLiteFromRow], #[PostgresFromRow], or #[MySQLFromRow]"
)]
pub trait FromDrizzleRow<Row: ?Sized>: Sized {
    /// Number of columns this type reads from the row.
    const COLUMN_COUNT: usize;

    /// Reads this type from `row`, starting at column `offset`.
    ///
    /// # Errors
    ///
    /// Returns an error if any column from `offset` through
    /// `offset + COLUMN_COUNT - 1` cannot be read or converted.
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError>;

    /// Reads this type from `row`, starting at column 0.
    ///
    /// # Errors
    ///
    /// Returns an error if the row cannot be decoded — see [`Self::from_row_at`].
    fn from_row(row: &Row) -> Result<Self, DrizzleError> {
        Self::from_row_at(row, 0)
    }
}

/// A multi-column row type that can check whether it is absent (NULL).
///
/// This makes `Option<T>` work as a [`FromDrizzleRow`] target for a model,
/// as in a LEFT JOIN where the joined table may have no matching row.
///
/// The table macros implement this for each select model. Single-column
/// types (`i32`, `String`, ...) have their own `Option<T>` impls instead.
pub trait NullProbeRow<Row: ?Sized>: FromDrizzleRow<Row> {
    /// Returns `true` if the column at `offset` is NULL.
    ///
    /// # Errors
    ///
    /// Returns an error if the row cannot be inspected at `offset` (e.g. the
    /// driver reports an out-of-range index or conversion failure).
    fn is_null_at(row: &Row, offset: usize) -> Result<bool, DrizzleError>;
}

// -- Tuple impls: generic over Row, composing inner impls --

macro_rules! impl_from_drizzle_row_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<__Row: ?Sized, $($T: FromDrizzleRow<__Row>),+> FromDrizzleRow<__Row> for ($($T,)+) {
            const COLUMN_COUNT: usize = 0 $(+ <$T as FromDrizzleRow<__Row>>::COLUMN_COUNT)+;

            #[allow(non_snake_case)]
            fn from_row_at(
                row: &__Row,
                offset: usize,
            ) -> Result<Self, DrizzleError> {
                let mut __off = offset;
                $(
                    let $T = <$T as FromDrizzleRow<__Row>>::from_row_at(row, __off)?;
                    __off += <$T as FromDrizzleRow<__Row>>::COLUMN_COUNT;
                )+
                Ok(($($T,)+))
            }
        }
    };
}

with_col_sizes_8!(impl_from_drizzle_row_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_from_drizzle_row_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_from_drizzle_row_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_from_drizzle_row_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_from_drizzle_row_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_from_drizzle_row_tuple);

// =============================================================================
// SQLTypeToRust — SQL type marker × dialect → canonical Rust type
// =============================================================================

/// The default Rust type for a SQL type marker in dialect `D`.
///
/// `D` is a dialect marker ([`SQLiteDialect`], [`PostgresDialect`] or
/// [`MySQLDialect`]), so the mapping can differ per database. For example,
/// `postgres::types::Int4` maps to `i32` and `sqlite::types::Integer` to
/// `i64`. The selected column types of `.select(...)` come from here.
///
/// Some types map to a feature-gated Rust type (`chrono`, `time`, `jiff`,
/// `uuid`, `serde`, `cidr`, ...). Without any matching feature, some fall
/// back to `String`, while PostgreSQL date/time, `Uuid` and `Json`/`Jsonb`
/// have no mapping at all, which is a compile error naming the feature to
/// enable.
#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` has no default Rust mapping for dialect `{D}`",
    label = "this SQL type has no default Rust mapping for this dialect",
    note = "enable `chrono` for Date/Time/Timestamp/TimestampTz, `uuid` for Uuid, or `serde` for Json/Jsonb"
)]
pub trait SQLTypeToRust<D> {
    type RustType;
}

// -- Dialect-native mappings ---------------------------------------------------

use crate::dialect::{MySQLDialect, PostgresDialect, SQLiteDialect};

impl<D, T> SQLTypeToRust<D> for crate::types::Array<T>
where
    T: crate::types::DataType + SQLTypeToRust<D>,
{
    type RustType = crate::prelude::Vec<<T as SQLTypeToRust<D>>::RustType>;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Integer {
    type RustType = i64;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Text {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Real {
    type RustType = f64;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Blob {
    type RustType = crate::prelude::Vec<u8>;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Numeric {
    type RustType = f64;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Any {
    type RustType = crate::prelude::String;
}

macro_rules! impl_mysql_sql_type_to_rust {
    ($rust:ty => $($sql:ty),+ $(,)?) => {
        $(
            impl SQLTypeToRust<MySQLDialect> for $sql {
                type RustType = $rust;
            }
        )+
    };
}

impl_mysql_sql_type_to_rust!(i8 => drizzle_types::mysql::types::TinyInt);
impl_mysql_sql_type_to_rust!(u8 => drizzle_types::mysql::types::TinyIntUnsigned);
impl_mysql_sql_type_to_rust!(i16 => drizzle_types::mysql::types::SmallInt);
impl_mysql_sql_type_to_rust!(u16 => drizzle_types::mysql::types::SmallIntUnsigned);
impl_mysql_sql_type_to_rust!(i32 =>
    drizzle_types::mysql::types::MediumInt,
    drizzle_types::mysql::types::Int,
);
impl_mysql_sql_type_to_rust!(u32 =>
    drizzle_types::mysql::types::MediumIntUnsigned,
    drizzle_types::mysql::types::IntUnsigned,
);
impl_mysql_sql_type_to_rust!(i64 => drizzle_types::mysql::types::BigInt);
impl_mysql_sql_type_to_rust!(u64 => drizzle_types::mysql::types::BigIntUnsigned);
impl_mysql_sql_type_to_rust!(f32 => drizzle_types::mysql::types::Float);
impl_mysql_sql_type_to_rust!(f64 => drizzle_types::mysql::types::Double);
impl_mysql_sql_type_to_rust!(bool => drizzle_types::mysql::types::Boolean);
impl_mysql_sql_type_to_rust!(crate::prelude::String =>
    drizzle_types::mysql::types::Char,
    drizzle_types::mysql::types::Varchar,
    drizzle_types::mysql::types::TinyText,
    drizzle_types::mysql::types::Text,
    drizzle_types::mysql::types::MediumText,
    drizzle_types::mysql::types::LongText,
    drizzle_types::mysql::types::Enum,
    drizzle_types::mysql::types::Set,
    drizzle_types::mysql::types::Any,
);
impl_mysql_sql_type_to_rust!(crate::prelude::Vec<u8> =>
    drizzle_types::mysql::types::Binary,
    drizzle_types::mysql::types::Varbinary,
    drizzle_types::mysql::types::TinyBlob,
    drizzle_types::mysql::types::Blob,
    drizzle_types::mysql::types::MediumBlob,
    drizzle_types::mysql::types::LongBlob,
    drizzle_types::mysql::types::Bit,
);
impl_mysql_sql_type_to_rust!(u16 => drizzle_types::mysql::types::Year);

#[cfg(feature = "rust-decimal")]
impl_mysql_sql_type_to_rust!(rust_decimal::Decimal => drizzle_types::mysql::types::Decimal);
#[cfg(not(feature = "rust-decimal"))]
impl_mysql_sql_type_to_rust!(crate::prelude::String => drizzle_types::mysql::types::Decimal);

#[cfg(feature = "serde")]
impl_mysql_sql_type_to_rust!(serde_json::Value => drizzle_types::mysql::types::Json);
#[cfg(not(feature = "serde"))]
impl_mysql_sql_type_to_rust!(crate::prelude::String => drizzle_types::mysql::types::Json);

#[cfg(feature = "chrono")]
impl_mysql_sql_type_to_rust!(chrono::NaiveDate => drizzle_types::mysql::types::Date);
#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl_mysql_sql_type_to_rust!(time::Date => drizzle_types::mysql::types::Date);
#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl_mysql_sql_type_to_rust!(jiff::civil::Date => drizzle_types::mysql::types::Date);
#[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
impl_mysql_sql_type_to_rust!(crate::prelude::String => drizzle_types::mysql::types::Date);

// Unlike SQL TIME in SQLite/PostgreSQL, MySQL TIME is a signed duration that
// can exceed 24 hours (up to 838:59:59). Clock-only chrono/time values cannot
// represent its full domain, so the canonical selected value remains text,
// matching Drizzle ORM's MySQL TIME mapping.
impl_mysql_sql_type_to_rust!(crate::prelude::String => drizzle_types::mysql::types::Time);

#[cfg(feature = "chrono")]
impl_mysql_sql_type_to_rust!(chrono::NaiveDateTime => drizzle_types::mysql::types::DateTime);
// MySQL TIMESTAMP is session-time-zone aware. Wire adapters must establish a
// UTC session before executing typed queries, so the public value is an
// explicit UTC instant rather than an ambiguous naive datetime.
#[cfg(feature = "chrono")]
impl_mysql_sql_type_to_rust!(chrono::DateTime<chrono::Utc> => drizzle_types::mysql::types::Timestamp);
#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl_mysql_sql_type_to_rust!(time::PrimitiveDateTime => drizzle_types::mysql::types::DateTime);
#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl_mysql_sql_type_to_rust!(time::OffsetDateTime => drizzle_types::mysql::types::Timestamp);
#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl_mysql_sql_type_to_rust!(jiff::civil::DateTime => drizzle_types::mysql::types::DateTime);
#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl_mysql_sql_type_to_rust!(jiff::Timestamp => drizzle_types::mysql::types::Timestamp);
#[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
impl_mysql_sql_type_to_rust!(crate::prelude::String =>
    drizzle_types::mysql::types::DateTime,
    drizzle_types::mysql::types::Timestamp,
);

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Int2 {
    type RustType = i16;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Int4 {
    type RustType = i32;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Int8 {
    type RustType = i64;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Float4 {
    type RustType = f32;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Float8 {
    type RustType = f64;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Varchar {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Text {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Char {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Bytea {
    type RustType = crate::prelude::Vec<u8>;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Boolean {
    type RustType = bool;
}

#[cfg(feature = "rust-decimal")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Numeric {
    type RustType = rust_decimal::Decimal;
}

#[cfg(not(feature = "rust-decimal"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Numeric {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Any {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamptz {
    type RustType = chrono::DateTime<chrono::Utc>;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamp {
    type RustType = chrono::NaiveDateTime;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Date {
    type RustType = chrono::NaiveDate;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Time {
    type RustType = chrono::NaiveTime;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timetz {
    type RustType = chrono::NaiveTime;
}

#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamptz {
    type RustType = time::OffsetDateTime;
}

#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamp {
    type RustType = time::PrimitiveDateTime;
}

#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Date {
    type RustType = time::Date;
}

#[cfg(all(not(feature = "chrono"), feature = "time"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Time {
    type RustType = time::Time;
}

#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamptz {
    type RustType = jiff::Timestamp;
}

#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamp {
    type RustType = jiff::civil::DateTime;
}

#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Date {
    type RustType = jiff::civil::Date;
}

#[cfg(all(not(any(feature = "chrono", feature = "time")), feature = "jiff"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Time {
    type RustType = jiff::civil::Time;
}

#[cfg(feature = "uuid")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Uuid {
    type RustType = uuid::Uuid;
}

#[cfg(feature = "serde")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Json {
    type RustType = serde_json::Value;
}

#[cfg(feature = "serde")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Jsonb {
    type RustType = serde_json::Value;
}

// -- Feature-gated type marker mappings --

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Interval {
    type RustType = chrono::Duration;
}

#[cfg(not(feature = "chrono"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Interval {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "cidr")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Inet {
    type RustType = cidr::IpInet;
}

#[cfg(not(feature = "cidr"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Inet {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "cidr")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Cidr {
    type RustType = cidr::IpCidr;
}

#[cfg(not(feature = "cidr"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Cidr {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::MacAddr {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::MacAddr8 {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "geo-types")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Point {
    type RustType = geo_types::Point<f64>;
}

#[cfg(not(feature = "geo-types"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Point {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "geo-types")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::LineString {
    type RustType = geo_types::LineString<f64>;
}

#[cfg(not(feature = "geo-types"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::LineString {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "geo-types")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Rect {
    type RustType = geo_types::Rect<f64>;
}

#[cfg(not(feature = "geo-types"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Rect {
    type RustType = crate::prelude::String;
}

#[cfg(feature = "bit-vec")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::BitString {
    type RustType = bit_vec::BitVec;
}

#[cfg(not(feature = "bit-vec"))]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::BitString {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Line {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::LineSegment {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Polygon {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Circle {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Enum {
    type RustType = crate::prelude::String;
}

// =============================================================================
// WrapNullable — Option<T> wrapping based on nullability
// =============================================================================

/// `T` for [`NonNull`](crate::expr::NonNull), `Option<T>` for
/// [`Null`](crate::expr::Null).
pub trait WrapNullable<T> {
    /// The wrapped type.
    type Output;
}

impl<T> WrapNullable<T> for crate::expr::NonNull {
    type Output = T;
}

impl<T> WrapNullable<T> for crate::expr::Null {
    type Output = Option<T>;
}

// =============================================================================
// ExprValueType — "what Rust type does this expression produce?"
// =============================================================================

/// The Rust type a selected column or typed expression decodes into.
///
/// Implemented for:
/// - columns (generated by the table macros);
/// - `SQLExpr<V, T, N, A, S>`: [`SQLTypeToRust`] of `T`, wrapped in `Option`
///   when `N` is [`Null`](crate::expr::Null);
/// - raw `SQL<'a, V>`: `()`, so the caller must name the row type
///   (`.all::<T>()`).
#[diagnostic::on_unimplemented(
    message = "cannot infer Rust type for expression `{Self}`",
    label = "use typed expressions or derive FromRow to specify the Rust type",
    note = "raw SQL and JSON expressions require explicit type annotation"
)]
pub trait ExprValueType {
    /// The decoded Rust type.
    type ValueType;
}

impl<T: ExprValueType + ?Sized> ExprValueType for &T {
    type ValueType = T::ValueType;
}

impl<V: crate::SQLParam, T, N, A, S> ExprValueType for crate::expr::SQLExpr<'_, V, T, N, A, S>
where
    T: crate::types::DataType + SQLTypeToRust<V::DialectMarker>,
    N: crate::expr::Nullability + WrapNullable<<T as SQLTypeToRust<V::DialectMarker>>::RustType>,
    A: crate::expr::AggregateKind,
{
    type ValueType = <N as WrapNullable<<T as SQLTypeToRust<V::DialectMarker>>::RustType>>::Output;
}

/// Raw SQL has no known type (`()`); the caller must name the row type.
impl<V: crate::SQLParam> ExprValueType for crate::sql::SQL<'_, V> {
    type ValueType = ();
}

// =============================================================================
// HasSelectModel — table → Select model (lifetime-free)
// =============================================================================

/// The select model and column count of a table.
///
/// Generated by `#[SQLiteTable]`, `#[PostgresTable]` and `#[MySQLTable]`.
/// `SELECT *` from the table decodes into `SelectModel`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a drizzle table",
    label = "ensure this type was derived with #[SQLiteTable], #[PostgresTable], or #[MySQLTable]"
)]
pub trait HasSelectModel {
    /// The struct a `SELECT *` row decodes into.
    type SelectModel;
    /// Number of columns in the table.
    const COLUMN_COUNT: usize;
}

impl<T: HasSelectModel + ?Sized> HasSelectModel for &T {
    type SelectModel = T::SelectModel;

    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
}

// =============================================================================
// ResolveRow — Marker + Table → default row type R
// =============================================================================

/// The row type a select marker produces when selecting from `Table`.
///
/// Computed at `.from(table)`: the table's select model for `SELECT *`, a
/// tuple of value types for explicit columns, `R` for `SelectAs<R>`, and
/// `()` for raw SQL.
#[diagnostic::on_unimplemented(
    message = "cannot resolve return type for this query",
    label = "the selected columns and table do not produce a known row type"
)]
pub trait ResolveRow<Table> {
    /// The inferred row type.
    type Row;
}

impl<T: HasSelectModel> ResolveRow<T> for SelectStar {
    type Row = T::SelectModel;
}

impl<T> ResolveRow<T> for SelectExpr {
    type Row = ();
}

impl<R, T> ResolveRow<T> for SelectAs<R>
where
    R: SelectAsFrom<T>,
{
    type Row = R;
}

impl<M, Scope, Used, T> ResolveRow<T> for Scoped<M, Scope, Used>
where
    M: ResolveRow<T>,
{
    type Row = M::Row;
}

/// The `FromRow` struct `Self` may select from `Table`.
///
/// Checked by `.select(MyRow::Select).from(table)`. `#[from(Table)]` on a
/// `FromRow` struct implements this for that table only. A struct without
/// `#[from(...)]` accepts any table.
#[diagnostic::on_unimplemented(
    message = "row selector `{Self}` cannot be used with table `{Table}`",
    label = "the #[from(...)] table does not match .from(...)",
    note = "set #[from(TheTable)] to the same table passed to .from(...)"
)]
pub trait SelectAsFrom<Table> {}

// -- SelectCols: column value types → row tuple --

macro_rules! impl_resolve_row_cols {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<__Table, $($T: ExprValueType),+> ResolveRow<__Table> for SelectCols<($($T,)+)> {
            type Row = ($(<$T as ExprValueType>::ValueType,)+);
        }
    };
}

with_col_sizes_8!(impl_resolve_row_cols);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_resolve_row_cols);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_resolve_row_cols);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_resolve_row_cols);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_resolve_row_cols);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_resolve_row_cols);

macro_rules! selected_columns_cons {
    () => {
        crate::Nil
    };
    ($head:ident $(, $tail:ident)*) => {
        crate::Cons<<$head as ExprValueType>::ValueType, selected_columns_cons!($($tail),*)>
    };
}

macro_rules! selected_expressions_cons {
    () => {
        crate::Nil
    };
    ($head:ident $(, $tail:ident)*) => {
        crate::Cons<$head, selected_expressions_cons!($($tail),*)>
    };
}

macro_rules! impl_selected_column_list_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<$($T: ExprValueType),+> SelectedColumnList for ($($T,)+) {
            type Columns = selected_columns_cons!($($T),+);
        }
    };
}

macro_rules! impl_selected_expression_list_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<$($T),+> SelectedExpressionList for ($($T,)+) {
            type Expressions = selected_expressions_cons!($($T),+);
        }
    };
}

with_col_sizes_8!(impl_selected_column_list_tuple);
with_col_sizes_8!(impl_selected_expression_list_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_selected_column_list_tuple);
#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_selected_expression_list_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_selected_column_list_tuple);
#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_selected_expression_list_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_selected_column_list_tuple);
#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_selected_expression_list_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_selected_column_list_tuple);
#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_selected_expression_list_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_selected_column_list_tuple);
#[cfg(feature = "col200")]
with_col_sizes_200!(impl_selected_expression_list_tuple);

// =============================================================================
// JoinRow — how joins transform the row type
// =============================================================================

use crate::scope::{FullJoin, InnerJoin, JoinRow, LeftJoin, RightJoin};

// LATERAL joins grow `SELECT *` rows like their plain kind.
impl<R, T, Kind> JoinRow<R, T, crate::scope::Lateral<Kind>> for SelectStar
where
    SelectStar: JoinRow<R, T, Kind>,
{
    type Row = <SelectStar as JoinRow<R, T, Kind>>::Row;
}

/// Selections that stay valid after a `LEFT JOIN LATERAL`.
///
/// `SELECT *` decodes the joined source as `Option<JoinedTable::SelectModel>`.
/// Explicit columns and `FromRow` selections are accepted only when they read
/// sources already in scope before the lateral join: a column of the lateral
/// source would have to become nullable on unmatched rows.
#[doc(hidden)]
pub trait LeftLateralSelection<Proof = ()>: left_lateral_private::Sealed {}

mod left_lateral_private {
    pub trait Sealed {}

    impl Sealed for super::SelectStar {}
    impl<Scope, Used> Sealed for super::Scoped<super::SelectStar, Scope, Used> {}
    impl<Columns, Scope, Used> Sealed for super::Scoped<super::SelectCols<Columns>, Scope, Used> {}
    impl<Row, Scope, Used> Sealed for super::Scoped<super::SelectAs<Row>, Scope, Used> {}
}

impl LeftLateralSelection for SelectStar {}
impl<Scope, Used> LeftLateralSelection for Scoped<SelectStar, Scope, Used> {}

impl<Columns, Scope, Used, Proof> LeftLateralSelection<Proof>
    for Scoped<SelectCols<Columns>, Scope, Used>
where
    Columns: SelectedExpressionList,
    Columns::Expressions: ProjectionIn<Scope, Proof>,
{
}

impl<Row, Scope, Used, Proof> LeftLateralSelection<Proof> for Scoped<SelectAs<Row>, Scope, Used>
where
    Row: SelectTableFields,
    Row::TableFields: TableFieldsIn<Scope, Proof>,
{
}

/// `SELECT *` + JOIN → `(CurrentRow, JoinedTable::SelectModel)`.
impl<R, T: HasSelectModel> JoinRow<R, T, InnerJoin> for SelectStar {
    type Row = (R, T::SelectModel);
}

/// `SELECT *` + LEFT JOIN → `(CurrentRow, Option<JoinedTable::SelectModel>)`.
impl<R, T: HasSelectModel> JoinRow<R, T, LeftJoin> for SelectStar {
    type Row = (R, Option<T::SelectModel>);
}

/// `SELECT *` + RIGHT JOIN → `(Option<CurrentRow>, JoinedTable::SelectModel)`.
impl<R, T: HasSelectModel> JoinRow<R, T, RightJoin> for SelectStar {
    type Row = (Option<R>, T::SelectModel);
}

/// `SELECT *` + FULL JOIN → `(Option<CurrentRow>, Option<JoinedTable::SelectModel>)`.
impl<R, T: HasSelectModel> JoinRow<R, T, FullJoin> for SelectStar {
    type Row = (Option<R>, Option<T::SelectModel>);
}

/// Explicit columns + JOIN → R unchanged.
impl<Cols, R, T, Kind> JoinRow<R, T, Kind> for SelectCols<Cols> {
    type Row = R;
}

/// Raw/untyped + JOIN → R unchanged.
impl<R, T, Kind> JoinRow<R, T, Kind> for SelectExpr {
    type Row = R;
}

/// Explicit model + JOIN → R unchanged.
impl<Row, R, T, Kind> JoinRow<R, T, Kind> for SelectAs<Row> {
    type Row = R;
}

// =============================================================================
// IntoSelectTarget — select arguments → Marker type
// =============================================================================

/// Something that can be passed to `.select(...)`; picks the select marker.
///
/// The marker decides how the row type is inferred:
/// - [`SelectStar`]: the table's select model;
/// - [`SelectCols<C>`]: a tuple of the columns' value types;
/// - [`SelectExpr`]: the caller names the row type;
/// - [`SelectAs<R>`]: the `FromRow` struct `R`.
///
/// Implemented for:
/// - `()` -> `SelectStar`
/// - `SQL<'a, V>` -> `SelectExpr`
/// - `SQLExpr<'a, V, T, N, A, S>` -> `SelectCols<(Self,)>`
/// - tuples `(A, B, ...)` -> `SelectCols<(A, B, ...)>`
/// - columns and tables (generated by the table macros)
/// - `FromRow` selectors (generated by the `FromRow` derives) -> `SelectAs<R>`
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a select target",
    label = "this type does not implement IntoSelectTarget",
    note = "implement IntoSelectTarget or use a column, table, or typed expression"
)]
pub trait IntoSelectTarget {
    /// The select marker.
    type Marker;
}

impl<T: IntoSelectTarget + ?Sized> IntoSelectTarget for &T {
    type Marker = T::Marker;
}

/// `select(())` → `SelectStar` — infer row type from the table.
impl IntoSelectTarget for () {
    type Marker = SelectStar;
}

/// `select(sql!(...))` → `SelectExpr` — user must specify row type.
impl<V: crate::SQLParam> IntoSelectTarget for crate::sql::SQL<'_, V> {
    type Marker = SelectExpr;
}

/// `select(typed_expr)` → `SelectCols<(Expr,)>` — single typed expression.
impl<V: crate::SQLParam, T, N, A, S> IntoSelectTarget for crate::expr::SQLExpr<'_, V, T, N, A, S>
where
    T: crate::types::DataType,
    N: crate::expr::Nullability,
    A: crate::expr::AggregateKind,
{
    type Marker = SelectCols<(Self,)>;
}

/// Tuples of select targets → `SelectCols<(A, B, ...)>`.
macro_rules! impl_into_select_target_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<$($T),+> IntoSelectTarget for ($($T,)+) {
            type Marker = SelectCols<($($T,)+)>;
        }
    };
}

with_col_sizes_8!(impl_into_select_target_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_into_select_target_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_into_select_target_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_into_select_target_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_into_select_target_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_into_select_target_tuple);
