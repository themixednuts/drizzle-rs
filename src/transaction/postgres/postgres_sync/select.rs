#![allow(clippy::type_complexity)]

use crate::transaction::postgres::postgres_sync::TransactionBuilder;
use drizzle_core::ToSQL;
use drizzle_core::traits::SQLTable;
use drizzle_postgres::builder::{
    CTEView, ExecutableState, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet,
    SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
    select::{AsCteState, IntoSelect, SelectBuilder},
};
use drizzle_postgres::common::PostgresSchemaType;
use drizzle_postgres::values::PostgresValue;
use std::marker::PhantomData;

impl<'tx, 'conn, 'q, Schema, M>
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
        T: ToSQL<'q, PostgresValue<'q>>,
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

impl<'tx, 'conn, 'q, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectFromSet, T, M, R>,
        SelectFromSet,
    >
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
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
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
        TOrderBy: drizzle_core::ToSQL<'q, PostgresValue<'q>>,
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
        expressions: impl IntoIterator<Item = impl ToSQL<'q, PostgresValue<'q>>>,
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
    pub fn join<J: drizzle_postgres::helpers::JoinArg<'q, T>>(
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
}

impl<'tx, 'conn, 'q, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectJoinSet, T, M, R>,
        SelectJoinSet,
    >
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
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
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
        TOrderBy: drizzle_core::ToSQL<'q, PostgresValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn join<J: drizzle_postgres::helpers::JoinArg<'q, T>>(
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
}

impl<'tx, 'conn, 'q, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectWhereSet, T, M, R>,
        SelectWhereSet,
    >
{
    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'q, PostgresValue<'q>>>,
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
        TOrderBy: drizzle_core::ToSQL<'q, PostgresValue<'q>>,
    {
        let builder = self.builder.order_by(expressions);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-GROUP BY State on TransactionBuilder
//------------------------------------------------------------------------------

impl<'tx, 'conn, 'q, Schema, T, M, R>
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
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
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
        TOrderBy: drizzle_core::ToSQL<'q, PostgresValue<'q>>,
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

impl<'tx, 'conn, 'q, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectLimitSet, T, M, R>,
        SelectLimitSet,
    >
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

impl<'tx, 'conn, 'q, Schema, T, M, R>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R>,
        SelectOrderSet,
    >
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

impl<'tx, 'conn, 'q, Schema, State, T, M, R>
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

impl<'tx, 'conn, 'q, Schema, T, M, R>
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
        TOrderBy: drizzle_core::ToSQL<'q, PostgresValue<'q>>,
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

impl<'tx, 'conn, 'q, Schema, State, T, M, R>
    TransactionBuilder<'tx, 'conn, Schema, SelectBuilder<'q, Schema, State, T, M, R>, State>
where
    State: AsCteState,
    T: SQLTable<'q, PostgresSchemaType, PostgresValue<'q>>,
{
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'q,
        <T as SQLTable<'q, PostgresSchemaType, PostgresValue<'q>>>::Aliased<Tag>,
        SelectBuilder<'q, Schema, State, T, M, R>,
    > {
        self.builder.into_cte::<Tag>()
    }
}
