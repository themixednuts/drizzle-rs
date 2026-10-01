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

impl ExecutableState for SelectFromSet {}
impl ExecutableState for SelectWhereSet {}
impl ExecutableState for SelectLimitSet {}
impl ExecutableState for SelectOffsetSet {}
impl ExecutableState for SelectOrderSet {}
impl ExecutableState for SelectGroupSet {}
impl ExecutableState for SelectJoinSet {}
impl ExecutableState for SelectSetOpSet {}

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

impl ExecutableState for InsertValuesSet {}
impl ExecutableState for InsertReturningSet {}
impl ExecutableState for InsertOnConflictSet {}
impl ExecutableState for InsertDoUpdateSet {}

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

impl ExecutableState for DeleteInitial {}
impl ExecutableState for DeleteWhereSet {}
impl ExecutableState for DeleteReturningSet {}

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

impl ExecutableState for UpdateSetClauseSet {}
impl ExecutableState for UpdateWhereSet {}
impl ExecutableState for UpdateReturningSet {}
