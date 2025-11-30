use std::marker::PhantomData;

use drizzle_core::{SQLTable, ToSQL};
use drizzle_postgres::{
    PostgresValue,
    builder::{CTEView, SelectFromSet, SelectInitial, SelectOffsetSet, select::SelectBuilder},
    common::PostgresSchemaType,
};
use drizzle_postgres::{
    builder::{SelectJoinSet, SelectLimitSet, SelectOrderSet, SelectWhereSet},
    traits::PostgresTable,
    ToPostgresSQL,
};

use crate::drizzle::postgres::tokio_postgres::DrizzleBuilder;

impl<'a, Schema>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectFromSet, T>, SelectFromSet>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.from(table);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectFromSet, T>, SelectFromSet>
where
    T: PostgresTable<'a>,
{
    #[inline]
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T>, SelectWhereSet>
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T>, SelectLimitSet>
    {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T>, SelectOrderSet>
    where
        TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn join<U>(
        self,
        table: U,
        on_condition: impl ToPostgresSQL<'a>,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
    where
        U: PostgresTable<'a>,
    {
        let builder = self.builder.join(table, on_condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
where
    T: PostgresTable<'a>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T>, SelectWhereSet>
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T>, SelectOrderSet>
    where
        TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn join<U>(
        self,
        table: U,
        condition: impl ToPostgresSQL<'a>,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
    where
        U: PostgresTable<'a>,
    {
        let builder = self.builder.join(table, condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T>, SelectWhereSet>
where
    T: PostgresTable<'a>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T>, SelectLimitSet>
    {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T>, SelectOrderSet>
    where
        TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

}

impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T>, SelectLimitSet>
where
    T: PostgresTable<'a>,
{
    pub fn offset(
        self,
        offset: usize,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectOffsetSet, T>, SelectOffsetSet>
    {
        let builder = self.builder.offset(offset);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T>, SelectOrderSet>
where
    T: PostgresTable<'a>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T>, SelectLimitSet>
    {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
