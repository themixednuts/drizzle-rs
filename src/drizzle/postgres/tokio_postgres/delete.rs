use std::marker::PhantomData;

use drizzle_core::ToSQL;
use drizzle_postgres::{
    PostgresValue,
    builder::{DeleteInitial, DeleteReturningSet, DeleteWhereSet, delete::DeleteBuilder},
    traits::PostgresTable,
};

use crate::drizzle::postgres::tokio_postgres::DrizzleBuilder;

impl<'a, Schema, Table>
    DrizzleBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteInitial, Table>, DeleteInitial>
where
    Table: PostgresTable<'a>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteWhereSet, Table>, DeleteWhereSet>
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
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        DeleteBuilder<'a, Schema, DeleteReturningSet, Table>,
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

impl<'a, Schema, Table>
    DrizzleBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteWhereSet, Table>, DeleteWhereSet>
{
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        DeleteBuilder<'a, Schema, DeleteReturningSet, Table>,
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
