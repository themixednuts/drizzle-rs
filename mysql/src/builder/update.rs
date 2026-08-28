use crate::{common::MySQLSchemaType, values::MySQLValue};
use drizzle_core::SQLTable;

pub use drizzle_core::builder::{UpdateInitial, UpdateSetClauseSet, UpdateWhereSet};

/// Marker for an `UPDATE` with an `ORDER BY` clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateOrderSet;

/// Marker for an `UPDATE` with a `LIMIT` clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateLimitSet;

impl drizzle_core::ExecutableState for UpdateOrderSet {}
impl drizzle_core::ExecutableState for UpdateLimitSet {}

/// Typed MySQL `UPDATE` builder.
pub type UpdateBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
    /// Sets the generated update model applied to matching rows.
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<Table, MySQLSchemaType, MySQLValue<'a>>(&values);
        drop(values);
        UpdateBuilder::from_sql(self.sql.append(sql))
    }
}

impl<'a, S, T> UpdateBuilder<'a, S, UpdateSetClauseSet, T> {
    /// Filters the rows updated by this statement.
    pub fn r#where<E>(self, condition: E) -> UpdateBuilder<'a, S, UpdateWhereSet, T>
    where
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        UpdateBuilder::from_sql(self.sql.append(crate::helpers::r#where(condition)))
    }
}

mutation_builder_methods!(
    UpdateBuilder,
    prepare: [UpdateSetClauseSet, UpdateWhereSet, UpdateOrderSet, UpdateLimitSet],
    order_by: [UpdateSetClauseSet, UpdateWhereSet] => UpdateOrderSet,
    limit: [UpdateSetClauseSet, UpdateWhereSet, UpdateOrderSet] => UpdateLimitSet,
);
