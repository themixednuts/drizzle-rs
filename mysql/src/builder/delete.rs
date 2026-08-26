use crate::values::MySQLValue;
use drizzle_core::ToSQL;

pub use drizzle_core::builder::{DeleteInitial, DeleteWhereSet};

#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteOrderSet;

#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteLimitSet;

impl drizzle_core::ExecutableState for DeleteOrderSet {}
impl drizzle_core::ExecutableState for DeleteLimitSet {}

pub type DeleteBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

macro_rules! delete_prepare {
    ($state:ty) => {
        impl<'a, S, T, M, R> DeleteBuilder<'a, S, $state, T, M, R> {
            #[must_use]
            pub fn prepare(&self) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
                self.prepared_statement()
            }
        }
    };
}

delete_prepare!(DeleteInitial);
delete_prepare!(DeleteWhereSet);
delete_prepare!(DeleteOrderSet);
delete_prepare!(DeleteLimitSet);

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    pub fn r#where<E>(self, condition: E) -> DeleteBuilder<'a, S, DeleteWhereSet, T>
    where
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DeleteBuilder::from_sql(self.sql.append(crate::helpers::r#where(condition)))
    }
}

macro_rules! delete_order_by {
    ($state:ty) => {
        impl<'a, S, T> DeleteBuilder<'a, S, $state, T> {
            pub fn order_by<O>(self, order: O) -> DeleteBuilder<'a, S, DeleteOrderSet, T>
            where
                O: ToSQL<'a, MySQLValue<'a>>,
            {
                DeleteBuilder::from_sql(self.sql.append(crate::helpers::order_by(order)))
            }
        }
    };
}

delete_order_by!(DeleteInitial);
delete_order_by!(DeleteWhereSet);

macro_rules! delete_limit {
    ($state:ty) => {
        impl<'a, S, T> DeleteBuilder<'a, S, $state, T> {
            #[track_caller]
            pub fn limit<P>(self, limit: P) -> DeleteBuilder<'a, S, DeleteLimitSet, T>
            where
                P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
            {
                DeleteBuilder::from_sql(self.sql.append(crate::helpers::limit(limit)))
            }
        }
    };
}

delete_limit!(DeleteInitial);
delete_limit!(DeleteWhereSet);
delete_limit!(DeleteOrderSet);
