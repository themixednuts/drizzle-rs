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
pub struct DrizzleBuilder<'a, Runner, Schema, Builder, State> {
    pub(crate) runner: Runner,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State, &'a ())>,
}

/// Shared Postgres query builder wrapper (relational query API).
#[cfg(feature = "query")]
pub struct DrizzleQueryBuilder<
    'db,
    'a,
    Runner,
    Schema,
    T,
    Rels = (),
    Cols = drizzle_core::query::AllColumns,
    Cl = drizzle_core::query::Clauses,
> {
    pub(crate) runner: Runner,
    pub(crate) builder: drizzle_core::query::QueryBuilder<'a, PostgresValue<'a>, T, Rels, Cols, Cl>,
    pub(crate) _schema: PhantomData<(&'db (), Schema)>,
}

/// Prepared relational query.
///
/// Created by [`DrizzleQueryBuilder::prepare`]. The prepared query is detached
/// from the connection; driver modules provide `find_many` and `find_first`
/// methods that take an explicit client plus parameter bindings.
#[cfg(feature = "query")]
#[derive(Debug, Clone)]
pub struct DrizzlePreparedQuery<'a, Driver, T, Rels, Cols> {
    pub(crate) inner: drizzle_core::prepared::PreparedStatement<'a, PostgresValue<'a>>,
    pub(crate) _marker: PhantomData<(Driver, T, Rels, Cols)>,
}

#[cfg(feature = "query")]
impl<'a, Driver, T, Rels, Cols> DrizzlePreparedQuery<'a, Driver, T, Rels, Cols> {
    /// Returns the prepared SQL string with dialect placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Returns the number of external parameter bindings expected.
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }
}

#[cfg(feature = "query")]
impl<Driver, T, Rels, Cols> core::fmt::Display for DrizzlePreparedQuery<'_, Driver, T, Rels, Cols> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.sql())
    }
}

/// Maps a relational query runner to the detached prepared-query driver marker.
#[cfg(feature = "query")]
pub trait RelationalPreparedDriver {
    type PreparedDriver;
}

