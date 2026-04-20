#![allow(clippy::type_complexity)]

use core::marker::PhantomData;

use crate::drizzle_pg_builder_join_impl;
use crate::drizzle_pg_builder_join_using_impl;
use drizzle_core::traits::{SQLModel, SQLTable, ToSQL};
use drizzle_core::{ConflictTarget, NamedConstraint};
use drizzle_postgres::builder::{
    self, CTEView, DeleteInitial, DeleteReturningSet, DeleteWhereSet, InsertDoUpdateSet,
    InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet, OnConflictBuilder,
    QueryBuilder, SelectForSet, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet,
    SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectWhereSet, UpdateFromSet, UpdateInitial,
    UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
    delete::DeleteBuilder,
    insert::InsertBuilder,
    select::{AsCteState, IntoSelect, SelectBuilder, SelectSetOpSet},
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresSchemaType;
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;

/// Shared Postgres drizzle builder wrapper.
#[derive(Debug)]
pub struct DrizzleBuilder<'a, DrizzleRef, Schema, Builder, State> {
    pub(crate) drizzle: DrizzleRef,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State, &'a ())>,
}

/// Shared Postgres query builder wrapper (relational query API).
#[cfg(feature = "query")]
pub struct DrizzleQueryBuilder<
    'db,
    'a,
    DrizzleRef,
    Schema,
    T,
    Rels = (),
    Cols = drizzle_core::query::AllColumns,
    Cl = drizzle_core::query::Clauses,
