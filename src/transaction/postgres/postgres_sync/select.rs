use crate::transaction::postgres::postgres_sync::TransactionBuilder;
use drizzle_core::ToSQL;
use drizzle_postgres::builder::{
    SelectFromSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet,
    SelectWhereSet, select::SelectBuilder,
};
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;
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
        T: ToSQL<'a, PostgresValue<'a>>,
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
    T: PostgresTable<'a>,
{
    #[inline]
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
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
        TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn join<J: drizzle_postgres::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    > {
        let builder = self.builder.join(arg);
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
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
where
    T: PostgresTable<'a>,
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
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
        TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn join<J: drizzle_postgres::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    > {
        let builder = self.builder.join(arg);
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
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    >
where
    T: PostgresTable<'a>,
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
        TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
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
    T: PostgresTable<'a>,
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
    T: PostgresTable<'a>,
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
