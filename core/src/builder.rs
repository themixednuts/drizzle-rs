use crate::{traits::SQLParam, OrderBy, SQL};

/// Marker trait for executable builder states.
pub trait ExecutableState {}

#[derive(Debug, Clone)]
pub struct BuilderInit;

impl ExecutableState for BuilderInit {}

/// Represents an ORDER BY clause in a query.
#[derive(Debug, Clone)]
pub struct OrderByClause<'a, V: SQLParam> {
    /// The expression to order by.
    pub expr: SQL<'a, V>,
    /// The direction to sort (ASC or DESC).
    pub direction: OrderBy,
}

impl<'a, V: SQLParam> OrderByClause<'a, V> {
    /// Creates a new ORDER BY clause.
    pub const fn new(expr: SQL<'a, V>, direction: OrderBy) -> Self {
        Self { expr, direction }
    }
}
