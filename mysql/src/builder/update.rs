use crate::{common::MySQLSchemaType, values::MySQLValue};
use drizzle_core::{SQLTable, ToSQL};

pub use drizzle_core::builder::{UpdateInitial, UpdateSetClauseSet, UpdateWhereSet};

#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateOrderSet;

#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateLimitSet;

impl drizzle_core::ExecutableState for UpdateOrderSet {}
impl drizzle_core::ExecutableState for UpdateLimitSet {}

pub type UpdateBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

macro_rules! update_prepare {
    ($state:ty) => {
        impl<'a, S, T, M, R> UpdateBuilder<'a, S, $state, T, M, R> {
            #[must_use]
            pub fn prepare(&self) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
                self.prepared_statement()
            }
        }
    };
}

update_prepare!(UpdateSetClauseSet);
update_prepare!(UpdateWhereSet);
update_prepare!(UpdateOrderSet);
update_prepare!(UpdateLimitSet);

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
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
    pub fn r#where<E>(self, condition: E) -> UpdateBuilder<'a, S, UpdateWhereSet, T>
    where
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        UpdateBuilder::from_sql(self.sql.append(crate::helpers::r#where(condition)))
    }
}

macro_rules! update_order_by {
    ($state:ty) => {
        impl<'a, S, T> UpdateBuilder<'a, S, $state, T> {
            pub fn order_by<O>(self, order: O) -> UpdateBuilder<'a, S, UpdateOrderSet, T>
            where
                O: ToSQL<'a, MySQLValue<'a>>,
            {
                UpdateBuilder::from_sql(self.sql.append(crate::helpers::order_by(order)))
            }
        }
    };
}

update_order_by!(UpdateSetClauseSet);
update_order_by!(UpdateWhereSet);

macro_rules! update_limit {
    ($state:ty) => {
        impl<'a, S, T> UpdateBuilder<'a, S, $state, T> {
            #[track_caller]
            pub fn limit<P>(self, limit: P) -> UpdateBuilder<'a, S, UpdateLimitSet, T>
            where
                P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
            {
                UpdateBuilder::from_sql(self.sql.append(crate::helpers::limit(limit)))
            }
        }
    };
}

update_limit!(UpdateSetClauseSet);
update_limit!(UpdateWhereSet);
update_limit!(UpdateOrderSet);
