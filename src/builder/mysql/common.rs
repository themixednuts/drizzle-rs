//! Driver-independent wrappers that attach a MySQL query to an executor.

#![allow(clippy::type_complexity)]

use core::marker::PhantomData;

use drizzle_core::{SQLIndex, SQLTable, ToSQL};
use drizzle_mysql::{
    builder::{
        self, CTEView, DeleteBuilder, DeleteInitial, DeleteLimitSet, DeleteOrderSet,
        DeleteWhereSet, ForShare, ForUpdate, InsertBuilder, InsertIgnoreSet, InsertInitial,
        InsertOnDuplicateKeyUpdateSet, InsertValuesSet, IntoSelect, IntoSelectQuery, NoWait,
        QueryBuilder, SelectBuilder, SelectForSet, SelectFromSet, SelectGroupSet, SelectHavingSet,
        SelectIndexHintSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
        SelectOrderSet, SelectSetOpSet, SelectWhereSet, SkipLocked, UpdateBuilder, UpdateInitial,
        UpdateLimitSet, UpdateOrderSet, UpdateSetClauseSet, UpdateWhereSet, Wait,
    },
    common::MySQLSchemaType,
    traits::{MySQLInsertSelectTarget, MySQLTable},
    values::MySQLValue,
};

/// A MySQL dialect builder attached to a concrete driver runner.
#[derive(Debug)]
pub struct DrizzleBuilder<'db, Runner, Schema, Builder, State> {
    pub(crate) runner: Runner,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State, &'db ())>,
}

impl<'db, Runner, Schema, Builder, State> DrizzleBuilder<'db, Runner, Schema, Builder, State> {
    #[inline]
    pub(crate) const fn new(runner: Runner, builder: Builder) -> Self {
        Self {
            runner,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    fn map<Next, NextState>(
        self,
        transform: impl FnOnce(Builder) -> Next,
    ) -> DrizzleBuilder<'db, Runner, Schema, Next, NextState> {
        DrizzleBuilder::new(self.runner, transform(self.builder))
    }

    /// Releases the driver borrow and returns the dialect builder.
    ///
    /// This is useful when a completed select becomes the right-hand side of
    /// a set operation or the source of an insert-select built on the same
    /// connection.
    #[must_use]
    pub fn detach(self) -> Builder {
        self.builder
    }
}

impl<'q, Runner, Schema, Builder, State> ToSQL<'q, MySQLValue<'q>>
    for DrizzleBuilder<'_, Runner, Schema, Builder, State>
where
    Builder: ToSQL<'q, MySQLValue<'q>>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        self.builder.to_sql()
    }

    fn into_sql(self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        self.builder.into_sql()
    }
}

impl<'q, Runner, Schema, Builder, State> drizzle_core::expr::Expr<'q, MySQLValue<'q>>
    for DrizzleBuilder<'_, Runner, Schema, Builder, State>
where
    Builder: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
{
    type SQLType = Builder::SQLType;
    type Nullable = Builder::Nullable;
    type Aggregate = Builder::Aggregate;
}

