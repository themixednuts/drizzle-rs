use crate::values::MySQLValue;
pub use drizzle_core::builder::{DeleteInitial, DeleteWhereSet};

/// Marker for a `DELETE` with an `ORDER BY` clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteOrderSet;

/// Marker for a `DELETE` with a `LIMIT` clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteLimitSet;

impl drizzle_core::ExecutableState for DeleteOrderSet {}
impl drizzle_core::ExecutableState for DeleteLimitSet {}

/// Typed MySQL `DELETE` builder.
pub type DeleteBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    /// Filters the rows deleted by this statement.
    pub fn r#where<E>(self, condition: E) -> DeleteBuilder<'a, S, DeleteWhereSet, T>
    where
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DeleteBuilder::from_sql(self.sql.append(crate::helpers::r#where(condition)))
    }
}

mutation_builder_methods!(
    DeleteBuilder,
    prepare: [DeleteInitial, DeleteWhereSet, DeleteOrderSet, DeleteLimitSet],
    order_by: [DeleteInitial, DeleteWhereSet] => DeleteOrderSet,
    limit: [DeleteInitial, DeleteWhereSet, DeleteOrderSet] => DeleteLimitSet,
);
