//! Shared typestate markers for dialect query builders.

use super::{ClauseAllowed, ExecutableState, clause};

//------------------------------------------------------------------------------
// SELECT states
//------------------------------------------------------------------------------

/// Marker for the initial state of `SelectBuilder`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectInitial;

/// Marker for the state after FROM clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectFromSet;

/// Marker for the state after JOIN clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectJoinSet;

/// Marker for the state after WHERE clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectWhereSet;

/// Marker for the state after GROUP BY clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectGroupSet;

/// Marker for the state after ORDER BY clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOrderSet;

/// Marker for the state after LIMIT clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectLimitSet;

/// Marker for the state after OFFSET clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOffsetSet;

/// Marker for the state after set operations (UNION/INTERSECT/EXCEPT).
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectSetOpSet;

impl ExecutableState for SelectFromSet {
    type Rows<R: crate::row::RowCount> = R;
}
impl ExecutableState for SelectWhereSet {
    type Rows<R: crate::row::RowCount> = R;
}
impl ExecutableState for SelectLimitSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for SelectOffsetSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
// A compound query reaches ORDER BY too, so it cannot keep an aggregate's
// one row.
impl ExecutableState for SelectOrderSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for SelectGroupSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for SelectJoinSet {
    type Rows<R: crate::row::RowCount> = R;
}
impl ExecutableState for SelectSetOpSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}

impl ClauseAllowed<clause::Where> for SelectFromSet {}
impl ClauseAllowed<clause::Where> for SelectJoinSet {}

impl ClauseAllowed<clause::GroupBy> for SelectFromSet {}
impl ClauseAllowed<clause::GroupBy> for SelectJoinSet {}
impl ClauseAllowed<clause::GroupBy> for SelectWhereSet {}

impl ClauseAllowed<clause::OrderBy> for SelectFromSet {}
impl ClauseAllowed<clause::OrderBy> for SelectJoinSet {}
impl ClauseAllowed<clause::OrderBy> for SelectWhereSet {}
impl ClauseAllowed<clause::OrderBy> for SelectGroupSet {}
// `SelectSetOpSet` is deliberately absent: a compound query orders by output
// column names, so each dialect builder provides its own `order_by` there.

impl ClauseAllowed<clause::Limit> for SelectFromSet {}
impl ClauseAllowed<clause::Limit> for SelectJoinSet {}
impl ClauseAllowed<clause::Limit> for SelectWhereSet {}
impl ClauseAllowed<clause::Limit> for SelectGroupSet {}
impl ClauseAllowed<clause::Limit> for SelectOrderSet {}
impl ClauseAllowed<clause::Limit> for SelectSetOpSet {}

impl ClauseAllowed<clause::Offset> for SelectFromSet {}
impl ClauseAllowed<clause::Offset> for SelectLimitSet {}
impl ClauseAllowed<clause::Offset> for SelectSetOpSet {}

impl ClauseAllowed<clause::Join> for SelectFromSet {}
impl ClauseAllowed<clause::Join> for SelectJoinSet {}

impl ClauseAllowed<clause::Having> for SelectGroupSet {}

impl ClauseAllowed<clause::Simple> for SelectFromSet {}
impl ClauseAllowed<clause::Simple> for SelectJoinSet {}
impl ClauseAllowed<clause::Simple> for SelectWhereSet {}
impl ClauseAllowed<clause::Simple> for SelectGroupSet {}
impl ClauseAllowed<clause::Simple> for SelectOrderSet {}
impl ClauseAllowed<clause::Simple> for SelectLimitSet {}
impl ClauseAllowed<clause::Simple> for SelectOffsetSet {}

impl ClauseAllowed<clause::Compound> for SelectFromSet {}
impl ClauseAllowed<clause::Compound> for SelectJoinSet {}
impl ClauseAllowed<clause::Compound> for SelectWhereSet {}
impl ClauseAllowed<clause::Compound> for SelectGroupSet {}
impl ClauseAllowed<clause::Compound> for SelectOrderSet {}
impl ClauseAllowed<clause::Compound> for SelectLimitSet {}
impl ClauseAllowed<clause::Compound> for SelectOffsetSet {}
impl ClauseAllowed<clause::Compound> for SelectSetOpSet {}

impl ClauseAllowed<clause::Source> for SelectFromSet {}
impl ClauseAllowed<clause::Source> for SelectJoinSet {}
impl ClauseAllowed<clause::Source> for SelectWhereSet {}
impl ClauseAllowed<clause::Source> for SelectGroupSet {}
impl ClauseAllowed<clause::Source> for SelectOrderSet {}
impl ClauseAllowed<clause::Source> for SelectLimitSet {}
impl ClauseAllowed<clause::Source> for SelectOffsetSet {}
impl ClauseAllowed<clause::Source> for SelectSetOpSet {}

//------------------------------------------------------------------------------
// INSERT states
//------------------------------------------------------------------------------

/// Marker for the initial state of `InsertBuilder`.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertInitial;

/// Marker for the state after VALUES are set.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertValuesSet;

/// Marker for the state after RETURNING clause is added.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertReturningSet;

/// Marker for the state after ON CONFLICT is set.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertOnConflictSet;

/// Marker for the state after DO UPDATE SET (before optional WHERE).
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertDoUpdateSet;

impl ExecutableState for InsertValuesSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for InsertReturningSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for InsertOnConflictSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for InsertDoUpdateSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}

//------------------------------------------------------------------------------
// DELETE states
//------------------------------------------------------------------------------

/// Marker for the initial state of `DeleteBuilder`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteInitial;

/// Marker for the state after WHERE clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteWhereSet;

/// Marker for the state after RETURNING clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteReturningSet;

impl ExecutableState for DeleteWhereSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for DeleteReturningSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}

//------------------------------------------------------------------------------
// UPDATE states
//------------------------------------------------------------------------------

/// Marker for the initial state of `UpdateBuilder`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateInitial;

/// Marker for the state after SET clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateSetClauseSet;

/// Marker for the state after WHERE clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateWhereSet;

/// Marker for the state after RETURNING clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateReturningSet;

impl ExecutableState for UpdateWhereSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
impl ExecutableState for UpdateReturningSet {
    type Rows<R: crate::row::RowCount> = crate::row::ZeroOrMore;
}