impl<'db, 'q, Runner, Schema>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        QueryBuilder<'q, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
{
    pub fn select<T>(
        self,
        columns: T,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
    {
        self.map(|builder| builder.select(columns))
    }

    pub fn select_distinct<T>(
        self,
        columns: T,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
    {
        self.map(|builder| builder.select_distinct(columns))
    }

    pub fn update<Table>(
        self,
        table: Table,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        UpdateBuilder<'q, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
    where
        Table: MySQLTable<'q>,
    {
        self.map(|builder| builder.update(table))
    }

    pub fn delete<Table>(
        self,
        table: Table,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        DeleteBuilder<'q, Schema, DeleteInitial, Table>,
        DeleteInitial,
    >
    where
        Table: MySQLTable<'q>,
    {
        self.map(|builder| builder.delete(table))
    }

    #[must_use]
    pub fn with<C>(self, cte: &C) -> Self
    where
        C: builder::CTEDefinition<'q>,
    {
        self.map(|builder| builder.with(cte))
    }
}

impl<'db, 'q, Runner, Schema, M>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectInitial, (), M>,
        SelectInitial,
    >
{
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<
        'db,
        Runner,
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
        T: ToSQL<'q, MySQLValue<'q>>,
        M: drizzle_core::ResolveRow<T>,
    {
        self.map(|builder| builder.from(table))
    }
}

macro_rules! select_method {
    (where) => {
        pub fn r#where<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<'q, Schema, SelectWhereSet, T, M, R, G>,
            SelectWhereSet,
        >
        where
            E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            self.map(|builder| builder.r#where(condition))
        }
    };
    (group_by) => {
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<'q, Schema, SelectGroupSet, T, M, R, Gr::Columns>,
            SelectGroupSet,
        >
        where
            Gr: drizzle_core::IntoGroupBy<'q, MySQLValue<'q>>,
        {
            self.map(|builder| builder.group_by(columns))
        }
    };
    (having) => {
        pub fn having<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<'q, Schema, SelectHavingSet, T, M, R, G>,
            SelectHavingSet,
        >
        where
            E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            self.map(|builder| builder.having(condition))
        }
    };
    (order_by) => {
        pub fn order_by<O>(
            self,
            order: O,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<'q, Schema, SelectOrderSet, T, M, R, G>,
            SelectOrderSet,
        >
        where
            O: ToSQL<'q, MySQLValue<'q>>,
        {
            self.map(|builder| builder.order_by(order))
        }
    };
    (limit) => {
        pub fn limit<P>(
            self,
            limit: P,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<'q, Schema, SelectLimitSet, T, M, R, G>,
            SelectLimitSet,
        >
        where
            P: drizzle_core::PaginationArg<'q, MySQLValue<'q>>,
        {
            self.map(|builder| builder.limit(limit))
        }
    };
    (offset) => {
        pub fn offset<P>(
            self,
            offset: P,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<'q, Schema, SelectOffsetSet, T, M, R, G>,
            SelectOffsetSet,
        >
        where
            P: drizzle_core::PaginationArg<'q, MySQLValue<'q>>,
        {
            self.map(|builder| builder.offset(offset))
        }
    };
    (joins) => {
        pub fn join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.join(arg))
        }

        pub fn inner_join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.inner_join(arg))
        }

        pub fn cross_join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.cross_join(arg))
        }

        pub fn left_join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterLeftJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterLeftJoin<R, J::JoinedTable>
                + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.left_join(arg))
        }

        pub fn left_outer_join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterLeftJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterLeftJoin<R, J::JoinedTable>
                + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.left_outer_join(arg))
        }

        pub fn right_join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterRightJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterRightJoin<R, J::JoinedTable>
                + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.right_join(arg))
        }

        pub fn right_outer_join<J>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
                <M as drizzle_core::AfterRightJoin<R, J::JoinedTable>>::NewRow,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::AfterRightJoin<R, J::JoinedTable>
                + drizzle_core::ScopePush<J::JoinedTable>,
        {
            self.map(|builder| builder.right_outer_join(arg))
        }
    };
}

macro_rules! select_states {
    ($($state:ty => [$($method:ident),* $(,)?]),+ $(,)?) => {$ (
        impl<'db, 'q, Runner, Schema, T, M, R, G>
            DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, $state, T, M, R, G>, $state>
        { $(select_method!($method);)* }
    )+ };
}

select_states! {
    SelectFromSet => [where, group_by, order_by, limit, offset, joins],
    SelectJoinSet => [where, group_by, order_by, limit, offset, joins],
    SelectWhereSet => [group_by, order_by, limit, offset],
    SelectGroupSet => [having, order_by, limit, offset],
    SelectHavingSet => [order_by, limit, offset],
    SelectOrderSet => [limit, offset],
    SelectLimitSet => [offset],
}

impl<'db, 'q, Runner, Schema, Kind, T, M, R, G>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectIndexHintSet<Kind>, T, M, R, G>,
        SelectIndexHintSet<Kind>,
    >
{
    select_method!(where);
    select_method!(group_by);
    select_method!(order_by);
    select_method!(limit);
    select_method!(offset);
    select_method!(joins);
}

