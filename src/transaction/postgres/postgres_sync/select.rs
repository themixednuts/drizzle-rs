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
        T: ToSQL<'a, PostgresValue<'a>>,
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

macro_rules! impl_tx_select_methods {
    ($($state:ty => [$($method:ident),* $(,)?]),+ $(,)?) => {
        $( impl<'a, 'conn, Schema, T, M, R, G>
            TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, $state, T, M, R, G>, $state>
        { $( impl_tx_select_methods!(@method $method); )* } )+
    };
    (@method r#where) => {
        #[inline]
        pub fn r#where<E>(self, condition: E) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T, M, R, G>, SelectWhereSet>
        where E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>, E::SQLType: drizzle_core::types::BooleanLike,
        { let builder = self.builder.r#where(condition); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
    (@method group_by) => {
        pub fn group_by<Gr>(self, columns: Gr) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, M, R, Gr::Columns>, SelectGroupSet>
        where Gr: drizzle_core::IntoGroupBy<'a, PostgresValue<'a>>,
        { let builder = self.builder.group_by(columns); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
    (@method having) => {
        pub fn having<E>(self, condition: E) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, M, R, G>, SelectGroupSet>
        where E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>, E::SQLType: drizzle_core::types::BooleanLike,
        { let builder = self.builder.having(condition); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
    (@method order_by) => {
        pub fn order_by<TOrderBy>(self, expressions: TOrderBy) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T, M, R, G>, SelectOrderSet>
        where TOrderBy: drizzle_core::ToSQL<'a, PostgresValue<'a>>,
        { let builder = self.builder.order_by(expressions); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
    (@method limit) => {
        #[inline]
        pub fn limit(self, limit: usize) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T, M, R, G>, SelectLimitSet>
        { let builder = self.builder.limit(limit); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
    (@method offset) => {
        pub fn offset(self, offset: usize) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectOffsetSet, T, M, R, G>, SelectOffsetSet>
        { let builder = self.builder.offset(offset); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
    (@method join) => {
        #[inline]
        pub fn join<J: drizzle_postgres::helpers::JoinArg<'a, T>>(self, arg: J) -> TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow, G>, SelectJoinSet>
        where M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
        { let builder = self.builder.join(arg); TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData } }
    };
}

impl_tx_select_methods! {
    SelectFromSet  => [r#where, group_by, order_by, limit, offset, join],
    SelectJoinSet  => [r#where, group_by, order_by, join],
    SelectWhereSet => [group_by, order_by, limit],
    SelectGroupSet => [having, order_by, limit],
    SelectOrderSet => [limit],
    SelectLimitSet => [offset],
    SelectSetOpSet => [order_by, limit, offset],
}

//------------------------------------------------------------------------------
// Set operations on TransactionBuilder
//------------------------------------------------------------------------------

impl<'a, 'conn, Schema, State, T, M, R, G>
    TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: ExecutableState,
{
    pub fn union(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R, G>,
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
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R, G>,
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
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R, G>,
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
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R, G>,
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
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R, G>,
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
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R, G>,
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
// into_cte on TransactionBuilder
//------------------------------------------------------------------------------

impl<'a, 'conn, Schema, State, T, M, R, G>
    TransactionBuilder<'a, 'conn, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: AsCteState,
    T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased<Tag>,
        SelectBuilder<'a, Schema, State, T, M, R, G>,
    > {
        self.builder.into_cte::<Tag>()
    }
}
