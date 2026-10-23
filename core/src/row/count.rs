//! How many rows a query returns, as far as its type knows.
//!
//! `.get()` returns the first row. Most queries can return none, so it
//! returns `Option<R>`. A query whose type proves it returns exactly one row
//! returns `R` instead: a SELECT of only aggregates (`count`, `sum`, `max`,
//! ...) with no GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET or set operation,
//! which returns one row even when no row matches its WHERE.

use super::{AggStatus, PkGroup, Scoped, SelectAs, SelectCols, SelectExpr, SelectStar};
use crate::error::DrizzleError;
use crate::expr::{AllAgg, AllScalar, MixedAgg};
use crate::{Cons, ExecutableState};

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::ExactlyOne {}
    impl Sealed for super::ZeroOrMore {}
}

/// How many rows a query returns, as far as its type knows: [`ExactlyOne`]
/// or [`ZeroOrMore`]. It decides what `.get()` returns.
pub trait RowCount: sealed::Sealed {
    /// What `.get()` returns for rows of type `R`.
    type Get<R>;

    /// The first row of a result as [`Self::Get`].
    ///
    /// # Errors
    ///
    /// [`ExactlyOne`] returns [`DrizzleError::NotFound`] when the database
    /// returned no row.
    fn first<R>(row: Option<R>) -> Result<Self::Get<R>, DrizzleError>;
}

/// The query always returns one row, so `.get()` returns it as `R`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExactlyOne;

impl RowCount for ExactlyOne {
    type Get<R> = R;

    fn first<R>(row: Option<R>) -> Result<R, DrizzleError> {
        row.ok_or(DrizzleError::NotFound)
    }
}

/// The query can return no row, so `.get()` returns `Option<R>`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZeroOrMore;

impl RowCount for ZeroOrMore {
    type Get<R> = Option<R>;

    fn first<R>(row: Option<R>) -> Result<Option<R>, DrizzleError> {
        Ok(row)
    }
}

/// The [`RowCount`] of a query that selects `Self`, grouped by `Grouped`, in
/// builder state `State`.
pub trait QueryRows<Grouped, State> {
    /// [`ExactlyOne`] or [`ZeroOrMore`].
    type Rows: RowCount;
}

impl<Mk, Grouped, State> QueryRows<Grouped, State> for Mk
where
    Mk: SelectionRows,
    Grouped: GroupingRows,
    State: ExecutableState,
{
    type Rows = State::Rows<Grouped::Rows<Mk::Rows>>;
}

/// What `.get()` returns for a query that selects `Mk`, grouped by
/// `Grouped`, in state `State`: `R` or `Option<R>`.
pub type GetRow<Mk, Grouped, State, R> =
    <<Mk as QueryRows<Grouped, State>>::Rows as RowCount>::Get<R>;

/// The [`RowCount`] a selection gives without GROUP BY: [`ExactlyOne`] for
/// a list of only aggregates, else [`ZeroOrMore`].
pub trait SelectionRows {
    /// [`ExactlyOne`] or [`ZeroOrMore`].
    type Rows: RowCount;
}

impl<M: SelectionRows, Scope, Used> SelectionRows for Scoped<M, Scope, Used> {
    type Rows = M::Rows;
}

impl SelectionRows for SelectStar {
    type Rows = ZeroOrMore;
}

impl SelectionRows for SelectExpr {
    type Rows = ZeroOrMore;
}

impl<R> SelectionRows for SelectAs<R> {
    type Rows = ZeroOrMore;
}

impl<Cols> SelectionRows for SelectCols<Cols>
where
    Cols: AggStatus,
    Cols::Status: StatusRows,
{
    type Rows = <Cols::Status as StatusRows>::Rows;
}

/// The [`RowCount`] after GROUP BY: unchanged without one (`()`), else
/// [`ZeroOrMore`].
pub trait GroupingRows {
    /// The row count of a selection with [`RowCount`] `R`.
    type Rows<R: RowCount>: RowCount;
}

impl GroupingRows for () {
    type Rows<R: RowCount> = R;
}

impl<Head, Tail> GroupingRows for Cons<Head, Tail> {
    type Rows<R: RowCount> = ZeroOrMore;
}

impl<Table> GroupingRows for PkGroup<Table> {
    type Rows<R: RowCount> = ZeroOrMore;
}

/// The [`RowCount`] a SELECT-list status gives without GROUP BY.
pub trait StatusRows {
    /// [`ExactlyOne`] or [`ZeroOrMore`].
    type Rows: RowCount;
}

impl StatusRows for AllAgg {
    type Rows = ExactlyOne;
}

impl StatusRows for AllScalar {
    type Rows = ZeroOrMore;
}

impl StatusRows for MixedAgg {
    type Rows = ZeroOrMore;
}

/// Raw SQL in a SELECT list counts as scalar: drizzle cannot see an
/// aggregate inside it, so the query may return no row.
impl<V: crate::SQLParam> crate::expr::HasAggStatus for crate::sql::SQL<'_, V> {
    type Status = AllScalar;
}

/// The first row of a query's result as `.get()` returns it: `R` when the
/// query selects `Mk` in a way that returns exactly one row, else
/// `Option<R>`.
///
/// # Errors
///
/// Returns [`DrizzleError::NotFound`] when the query should return exactly
/// one row but the database returned none.
#[doc(hidden)]
pub fn first_row<Mk, Grouped, State, R>(
    row: Option<R>,
) -> Result<GetRow<Mk, Grouped, State, R>, DrizzleError>
where
    Mk: QueryRows<Grouped, State>,
{
    <Mk::Rows as RowCount>::first(row)
}
