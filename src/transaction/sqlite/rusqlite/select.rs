#![allow(clippy::type_complexity)]

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

impl<'a, 'conn, Schema, M>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectInitial, (), M>,
        SelectInitial,
    >
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectFromSet,
            T,
            drizzle_core::Scoped<M, drizzle_core::Cons<T, drizzle_core::Nil>>,
            <M as drizzle_core::ResolveRow<T>>::Row,
        >,
        SelectFromSet,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
        M: drizzle_core::ResolveRow<T>,
    {
        let builder = self.builder.from(table);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectFromSet, T, M, R>,
        SelectFromSet,
    >
where
    T: SQLiteTable<'a>,
{
    #[inline]
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T, M, R>,
        SelectWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
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
        SelectBuilder<'a, Schema, SelectLimitSet, T, M, R>,
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
        SelectBuilder<'a, Schema, SelectOrderSet, T, M, R>,
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
    pub fn join<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectJoinSet,
            J::JoinedTable,
            <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
            <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
        >,
        SelectJoinSet,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
    {
        let builder = self.builder.join(arg);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
    transaction_builder_join_impl!('a, 'conn);
}

#[cfg(feature = "sqlite")]
impl<'a, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T, M, R>,
        SelectJoinSet,
    >
where
    T: SQLiteTable<'a>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T, M, R>,
        SelectWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
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
        SelectBuilder<'a, Schema, SelectOrderSet, T, M, R>,
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

    pub fn join<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectJoinSet,
            J::JoinedTable,
            <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
            <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
        >,
        SelectJoinSet,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
    {
        let builder = self.builder.join(arg);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
    transaction_builder_join_impl!('a, 'conn);
}

#[cfg(feature = "sqlite")]
impl<'a, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T, M, R>,
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
        SelectBuilder<'a, Schema, SelectLimitSet, T, M, R>,
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
        SelectBuilder<'a, Schema, SelectOrderSet, T, M, R>,
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

impl<'a, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T, M, R>,
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
        SelectBuilder<'a, Schema, SelectOffsetSet, T, M, R>,
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

impl<'a, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T, M, R>,
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
        SelectBuilder<'a, Schema, SelectLimitSet, T, M, R>,
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
