//! Shared typestate markers for dialect query builders.

use super::{
    ExecutableState, GroupByAllowed, GroupByApplied, HavingAllowed, JoinAllowed, LimitAllowed,
    OffsetAllowed, OrderByAllowed, WhereAllowed,
};

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

impl WhereAllowed for SelectFromSet {}
impl WhereAllowed for SelectJoinSet {}

impl GroupByAllowed for SelectFromSet {}
impl GroupByAllowed for SelectJoinSet {}
impl GroupByAllowed for SelectWhereSet {}

impl OrderByAllowed for SelectFromSet {}
impl OrderByAllowed for SelectJoinSet {}
impl OrderByAllowed for SelectWhereSet {}
impl OrderByAllowed for SelectGroupSet {}
impl OrderByAllowed for SelectSetOpSet {}

impl LimitAllowed for SelectFromSet {}
impl LimitAllowed for SelectJoinSet {}
impl LimitAllowed for SelectWhereSet {}
impl LimitAllowed for SelectGroupSet {}
impl LimitAllowed for SelectOrderSet {}
impl LimitAllowed for SelectSetOpSet {}

impl OffsetAllowed for SelectFromSet {}
impl OffsetAllowed for SelectLimitSet {}
impl OffsetAllowed for SelectSetOpSet {}

impl JoinAllowed for SelectFromSet {}
impl JoinAllowed for SelectJoinSet {}

impl HavingAllowed for SelectGroupSet {}

impl GroupByApplied for SelectGroupSet {}
impl GroupByApplied for SelectOrderSet {}
impl GroupByApplied for SelectLimitSet {}
impl GroupByApplied for SelectOffsetSet {}
impl GroupByApplied for SelectSetOpSet {}

#[doc(hidden)]
pub trait AsCteState {}

impl AsCteState for SelectFromSet {}
impl AsCteState for SelectJoinSet {}
impl AsCteState for SelectWhereSet {}
impl AsCteState for SelectGroupSet {}
impl AsCteState for SelectOrderSet {}
impl AsCteState for SelectLimitSet {}
impl AsCteState for SelectOffsetSet {}

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
