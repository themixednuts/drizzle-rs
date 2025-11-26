use crate::drizzle::sqlite::libsql::DrizzleBuilder;
use crate::drizzle_builder_join_impl;
use drizzle_core::ToSQL;
use drizzle_sqlite::builder::{SelectJoinSet, SelectLimitSet, SelectOrderSet, SelectWhereSet};
use drizzle_sqlite::traits::{SQLiteTable, ToSQLiteSQL};
use drizzle_sqlite::{
    SQLiteValue,
    builder::{SelectFromSet, SelectInitial, SelectOffsetSet, select::SelectBuilder},
};
use std::marker::PhantomData;

impl<'a, Schema>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectFromSet, T>, SelectFromSet>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
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
    T: SQLiteTable<'a>,
{
    #[inline]
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
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
        on_condition: impl ToSQLiteSQL<'a>,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
    where
        U: SQLiteTable<'a>,
    {
        let builder = self.builder.join(table, on_condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
    drizzle_builder_join_impl!();
}

#[cfg(feature = "sqlite")]
impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
where
    T: SQLiteTable<'a>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
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
        condition: impl ToSQLiteSQL<'a>,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
    where
        U: SQLiteTable<'a>,
    {
        let builder = self.builder.join(table, condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
    drizzle_builder_join_impl!();
}

#[cfg(feature = "sqlite")]
impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T>, SelectWhereSet>
where
    T: SQLiteTable<'a>,
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
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
    T: SQLiteTable<'a>,
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
    T: SQLiteTable<'a>,
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
