#![allow(clippy::wrong_self_convention)]

use std::marker::PhantomData;

use drizzle_core::traits::{SQLTable, ToSQL};
use drizzle_sqlite::{
    builder::{CTEView, SelectFromSet, SelectInitial, SelectOffsetSet, select::SelectBuilder},
    common::SQLiteSchemaType,
    values::SQLiteValue,
};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{SelectJoinSet, SelectLimitSet, SelectOrderSet, SelectWhereSet},
    traits::{SQLiteTable, ToSQLiteSQL},
};

use crate::builder::sqlite::rusqlite::DrizzleBuilder;
#[cfg(feature = "sqlite")]
use crate::drizzle_builder_join_impl;

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

#[cfg(feature = "sqlite")]
impl<'a, Schema, T>
    DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectFromSet, T>, SelectFromSet>
where
    T: SQLiteTable<'a>,
{
    #[inline]
    pub fn r#where(
        self,
        condition: drizzle_core::sql::SQL<'a, SQLiteValue<'a>>,
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
        TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
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

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    /// Returns a CTEView with typed field access via Deref to the aliased table.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
    > {
        self.builder.as_cte(name)
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
        condition: drizzle_core::sql::SQL<'a, SQLiteValue<'a>>,
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
        TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
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
        TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
    > {
        self.builder.as_cte(name)
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
