use crate::transaction::sqlite::rusqlite::TransactionBuilder;
use crate::transaction_builder_join_impl;
use drizzle_core::ToSQL;
use drizzle_sqlite::builder::{
    SelectFromSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet,
    SelectWhereSet, select::SelectBuilder,
};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;
use std::marker::PhantomData;

impl<'a, 'conn, Schema>
    TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
        SelectFromSet,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.from(table);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, T>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
        SelectFromSet,
    >
where
    T: SQLiteTable<'a>,
{
    #[inline]
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn join<U>(
        self,
        table: U,
        on_condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
    where
        U: SQLiteTable<'a>,
    {
        let builder = self.builder.join(table, on_condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
    transaction_builder_join_impl!('a, 'conn);
}

#[cfg(feature = "sqlite")]
impl<'a, 'conn, Schema, T>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
where
    T: SQLiteTable<'a>,
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn join<U>(
        self,
        table: U,
        condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
    where
        U: SQLiteTable<'a>,
    {
        let builder = self.builder.join(table, condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
    transaction_builder_join_impl!('a, 'conn);
}

#[cfg(feature = "sqlite")]
impl<'a, 'conn, Schema, T>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    >
where
    T: SQLiteTable<'a>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, T>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    >
where
    T: SQLiteTable<'a>,
{
    pub fn offset(
        self,
        offset: usize,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOffsetSet, T>,
        SelectOffsetSet,
    > {
        let builder = self.builder.offset(offset);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, T>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
where
    T: SQLiteTable<'a>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