impl<'db, 'q, Runner, Schema, T, M, R, G>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>,
        SelectSetOpSet,
    >
{
    pub fn order_by<O, Proof>(
        self,
        order: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectOrderSet, T, M, R, G>,
        SelectOrderSet,
    >
    where
        O: drizzle_mysql::helpers::SetOrderBy<'q, M, T, Proof>,
    {
        self.map(|builder| builder.order_by::<O, Proof>(order))
    }

    select_method!(limit);
    select_method!(offset);
}

impl<'db, 'q, Runner, Schema, T, M, R, G>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectFromSet, T, M, R, G>,
        SelectFromSet,
    >
where
    T: MySQLTable<'q>,
{
    pub fn use_index<Index>(
        self,
        index: Index,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectIndexHintSet<drizzle_mysql::helpers::UseIndex>, T, M, R, G>,
        SelectIndexHintSet<drizzle_mysql::helpers::UseIndex>,
    >
    where
        Index: SQLIndex<'q, MySQLSchemaType, MySQLValue<'q>, Table = T>,
    {
        self.map(|builder| builder.use_index(index))
    }

    pub fn force_index<Index>(
        self,
        index: Index,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectIndexHintSet<drizzle_mysql::helpers::ForceIndex>,
            T,
            M,
            R,
            G,
        >,
        SelectIndexHintSet<drizzle_mysql::helpers::ForceIndex>,
    >
    where
        Index: SQLIndex<'q, MySQLSchemaType, MySQLValue<'q>, Table = T>,
    {
        self.map(|builder| builder.force_index(index))
    }

    pub fn ignore_index<Index>(
        self,
        index: Index,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectIndexHintSet<drizzle_mysql::helpers::IgnoreIndex>,
            T,
            M,
            R,
            G,
        >,
        SelectIndexHintSet<drizzle_mysql::helpers::IgnoreIndex>,
    >
    where
        Index: SQLIndex<'q, MySQLSchemaType, MySQLValue<'q>, Table = T>,
    {
        self.map(|builder| builder.ignore_index(index))
    }
}

macro_rules! set_operations {
    ($state:ty $(, $extra:ident)*) => {
        impl<'db, 'q, Runner, Schema, T, M, R, G, $($extra),*>
            DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, $state, T, M, R, G>, $state>
        {
            pub fn union(self, other: impl IntoSelectQuery<'q, Schema, R>) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>, SelectSetOpSet> { self.map(|builder| builder.union(other)) }
            pub fn union_all(self, other: impl IntoSelectQuery<'q, Schema, R>) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>, SelectSetOpSet> { self.map(|builder| builder.union_all(other)) }
            pub fn intersect(self, other: impl IntoSelectQuery<'q, Schema, R>) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>, SelectSetOpSet> { self.map(|builder| builder.intersect(other)) }
            pub fn intersect_all(self, other: impl IntoSelectQuery<'q, Schema, R>) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>, SelectSetOpSet> { self.map(|builder| builder.intersect_all(other)) }
            pub fn except(self, other: impl IntoSelectQuery<'q, Schema, R>) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>, SelectSetOpSet> { self.map(|builder| builder.except(other)) }
            pub fn except_all(self, other: impl IntoSelectQuery<'q, Schema, R>) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>, SelectSetOpSet> { self.map(|builder| builder.except_all(other)) }
        }
    };
}

set_operations!(SelectFromSet);
set_operations!(SelectJoinSet);
set_operations!(SelectWhereSet);
set_operations!(SelectGroupSet);
set_operations!(SelectHavingSet);
set_operations!(SelectOrderSet);
set_operations!(SelectLimitSet);
set_operations!(SelectOffsetSet);
set_operations!(SelectSetOpSet);

impl<'q, Runner, Schema, State, T, M, R, G> IntoSelectQuery<'q, Schema, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    SelectBuilder<'q, Schema, State, T, M, R, G>: IntoSelect<'q, Schema, R, Marker = M>,
{
    type Marker = M;
    type Select = SelectBuilder<'q, Schema, State, T, M, R, G>;

    fn into_select_query(self) -> Self::Select {
        self.builder
    }
}

impl<Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, QueryBuilder<'_, Schema, State, T, M, R, G>, State>
where
    State: builder::ExecutableState,
{
    pub fn comment(self, text: impl AsRef<str>) -> Self {
        self.map(|builder| builder.comment(text))
    }

    pub fn comment_tags<I, K, V>(self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.map(|builder| builder.comment_tags(pairs))
    }
}

impl<'db, 'q, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: builder::select::AsCteState + builder::ExecutableState,
    T: SQLTable<'q, MySQLSchemaType, MySQLValue<'q>>,
{
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'q,
        <T as SQLTable<'q, MySQLSchemaType, MySQLValue<'q>>>::Aliased<Tag>,
        SelectBuilder<'q, Schema, State, T, M, R, G>,
    > {
        self.builder.into_cte::<Tag>()
    }
}

macro_rules! insert_sources {
    ($state:ty) => {
        impl<'db, 'q, Runner, Schema, Table>
            DrizzleBuilder<'db, Runner, Schema, InsertBuilder<'q, Schema, $state, Table>, $state>
        where
            Table: MySQLTable<'q>,
        {
            pub fn value<T>(
                self,
                value: Table::Insert<T>,
            ) -> DrizzleBuilder<
                'db,
                Runner,
                Schema,
                InsertBuilder<'q, Schema, InsertValuesSet, Table>,
                InsertValuesSet,
            > {
                self.map(|builder| builder.value(value))
            }

            pub fn values<I, T>(
                self,
                values: I,
            ) -> DrizzleBuilder<
                'db,
                Runner,
                Schema,
                InsertBuilder<'q, Schema, InsertValuesSet, Table>,
                InsertValuesSet,
            >
            where
                I: IntoIterator<Item = Table::Insert<T>>,
            {
                self.map(|builder| builder.values(values))
            }

            pub fn select<Q, R>(
                self,
                query: Q,
            ) -> DrizzleBuilder<
                'db,
                Runner,
                Schema,
                InsertBuilder<'q, Schema, InsertValuesSet, Table>,
                InsertValuesSet,
            >
            where
                Table: MySQLInsertSelectTarget,
                Q: IntoSelectQuery<'q, Schema, R>,
                Q::Marker: builder::insert::InsertSelectCompatible<'q, Table, R>,
            {
                self.map(|builder| builder.select(query))
            }
        }
    };
}

insert_sources!(InsertInitial);
insert_sources!(InsertIgnoreSet);

impl<'db, 'q, Runner, Schema, Table>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        InsertBuilder<'q, Schema, InsertInitial, Table>,
        InsertInitial,
    >
where
    Table: MySQLTable<'q>,
{
    pub fn ignore(
        self,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        InsertBuilder<'q, Schema, InsertIgnoreSet, Table>,
        InsertIgnoreSet,
    > {
        self.map(|builder| builder.ignore())
    }
}

impl<'db, 'q, Runner, Schema, Table, M, R>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        InsertBuilder<'q, Schema, InsertValuesSet, Table, M, R>,
        InsertValuesSet,
    >
where
    Table: MySQLTable<'q>,
{
    pub fn on_duplicate_key_update(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        InsertBuilder<'q, Schema, InsertOnDuplicateKeyUpdateSet, Table, M, R>,
        InsertOnDuplicateKeyUpdateSet,
    > {
        self.map(|builder| builder.on_duplicate_key_update(values))
    }
}

impl<'db, 'q, Runner, Schema, Table>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        UpdateBuilder<'q, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: SQLTable<'q, MySQLSchemaType, MySQLValue<'q>>,
{
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        UpdateBuilder<'q, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        self.map(|builder| builder.set(values))
    }
}

macro_rules! update_methods {
    ($state:ty => [$($method:ident),* $(,)?]) => {
        impl<'db, 'q, Runner, Schema, Table> DrizzleBuilder<'db, Runner, Schema, UpdateBuilder<'q, Schema, $state, Table>, $state> {
            $(update_methods!(@method $method);)*
        }
    };
    (@method where) => { pub fn r#where<E>(self, condition: E) -> DrizzleBuilder<'db, Runner, Schema, UpdateBuilder<'q, Schema, UpdateWhereSet, Table>, UpdateWhereSet> where E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>, E::SQLType: drizzle_core::types::BooleanLike { self.map(|builder| builder.r#where(condition)) } };
    (@method order_by) => { pub fn order_by<O>(self, order: O) -> DrizzleBuilder<'db, Runner, Schema, UpdateBuilder<'q, Schema, UpdateOrderSet, Table>, UpdateOrderSet> where O: ToSQL<'q, MySQLValue<'q>> { self.map(|builder| builder.order_by(order)) } };
    (@method limit) => { pub fn limit<P>(self, limit: P) -> DrizzleBuilder<'db, Runner, Schema, UpdateBuilder<'q, Schema, UpdateLimitSet, Table>, UpdateLimitSet> where P: drizzle_core::PaginationArg<'q, MySQLValue<'q>> { self.map(|builder| builder.limit(limit)) } };
}

update_methods!(UpdateSetClauseSet => [where, order_by, limit]);
update_methods!(UpdateWhereSet => [order_by, limit]);
update_methods!(UpdateOrderSet => [limit]);

macro_rules! delete_methods {
    ($state:ty => [$($method:ident),* $(,)?]) => {
        impl<'db, 'q, Runner, Schema, Table> DrizzleBuilder<'db, Runner, Schema, DeleteBuilder<'q, Schema, $state, Table>, $state> {
            $(delete_methods!(@method $method);)*
        }
    };
    (@method where) => { pub fn r#where<E>(self, condition: E) -> DrizzleBuilder<'db, Runner, Schema, DeleteBuilder<'q, Schema, DeleteWhereSet, Table>, DeleteWhereSet> where E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>, E::SQLType: drizzle_core::types::BooleanLike { self.map(|builder| builder.r#where(condition)) } };
    (@method order_by) => { pub fn order_by<O>(self, order: O) -> DrizzleBuilder<'db, Runner, Schema, DeleteBuilder<'q, Schema, DeleteOrderSet, Table>, DeleteOrderSet> where O: ToSQL<'q, MySQLValue<'q>> { self.map(|builder| builder.order_by(order)) } };
    (@method limit) => { pub fn limit<P>(self, limit: P) -> DrizzleBuilder<'db, Runner, Schema, DeleteBuilder<'q, Schema, DeleteLimitSet, Table>, DeleteLimitSet> where P: drizzle_core::PaginationArg<'q, MySQLValue<'q>> { self.map(|builder| builder.limit(limit)) } };
}

delete_methods!(DeleteInitial => [where, order_by, limit]);
delete_methods!(DeleteWhereSet => [order_by, limit]);
delete_methods!(DeleteOrderSet => [limit]);

macro_rules! locking_reads {
    ($state:ty $(, $extra:ident)*) => {
        impl<'db, 'q, Runner, Schema, T, M, R, G, $($extra),*>
            DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, $state, T, M, R, G>, $state>
        {
            pub fn for_update(self) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectForSet<ForUpdate>, T, M, R, G>, SelectForSet<ForUpdate>> { self.map(|builder| builder.for_update()) }
            pub fn for_share(self) -> DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, SelectForSet<ForShare>, T, M, R, G>, SelectForSet<ForShare>> { self.map(|builder| builder.for_share()) }
        }
    };
}

locking_reads!(SelectFromSet);
locking_reads!(SelectIndexHintSet<Kind>, Kind);
locking_reads!(SelectJoinSet);
locking_reads!(SelectWhereSet);
locking_reads!(SelectGroupSet);
locking_reads!(SelectHavingSet);
locking_reads!(SelectOrderSet);
locking_reads!(SelectLimitSet);
locking_reads!(SelectOffsetSet);

impl<'db, 'q, Runner, Schema, Strength, T, M, R, G>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectForSet<Strength, Wait>, T, M, R, G>,
        SelectForSet<Strength, Wait>,
    >
{
    pub fn nowait(
        self,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectForSet<Strength, NoWait>, T, M, R, G>,
        SelectForSet<Strength, NoWait>,
    > {
        self.map(|builder| builder.nowait())
    }
    pub fn skip_locked(
        self,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectForSet<Strength, SkipLocked>, T, M, R, G>,
        SelectForSet<Strength, SkipLocked>,
    > {
        self.map(|builder| builder.skip_locked())
    }
}
