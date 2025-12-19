use std::marker::PhantomData;

use drizzle_core::traits::ToSQL;
use drizzle_postgres::{
    PostgresValue,
    builder::{
        UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
        update::UpdateBuilder,
    },
    traits::PostgresTable,
};

use crate::builder::postgres::tokio_postgres::DrizzleBuilder;

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
where
    Table: PostgresTable<'b>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: drizzle_core::sql::SQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateWhereSet, Table>, UpdateWhereSet>
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
        UpdateBuilder<'b, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
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
    DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateWhereSet, Table>, UpdateWhereSet>
{
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'b, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
