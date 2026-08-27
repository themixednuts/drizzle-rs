use crate::values::MySQLValue;
pub use drizzle_core::builder::{DeleteInitial, DeleteWhereSet};

#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteOrderSet;

#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteLimitSet;

impl drizzle_core::ExecutableState for DeleteOrderSet {}
impl drizzle_core::ExecutableState for DeleteLimitSet {}

pub type DeleteBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
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
