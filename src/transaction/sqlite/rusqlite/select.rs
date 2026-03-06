#![allow(clippy::type_complexity)]

use crate::transaction::sqlite::rusqlite::TransactionBuilder;
use crate::transaction_builder_join_impl;
use drizzle_core::ToSQL;
use drizzle_sqlite::builder::{
    CTEView, ExecutableState, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet,
    SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectWhereSet,
    select::{AsCteState, IntoSelect, SelectBuilder, SelectSetOpSet},
};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;
use std::marker::PhantomData;

impl<'tx, 'q, 'conn, Schema, M>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectInitial, (), M>,
        SelectInitial,
    >
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectFromSet,
            T,
            drizzle_core::Scoped<M, drizzle_core::Cons<T, drizzle_core::Nil>>,
            <M as drizzle_core::ResolveRow<T>>::Row,
        >,
        SelectFromSet,
    >
    where
        T: ToSQL<'q, SQLiteValue<'q>>,
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

impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectFromSet, T, M, R>,
        SelectFromSet,
    >
where
    T: SQLiteTable<'q>,
{
    #[inline]
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectWhereSet, T, M, R>,
        SelectWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'q, SQLiteValue<'q>>,
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
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
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
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'q, SQLiteValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'q, SQLiteValue<'q>>>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectGroupSet, T, M, R>,
        SelectGroupSet,
    > {
        let builder = self.builder.group_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn join<J: drizzle_sqlite::helpers::JoinArg<'q, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<
            'q,
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
    transaction_builder_join_impl!('q; 'tx, 'conn);
}

#[cfg(feature = "sqlite")]
impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectJoinSet, T, M, R>,
        SelectJoinSet,
    >
where
    T: SQLiteTable<'q>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectWhereSet, T, M, R>,
        SelectWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'q, SQLiteValue<'q>>,
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
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'q, SQLiteValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn join<J: drizzle_sqlite::helpers::JoinArg<'q, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<
            'q,
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
    transaction_builder_join_impl!('q; 'tx, 'conn);
}

#[cfg(feature = "sqlite")]
impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectWhereSet, T, M, R>,
        SelectWhereSet,
    >
where
    T: SQLiteTable<'q>,
{
    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'q, SQLiteValue<'q>>>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectGroupSet, T, M, R>,
        SelectGroupSet,
    > {
        let builder = self.builder.group_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
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
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'q, SQLiteValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectGroupSet, T, M, R>,
        SelectGroupSet,
    >
{
    pub fn having<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectGroupSet, T, M, R>,
        SelectGroupSet,
    >
    where
        E: drizzle_core::expr::Expr<'q, SQLiteValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.having(condition);
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
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'q, SQLiteValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
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

impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
        SelectLimitSet,
    >
where
    T: SQLiteTable<'q>,
{
    pub fn offset(
        self,
        offset: usize,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOffsetSet, T, M, R>,
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

impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
where
    T: SQLiteTable<'q>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
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

//------------------------------------------------------------------------------
// Set operations on TransactionBuilder
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'tx, 'q, 'conn, Schema, State, T, M, R>
    TransactionBuilder<'tx, 'conn, Schema, SelectBuilder<'q, Schema, State, T, M, R>, State>
where
    State: ExecutableState,
{
    pub fn union(
        self,
        other: impl IntoSelect<'q, Schema, M, R>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.union(other),
            _phantom: PhantomData,
        }
    }

    pub fn union_all(
        self,
        other: impl IntoSelect<'q, Schema, M, R>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.union_all(other),
            _phantom: PhantomData,
        }
    }

    pub fn intersect(
        self,
        other: impl IntoSelect<'q, Schema, M, R>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.intersect(other),
            _phantom: PhantomData,
        }
    }

    pub fn intersect_all(
        self,
        other: impl IntoSelect<'q, Schema, M, R>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.intersect_all(other),
            _phantom: PhantomData,
        }
    }

    pub fn except(
        self,
        other: impl IntoSelect<'q, Schema, M, R>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.except(other),
            _phantom: PhantomData,
        }
    }

    pub fn except_all(
        self,
        other: impl IntoSelect<'q, Schema, M, R>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.except_all(other),
            _phantom: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-SetOp state on TransactionBuilder
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'tx, 'q, 'conn, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    >
{
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'q, SQLiteValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn limit(
        self,
        limit: usize,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn offset(
        self,
        offset: usize,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOffsetSet, T, M, R>,
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

//------------------------------------------------------------------------------
// into_cte on TransactionBuilder
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'tx, 'q, 'conn, Schema, State, T, M, R>
    TransactionBuilder<'tx, 'conn, Schema, SelectBuilder<'q, Schema, State, T, M, R>, State>
where
    State: AsCteState,
    T: drizzle_core::traits::SQLTable<
            'q,
            drizzle_sqlite::common::SQLiteSchemaType,
            SQLiteValue<'q>,
        >,
{
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'q,
        <T as drizzle_core::traits::SQLTable<
            'q,
            drizzle_sqlite::common::SQLiteSchemaType,
            SQLiteValue<'q>,
        >>::Aliased<Tag>,
        SelectBuilder<'q, Schema, State, T, M, R>,
    > {
        self.builder.into_cte::<Tag>()
    }
}