#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, Cols, Cl>
{
    /// Includes a relation in the query results.
    #[allow(clippy::type_complexity)]
    pub fn with<R, N, C, RCl>(
        self,
        handle: drizzle_core::query::RelationHandle<'a, PostgresValue<'a>, R, N, C, RCl>,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        (
            drizzle_core::query::RelationHandle<'a, PostgresValue<'a>, R, N, C, RCl>,
            Rels,
        ),
        Cols,
        Cl,
    >
    where
        R: drizzle_core::relation::RelationDef<Source = T> + 'static,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.with(handle),
            _schema: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, drizzle_core::query::AllColumns, Cl>
where
    T: drizzle_core::query::QueryTable,
    Rels: drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
    Runner: RelationalPreparedDriver,
{
    /// Creates a prepared relational query.
    pub fn prepare(
        self,
    ) -> DrizzlePreparedQuery<
        'a,
        <Runner as RelationalPreparedDriver>::PreparedDriver,
        T,
        Rels,
        drizzle_core::query::AllColumns,
    > {
        let builder = self.builder;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE_NAME,
            T::COLUMN_NAMES,
            T::BLOB_COLUMNS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            false,
        );
        DrizzlePreparedQuery {
            inner: drizzle_core::prepared::prepare_render(&query_sql),
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, drizzle_core::query::PartialColumns, Cl>
where
    T: drizzle_core::query::QueryTable,
    Rels: drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
    Runner: RelationalPreparedDriver,
{
    /// Creates a prepared relational query.
    pub fn prepare(
        self,
    ) -> DrizzlePreparedQuery<
        'a,
        <Runner as RelationalPreparedDriver>::PreparedDriver,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
    > {
        let builder = self.builder;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = builder.cols.columns;
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE_NAME,
            &col_refs,
            T::BLOB_COLUMNS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            true,
        );
        DrizzlePreparedQuery {
            inner: drizzle_core::prepared::prepare_render(&query_sql),
            _marker: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, Ord, Lim>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder: self.builder.r#where(condition),
            _schema: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, W, Lim>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder: self.builder.order_by(expr),
            _schema: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, W, Ord>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Sets a LIMIT on the query. Can only be called once.
    pub fn limit<P>(
        self,
        n: P,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasLimit>,
    >
    where
        P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.limit(n),
            _schema: PhantomData,
        }
    }
}

/// OFFSET requires LIMIT to have been set first.
#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, W, Ord>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasLimit>,
    >
{
    /// Sets an OFFSET on the query. Requires `.limit()` first.
    pub fn offset<P>(
        self,
        n: P,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasOffset>,
    >
    where
        P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.offset(n),
            _schema: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, drizzle_core::query::AllColumns, Cl>
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
        Runner,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    > {
        DrizzleQueryBuilder {
            runner: self.runner,
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
        Runner,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    > {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.omit(selector),
            _schema: PhantomData,
        }
    }
}

/// Intermediate builder for typed ON CONFLICT within a `PostgreSQL` Drizzle wrapper.
pub struct DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
    runner: Runner,
    builder: OnConflictBuilder<'b, Schema, Table>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, 'b, Runner, Schema, Table> DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
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
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
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
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.do_update(set),
            state: PhantomData,
        }
    }
}

impl<'a, Runner, S, T, State> ToSQL<'a, PostgresValue<'a>>
    for DrizzleBuilder<'_, Runner, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}

impl<'a, Runner, S, T, State> drizzle_core::expr::Expr<'a, PostgresValue<'a>>
    for DrizzleBuilder<'_, Runner, S, T, State>
where
    T: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
{
    type SQLType = T::SQLType;
    type Nullable = T::Nullable;
    type Aggregate = T::Aggregate;
}

impl<'d, 'a, Runner, Schema>
    DrizzleBuilder<'d, Runner, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), T::Marker>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
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
        Runner,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), T::Marker>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select_distinct(query);
        DrizzleBuilder {
            runner: self.runner,
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
        Runner,
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
            runner: self.runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Runner, Schema, M>
    DrizzleBuilder<
        'd,
        Runner,
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
        Runner,
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
            runner: self.runner,
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
            impl<'d, 'a, Runner, Schema, T, M, R, G>
                DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, $state, T, M, R, G>, $state>
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
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T, M, R, G>, SelectWhereSet>
        where
            E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.r#where(condition);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method group_by) => {
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, M, R, Gr::Columns>, SelectGroupSet>
        where
            Gr: drizzle_core::IntoGroupBy<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.group_by(columns);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method having) => {
        pub fn having<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, M, R, G>, SelectGroupSet>
        where
            E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.having(condition);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method order_by) => {
        pub fn order_by<TOrderBy>(
            self,
            expressions: TOrderBy,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T, M, R, G>, SelectOrderSet>
        where
            TOrderBy: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.order_by(expressions);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method limit) => {
        pub fn limit<P>(
            self,
            limit: P,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T, M, R, G>, SelectLimitSet>
        where
            P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.limit(limit);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method offset) => {
        pub fn offset<P>(
            self,
            offset: P,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectOffsetSet, T, M, R, G>, SelectOffsetSet>
        where
            P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.offset(offset);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method join) => {
        #[inline]
        pub fn join<J: drizzle_postgres::helpers::JoinArg<'a, T>>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'd,
            Runner,
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
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
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

impl<'a, Runner, Schema, State, T, M, R, G> IntoSelect<'a, Schema, M, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
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

impl<Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, QueryBuilder<'_, Schema, State, T, M, R, G>, State>
where
    State: drizzle_postgres::builder::ExecutableState,
{
    /// Attaches a free-form [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment to the query. See [`QueryBuilder::comment`] for details.
    #[inline]
    pub fn comment(self, text: impl AsRef<str>) -> Self {
        DrizzleBuilder {
            runner: self.runner,
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
            runner: self.runner,
            builder: self.builder.comment_tags(pairs),
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Set operations on DrizzleBuilder
//------------------------------------------------------------------------------

impl<'d, 'a, Runner, Schema, State, T, M, R>
    DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R>, State>
where
    State: drizzle_postgres::builder::ExecutableState,
{
    pub fn union(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.union(other),
            state: PhantomData,
        }
    }

    pub fn union_all(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.union_all(other),
            state: PhantomData,
        }
    }

    pub fn intersect(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.intersect(other),
            state: PhantomData,
        }
    }

    pub fn intersect_all(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.intersect_all(other),
            state: PhantomData,
        }
    }

    pub fn except(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.except(other),
            state: PhantomData,
        }
    }

    pub fn except_all(
        self,
        other: impl IntoSelect<'a, Schema, M, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectSetOpSet, T, M, R>,
        SelectSetOpSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.except_all(other),
            state: PhantomData,
        }
    }
}

impl<'a, Runner, Schema, State, T, M, R>
    DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R>, State>
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

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn select<Q>(
        self,
        query: Q,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'b>,
        Q: ToSQL<'b, PostgresValue<'b>>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
    ) -> DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
        DrizzleOnConflictBuilder {
            runner: self.runner,
            builder: self.builder.on_conflict(target),
            _phantom: PhantomData,
        }
    }

    /// Begins a typed ON CONFLICT ON CONSTRAINT clause (PostgreSQL-only).
    pub fn on_conflict_on_constraint<C: NamedConstraint<Table>>(
        self,
        target: C,
    ) -> DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
        DrizzleOnConflictBuilder {
            runner: self.runner,
            builder: self.builder.on_conflict_on_constraint(target),
            _phantom: PhantomData,
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleBuilder {
            runner: self.runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table>,
        UpdateFromSet,
    > {
        let builder = self.builder.from(source.to_sql());
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    pub fn r#where<E>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
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
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
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
        Runner,
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
            runner: self.runner,
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
            impl<'d, 'a, Runner, Schema, T, M, R>
                DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, $state, T, M, R>, $state>
            {
                /// Adds FOR UPDATE clause to lock selected rows for update.
                pub fn for_update(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_update();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Adds FOR SHARE clause to lock selected rows for shared access.
                pub fn for_share(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_share();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Adds FOR NO KEY UPDATE clause.
                pub fn for_no_key_update(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_no_key_update();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Adds FOR KEY SHARE clause.
                pub fn for_key_share(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_key_share();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Adds FOR UPDATE OF table clause to lock only rows from a specific table.
                pub fn for_update_of<U: PostgresTable<'a>>(self, table: U) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_update_of(table);
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Adds FOR SHARE OF table clause to lock only rows from a specific table.
                pub fn for_share_of<U: PostgresTable<'a>>(self, table: U) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_share_of(table);
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
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
impl<Runner, Schema, T, M, R>
    DrizzleBuilder<
        '_,
        Runner,
        Schema,
        SelectBuilder<'_, Schema, SelectForSet, T, M, R>,
        SelectForSet,
    >
{
    /// Adds NOWAIT option to fail immediately if rows are locked.
    pub fn nowait(self) -> Self {
        let builder = self.builder.nowait();
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Adds SKIP LOCKED option to skip over locked rows.
    pub fn skip_locked(self) -> Self {
        let builder = self.builder.skip_locked();
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}
