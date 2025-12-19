use std::marker::PhantomData;

use drizzle_core::traits::ToSQL;
use drizzle_postgres::{
    PostgresValue,
    builder::{DeleteInitial, DeleteReturningSet, DeleteWhereSet, delete::DeleteBuilder},
    traits::PostgresTable,
};

use crate::builder::postgres::postgres_sync::DrizzleBuilder;

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, Table>, DeleteInitial>
where
    Table: PostgresTable<'b>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::sql::SQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteWhereSet, Table>, DeleteWhereSet>
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        DeleteBuilder<'b, Schema, DeleteReturningSet, Table>,
        DeleteReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteWhereSet, Table>, DeleteWhereSet>
{
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        DeleteBuilder<'b, Schema, DeleteReturningSet, Table>,
        DeleteReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