> {
    pub(crate) drizzle: DrizzleRef,
    pub(crate) builder: drizzle_core::query::QueryBuilder<PostgresValue<'a>, T, Rels, Cols, Cl>,
    pub(crate) _schema: PhantomData<(&'db (), Schema)>,
}

#[cfg(feature = "query")]
impl<'db, 'a, DrizzleRef, Schema, T, Rels, Cols, Cl>
    DrizzleQueryBuilder<'db, 'a, DrizzleRef, Schema, T, Rels, Cols, Cl>
{
    /// Includes a relation in the query results.
    #[allow(clippy::type_complexity)]
    pub fn with<R, N, C, RCl>(
        self,
        handle: drizzle_core::query::RelationHandle<PostgresValue<'a>, R, N, C, RCl>,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        (
            drizzle_core::query::RelationHandle<PostgresValue<'a>, R, N, C, RCl>,
            Rels,
        ),
        Cols,
        Cl,
    >
    where
        R: drizzle_core::relation::RelationDef<Source = T> + 'static,
    {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.with(handle),
            _schema: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
#[cfg(feature = "query")]
impl<'db, 'a, DrizzleRef, Schema, T, Rels, Cols, Ord, Lim>
    DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<drizzle_core::query::NoWhere, Ord, Lim>,
    >
{
    /// Sets the WHERE clause for the query.
    ///
    /// Can only be called once. To combine conditions, use `and(a, b)` or `or(a, b)`.
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<drizzle_core::query::HasWhere, Ord, Lim>,
    >
    where
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.r#where(condition),
            _schema: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
#[cfg(feature = "query")]
impl<'db, 'a, DrizzleRef, Schema, T, Rels, Cols, W, Lim>
    DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, drizzle_core::query::NoOrderBy, Lim>,
    >
{
    /// Adds a typed ORDER BY clause. Can only be called once.
    pub fn order_by<E>(
        self,
        expr: E,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, drizzle_core::query::HasOrderBy, Lim>,
    >
    where
        E: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
    {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.order_by(expr),
            _schema: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
#[cfg(feature = "query")]
impl<'db, 'a, DrizzleRef, Schema, T, Rels, Cols, W, Ord>
    DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Sets a LIMIT on the query. Can only be called once.
    pub fn limit(
        self,
        n: u32,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasLimit>,
    > {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.limit(n),
            _schema: PhantomData,
        }
    }
}

/// OFFSET requires LIMIT to have been set first.
#[cfg(feature = "query")]
impl<'db, 'a, DrizzleRef, Schema, T, Rels, Cols, W, Ord>
    DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasLimit>,
    >
{
    /// Sets an OFFSET on the query. Requires `.limit()` first.
    pub fn offset(
        self,
        n: u32,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasOffset>,
    > {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.offset(n),
            _schema: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'a, DrizzleRef, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, DrizzleRef, Schema, T, Rels, drizzle_core::query::AllColumns, Cl>
where
    T: drizzle_core::query::QueryTable,
{
    /// Selects only the specified columns (include list).
    pub fn columns<S: drizzle_core::query::IntoColumnSelection>(
        self,
        selector: S,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    > {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.columns(selector),
            _schema: PhantomData,
        }
    }

    /// Selects all columns except the specified ones (exclude list).
    pub fn omit<S: drizzle_core::query::IntoColumnSelection>(
        self,
        selector: S,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        DrizzleRef,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    > {
        DrizzleQueryBuilder {
            drizzle: self.drizzle,
            builder: self.builder.omit(selector),
            _schema: PhantomData,
        }
    }
}

/// Intermediate builder for typed ON CONFLICT within a `PostgreSQL` Drizzle wrapper.
pub struct DrizzleOnConflictBuilder<'a, 'b, DrizzleRef, Schema, Table> {
    drizzle: DrizzleRef,
    builder: OnConflictBuilder<'b, Schema, Table>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleOnConflictBuilder<'a, 'b, DrizzleRef, Schema, Table>
{
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// `ON CONFLICT (cols) DO NOTHING`
    pub fn do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.do_nothing(),
            state: PhantomData,
        }
    }

    /// `ON CONFLICT (cols) DO UPDATE SET ...`
    pub fn do_update(
        self,
        set: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.do_update(set),
            state: PhantomData,
        }
    }
}

impl<'a, DrizzleRef, S, T, State> ToSQL<'a, PostgresValue<'a>>
    for DrizzleBuilder<'_, DrizzleRef, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}

impl<'a, DrizzleRef, S, T, State> drizzle_core::expr::Expr<'a, PostgresValue<'a>>
    for DrizzleBuilder<'_, DrizzleRef, S, T, State>
where
    T: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
{
    type SQLType = T::SQLType;
    type Nullable = T::Nullable;
    type Aggregate = T::Aggregate;
}

impl<'d, 'a, DrizzleRef, Schema>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), T::Marker>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn select_distinct<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), T::Marker>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select_distinct(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn select_distinct_on<On, Columns>(
        self,
        on: On,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), Columns::Marker>,
        builder::select::SelectInitial,
    >
    where
        On: ToSQL<'a, PostgresValue<'a>>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select_distinct_on(on, columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(self, cte: &C) -> Self
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, DrizzleRef, Schema, M>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectInitial, (), M>,
        SelectInitial,
    >
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
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
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

/// Generates select-method impl blocks for each given state type, avoiding E0592
/// overlap with insert/update/delete impls that share method names on the same
/// generic `DrizzleBuilder` type.
macro_rules! impl_select_methods {
    ($($state:ty => [$($method:ident),* $(,)?]),+ $(,)?) => {
        $(
            impl<'d, 'a, DrizzleRef, Schema, T, M, R, G>
                DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, $state, T, M, R, G>, $state>
            {
                $( impl_select_methods!(@method $method); )*
            }
        )+
    };

    // ---- individual method expansions ----

    (@method r#where) => {
        #[inline]
        pub fn r#where<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T, M, R, G>, SelectWhereSet>
        where
            E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.r#where(condition);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }
    };

    (@method group_by) => {
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, M, R, Gr::Columns>, SelectGroupSet>
        where
            Gr: drizzle_core::IntoGroupBy<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.group_by(columns);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }
    };

    (@method having) => {
        pub fn having<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, M, R, G>, SelectGroupSet>
        where
            E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.having(condition);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }
    };

    (@method order_by) => {
        pub fn order_by<TOrderBy>(
            self,
            expressions: TOrderBy,
        ) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T, M, R, G>, SelectOrderSet>
        where
            TOrderBy: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.order_by(expressions);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }
    };

    (@method limit) => {
        pub fn limit(
            self,
            limit: usize,
        ) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T, M, R, G>, SelectLimitSet>
        {
            let builder = self.builder.limit(limit);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }
    };

    (@method offset) => {
        pub fn offset(
            self,
            offset: usize,
        ) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectOffsetSet, T, M, R, G>, SelectOffsetSet>
        {
            let builder = self.builder.offset(offset);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }
    };

    (@method join) => {
        #[inline]
        pub fn join<J: drizzle_postgres::helpers::JoinArg<'a, T>>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'd,
            DrizzleRef,
            Schema,
            SelectBuilder<
                'a,
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
            M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
        {
            let builder = self.builder.join(arg);
            DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
        }

        crate::drizzle_pg_builder_join_impl!();
        crate::drizzle_pg_builder_join_using_impl!();
    };
}

// Select method availability by state, mirroring capability trait impls:
impl_select_methods! {
    SelectFromSet  => [r#where, group_by, order_by, limit, offset, join],
    SelectJoinSet  => [r#where, group_by, order_by, join],
    SelectWhereSet => [group_by, order_by, limit],
    SelectGroupSet => [having, order_by, limit],
    SelectOrderSet => [limit],
    SelectLimitSet => [offset],
    SelectSetOpSet => [order_by, limit, offset],
}

//------------------------------------------------------------------------------
// IntoSelect for DrizzleBuilder
//------------------------------------------------------------------------------

impl<'a, DrizzleRef, Schema, State, T, M, R, G> IntoSelect<'a, Schema, M, R>
    for DrizzleBuilder<'_, DrizzleRef, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: drizzle_postgres::builder::ExecutableState,
{
    type State = State;
    type Table = T;
    fn into_select(self) -> SelectBuilder<'a, Schema, State, T, M, R> {
        self.builder.into_select()
    }
}

//------------------------------------------------------------------------------
// sqlcommenter .comment() / .comment_tags() on DrizzleBuilder
//------------------------------------------------------------------------------
//
// Forwards to the inner `QueryBuilder::comment` / `comment_tags`. Because every
// select/insert/update/delete builder is a type alias for `QueryBuilder`, one
// generic impl here covers all four operation kinds.

impl<DrizzleRef, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, DrizzleRef, Schema, QueryBuilder<'_, Schema, State, T, M, R, G>, State>
where
    State: drizzle_postgres::builder::ExecutableState,
{
    /// Attaches a free-form [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment to the query. See [`QueryBuilder::comment`] for details.
    #[inline]
    pub fn comment(self, text: impl AsRef<str>) -> Self {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.comment(text),
            state: PhantomData,
        }
    }

    /// Attaches a tag-style [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment to the query. See [`QueryBuilder::comment_tags`] for details.
    #[inline]
    pub fn comment_tags<I, K, V>(self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.comment_tags(pairs),
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Set operations on DrizzleBuilder
//------------------------------------------------------------------------------

impl<'d, 'a, DrizzleRef, Schema, State, T, M, R>
    DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, State, T, M, R>, State>
where
    State: drizzle_postgres::builder::ExecutableState,
{
    pub fn union(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.union(other),
            state: PhantomData,
        }
    }

    pub fn union_all(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.union_all(other),
            state: PhantomData,
        }
    }

    pub fn intersect(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.intersect(other),
            state: PhantomData,
        }
    }

    pub fn intersect_all(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.intersect_all(other),
            state: PhantomData,
        }
    }

    pub fn except(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.except(other),
            state: PhantomData,
        }
    }

    pub fn except_all(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.except_all(other),
            state: PhantomData,
        }
    }
}

impl<'a, DrizzleRef, Schema, State, T, M, R>
    DrizzleBuilder<'_, DrizzleRef, Schema, SelectBuilder<'a, Schema, State, T, M, R>, State>
where
    State: AsCteState,
    T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    /// Converts this SELECT query into a typed CTE using alias tag name.
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased<Tag>,
        SelectBuilder<'a, Schema, State, T, M, R>,
    > {
        self.builder.into_cte::<Tag>()
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertInitial, Table>,
        InsertInitial,
    >
{
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'b>,
        Table::Insert<T>: SQLModel<'b, PostgresValue<'b>>,
    {
        self.values([value])
    }

    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'b>,
        Table::Insert<T>: SQLModel<'b, PostgresValue<'b>>,
    {
        let builder = self.builder.values(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: PostgresTable<'b>,
{
    /// Begins a typed ON CONFLICT clause targeting specific columns.
    pub fn on_conflict<C: ConflictTarget<Table>>(
        self,
        target: C,
    ) -> DrizzleOnConflictBuilder<'a, 'b, DrizzleRef, Schema, Table> {
        DrizzleOnConflictBuilder {
            drizzle: self.drizzle,
            builder: self.builder.on_conflict(target),
            _phantom: PhantomData,
        }
    }

    /// Begins a typed ON CONFLICT ON CONSTRAINT clause (PostgreSQL-only).
    pub fn on_conflict_on_constraint<C: NamedConstraint<Table>>(
        self,
        target: C,
    ) -> DrizzleOnConflictBuilder<'a, 'b, DrizzleRef, Schema, Table> {
        DrizzleOnConflictBuilder {
            drizzle: self.drizzle,
            builder: self.builder.on_conflict_on_constraint(target),
            _phantom: PhantomData,
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.on_conflict_do_nothing(),
            state: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<
            'b,
            Schema,
            InsertReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        InsertReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<
            'b,
            Schema,
            InsertReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        InsertReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    >
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.r#where(condition),
            state: PhantomData,
        }
    }

    /// Adds RETURNING clause after DO UPDATE SET
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<
            'b,
            Schema,
            InsertReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        InsertReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: PostgresTable<'b>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn from(
        self,
        source: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table>,
        UpdateFromSet,
    > {
        let builder = self.builder.from(source.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn r#where<E>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<
            'b,
            Schema,
            UpdateReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        UpdateReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table>,
        UpdateFromSet,
    >
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<
            'b,
            Schema,
            UpdateReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        UpdateReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<
            'b,
            Schema,
            UpdateReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        UpdateReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteInitial, Table>,
        DeleteInitial,
    >
where
    Table: PostgresTable<'b>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteWhereSet, Table>,
        DeleteWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<
            'b,
            Schema,
            DeleteReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        DeleteReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteWhereSet, Table>,
        DeleteWhereSet,
    >
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<
            'b,
            Schema,
            DeleteReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        DeleteReturningSet,
    >
    where
        Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// FOR UPDATE/SHARE Row Locking (PostgreSQL-specific)
//------------------------------------------------------------------------------

macro_rules! impl_for_update_methods {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<'d, 'a, DrizzleRef, Schema, T, M, R>
                DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, $state, T, M, R>, $state>
            {
                /// Adds FOR UPDATE clause to lock selected rows for update.
                pub fn for_update(self) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_update();
                    DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
                }

                /// Adds FOR SHARE clause to lock selected rows for shared access.
                pub fn for_share(self) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_share();
                    DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
                }

                /// Adds FOR NO KEY UPDATE clause.
                pub fn for_no_key_update(self) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_no_key_update();
                    DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
                }

                /// Adds FOR KEY SHARE clause.
                pub fn for_key_share(self) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_key_share();
                    DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
                }

                /// Adds FOR UPDATE OF table clause to lock only rows from a specific table.
                pub fn for_update_of<U: PostgresTable<'a>>(self, table: U) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_update_of(table);
                    DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
                }

                /// Adds FOR SHARE OF table clause to lock only rows from a specific table.
                pub fn for_share_of<U: PostgresTable<'a>>(self, table: U) -> DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_share_of(table);
                    DrizzleBuilder { drizzle: self.drizzle, builder, state: PhantomData }
                }
            }
        )+
    };
}

impl_for_update_methods!(
    SelectFromSet,
    SelectWhereSet,
    SelectOrderSet,
    SelectLimitSet,
    SelectOffsetSet,
    SelectJoinSet,
    SelectGroupSet,
);

// Implement NOWAIT and SKIP LOCKED on SelectForSet
impl<DrizzleRef, Schema, T, M, R>
    DrizzleBuilder<
        '_,
        DrizzleRef,
        Schema,
        SelectBuilder<'_, Schema, SelectForSet, T, M, R>,
        SelectForSet,
    >
{
    /// Adds NOWAIT option to fail immediately if rows are locked.
    pub fn nowait(self) -> Self {
        let builder = self.builder.nowait();
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    /// Adds SKIP LOCKED option to skip over locked rows.
    pub fn skip_locked(self) -> Self {
        let builder = self.builder.skip_locked();
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
