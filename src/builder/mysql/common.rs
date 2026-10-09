//! Driver-independent wrappers that attach a MySQL query to an executor.

#![allow(clippy::type_complexity)]

use core::marker::PhantomData;

use drizzle_core::{SQLIndex, SQLTable, ToSQL};
use drizzle_mysql::{
    builder::{
        self, CTEView, CompletedSelect, DeleteBuilder, DeleteInitial, DeleteLimitSet,
        DeleteOrderSet, DeleteWhereSet, ForShare, ForUpdate, InsertBuilder, InsertColumnsSet,
        InsertIgnoreSet, InsertInitial, InsertOnDuplicateKeyUpdateSet, InsertValuesSet,
        IntoSelectQuery, NoWait, QueryBuilder, SelectBuilder, SelectForSet, SelectFromSet,
        SelectGroupSet, SelectHavingSet, SelectIndexHintSet, SelectInitial, SelectJoinSet,
        SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
        SkipLocked, UpdateBuilder, UpdateInitial, UpdateLimitSet, UpdateOrderSet,
        UpdateSetClauseSet, UpdateWhereSet, Wait,
    },
    common::MySQLSchemaType,
    traits::MySQLTable,
    values::MySQLValue,
};

/// A query being built against a MySQL `Drizzle` handle or transaction.
///
/// Start one with `select`, `insert`, `update`, or `delete`, chain clauses,
/// then run it with the adapter's `.execute()`, `.all()`, `.get()`, or
/// `.rows()`. Each clause method is only available where it is valid SQL.
/// The builder implements `ToSQL`, so `.to_sql().sql()` shows the SQL it will
/// run.
#[derive(Debug)]
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleBuilder<'db, Runner, Schema, Builder, State> {
    pub(crate) runner: Runner,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State, &'db ())>,
}

/// A relational query (`db.query(table)`), which loads rows together with
/// their related rows in one SQL statement.
///
/// Add relations with [`with`](Self::with), narrow it with
/// [`r#where`](Self::r#where), [`order_by`](Self::order_by),
/// [`limit`](Self::limit), and [`offset`](Self::offset), then run it with
/// the adapter's `find_many()` or `find_first()`. Each clause can be set once.
#[cfg(feature = "query")]
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleQueryBuilder<
    'db,
    'q,
    Runner,
    Schema,
    Table,
    Relations = (),
    Columns = drizzle_core::query::AllColumns,
    Clauses = drizzle_core::query::Clauses,
> {
    pub(crate) runner: Runner,
    pub(crate) builder:
        drizzle_core::query::QueryBuilder<'q, MySQLValue<'q>, Table, Relations, Columns, Clauses>,
    pub(crate) state: PhantomData<(&'db (), Schema)>,
}

/// A relational query rendered once, made by
/// [`DrizzleQueryBuilder::prepare`].
///
/// It is detached from the connection: the adapter's `find_many` and
/// `find_first` take the connection and the placeholder bindings on each
/// call.
#[cfg(feature = "query")]
#[derive(Debug, Clone)]
pub struct DrizzlePreparedQuery<'q, Driver, Table, Relations, Columns> {
    pub(crate) inner: drizzle_core::prepared::PreparedStatement<'q, MySQLValue<'q>>,
    pub(crate) state: PhantomData<(Driver, Table, Relations, Columns)>,
}

#[cfg(feature = "query")]
impl<Driver, Table, Relations, Columns>
    DrizzlePreparedQuery<'_, Driver, Table, Relations, Columns>
{
    /// Returns the rendered SQL, with `?` placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Returns how many placeholder bindings each run expects.
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }
}

#[cfg(feature = "query")]
impl<Driver, Table, Relations, Columns> core::fmt::Display
    for DrizzlePreparedQuery<'_, Driver, Table, Relations, Columns>
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.sql())
    }
}

#[cfg(feature = "query")]
pub use crate::builder::RelationalPreparedDriver;

#[cfg(feature = "query")]
pub(crate) fn render_relational_all<'q, Table, Relations, Clauses>(
    builder: drizzle_core::query::QueryBuilder<
        'q,
        MySQLValue<'q>,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        Clauses,
    >,
) -> drizzle_core::SQL<'q, MySQLValue<'q>>
where
    Table: drizzle_core::query::QueryTable,
    Relations: drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
{
    let mut relations = Vec::new();
    builder.relations.render_into(&mut relations);
    drizzle_core::query::build_query_sql(
        Table::TABLE,
        Table::COLUMN_NAMES,
        Table::BLOB_COLUMNS,
        Table::JSON_PROJECTIONS,
        relations,
        builder.where_sql,
        builder.order_by_sql,
        builder.limit,
        builder.offset,
        false,
    )
}

#[cfg(feature = "query")]
pub(crate) fn render_relational_partial<'q, Table, Relations, Clauses>(
    builder: drizzle_core::query::QueryBuilder<
        'q,
        MySQLValue<'q>,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        Clauses,
    >,
) -> drizzle_core::SQL<'q, MySQLValue<'q>>
where
    Table: drizzle_core::query::QueryTable,
    Relations: drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
{
    let mut relations = Vec::new();
    builder.relations.render_into(&mut relations);
    let columns = builder.cols.columns;
    drizzle_core::query::build_query_sql(
        Table::TABLE,
        &columns,
        Table::BLOB_COLUMNS,
        Table::JSON_PROJECTIONS,
        relations,
        builder.where_sql,
        builder.order_by_sql,
        builder.limit,
        builder.offset,
        true,
    )
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Clauses>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        Clauses,
    >
where
    Runner: RelationalPreparedDriver,
    Table: drizzle_core::query::QueryTable,
    Relations: drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
{
    /// Renders and detaches a reusable prepared relational query.
    ///
    /// The returned query is not bound to this runner or transaction; pass the
    /// adapter connection when executing it.
    pub fn prepare(
        self,
    ) -> DrizzlePreparedQuery<
        'q,
        Runner::PreparedDriver,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
    > {
        DrizzlePreparedQuery {
            inner: drizzle_core::prepared::prepare_render(&render_relational_all(self.builder)),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Clauses>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        Clauses,
    >
where
    Runner: RelationalPreparedDriver,
    Table: drizzle_core::query::QueryTable,
    Relations: drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
{
    /// Renders and detaches a reusable prepared partial relational query.
    ///
    /// The returned query is not bound to this runner or transaction; pass the
    /// adapter connection when executing it.
    pub fn prepare(
        self,
    ) -> DrizzlePreparedQuery<
        'q,
        Runner::PreparedDriver,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
    > {
        DrizzlePreparedQuery {
            inner: drizzle_core::prepared::prepare_render(&render_relational_partial(self.builder)),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Columns, Clauses>
    DrizzleQueryBuilder<'db, 'q, Runner, Schema, Table, Relations, Columns, Clauses>
{
    /// Loads a relation with each row, such as `users.posts()`.
    ///
    /// Relation handles can nest their own `.with(..)` and clauses. Call `with`
    /// again to load more relations.
    #[allow(clippy::type_complexity)]
    pub fn with<Relation, Cardinality, ChildColumns, RelationClauses>(
        self,
        relation: drizzle_core::query::RelationHandle<
            'q,
            MySQLValue<'q>,
            Relation,
            Cardinality,
            ChildColumns,
            RelationClauses,
        >,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        (
            drizzle_core::query::RelationHandle<
                'q,
                MySQLValue<'q>,
                Relation,
                Cardinality,
                ChildColumns,
                RelationClauses,
            >,
            Relations,
        ),
        Columns,
        Clauses,
    >
    where
        Relation: drizzle_core::relation::RelationDef<Source = Table> + 'static,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.with(relation),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Columns, Order, Limit>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<drizzle_core::query::NoWhere, Order, Limit>,
    >
{
    /// Filters the root rows. Combine conditions with a tuple (`AND`), `and`,
    /// or `or`; this can be called once.
    ///
    /// The condition may only read the queried table's columns.
    pub fn r#where<Expr, ScopeProof>(
        self,
        condition: Expr,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<drizzle_core::query::HasWhere, Order, Limit>,
    >
    where
        Expr: drizzle_core::expr::ExprSources,
        Expr::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        Expr: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
        Expr::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.r#where(condition),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Columns, Where, Limit>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<Where, drizzle_core::query::NoOrderBy, Limit>,
    >
{
    /// Orders the root rows. This can be called once; pass a tuple to order by
    /// several columns.
    pub fn order_by<Expr, ScopeProof>(
        self,
        expression: Expr,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<Where, drizzle_core::query::HasOrderBy, Limit>,
    >
    where
        Expr: drizzle_core::expr::ExprSources,
        Expr::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        Expr: ToSQL<'q, MySQLValue<'q>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.order_by(expression),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Columns, Where, Order>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
{
    /// Returns at most `n` root rows. This can be called once.
    pub fn limit<Arg>(
        self,
        limit: Arg,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::HasLimit>,
    >
    where
        Arg: drizzle_core::PaginationArg<'q, MySQLValue<'q>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.limit(limit),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Columns, Where, Order>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::HasLimit>,
    >
{
    /// Skips the first `n` root rows. Call [`limit`](Self::limit) first.
    pub fn offset<Arg>(
        self,
        offset: Arg,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        Columns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::HasOffset>,
    >
    where
        Arg: drizzle_core::PaginationArg<'q, MySQLValue<'q>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.offset(offset),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Runner, Schema, Table, Relations, Clauses>
    DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        Clauses,
    >
where
    Table: drizzle_core::query::QueryTable,
{
    /// Loads only the listed columns.
    ///
    /// Rows then use the table's `PartialSelect*` model, where every field is an
    /// `Option` and unselected ones are `None`.
    pub fn columns<Selection: drizzle_core::query::IntoColumnSelection>(
        self,
        selection: Selection,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        Clauses,
    > {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.columns(selection),
            state: PhantomData,
        }
    }

    /// Loads every column except the listed ones.
    ///
    /// Rows then use the table's `PartialSelect*` model, where every field is an
    /// `Option` and omitted ones are `None`.
    pub fn omit<Selection: drizzle_core::query::IntoColumnSelection>(
        self,
        selection: Selection,
    ) -> DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        Clauses,
    > {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.omit(selection),
            state: PhantomData,
        }
    }
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

impl<Runner, Schema, Builder: drizzle_core::expr::SelectQuery, State>
    drizzle_core::expr::SelectQuery for DrizzleBuilder<'_, Runner, Schema, Builder, State>
{
}

impl<Runner, Schema, Builder, State> drizzle_core::expr::ExprSources
    for DrizzleBuilder<'_, Runner, Schema, Builder, State>
where
    Builder: drizzle_core::expr::ExprSources,
{
    type Sources = Builder::Sources;
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
    /// Starts the `SELECT` that follows the `WITH` clause.
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

    /// Starts the `SELECT DISTINCT` that follows the `WITH` clause.
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

    /// Starts the `UPDATE` that follows the `WITH` clause.
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

    /// Starts the `DELETE` that follows the `WITH` clause.
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

    /// Adds another common table expression to the `WITH` clause.
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
    /// Sets the table (or other source) the query reads from.
    ///
    /// The source decides the row type for `select(())` and brings its columns
    /// into scope for the rest of the query. It can be a table, a table alias, a
    /// CTE, or a derived table made with `.alias(..)`.
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
            drizzle_core::FromMarker<M, T>,
            <M as drizzle_core::ResolveRow<T>>::Row,
        >,
        SelectFromSet,
    >
    where
        T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::ScopeEntry,
        M: drizzle_core::ResolveRow<T>,
    {
        self.map(|builder| builder.from(table))
    }
}

macro_rules! select_method {
    (where) => {
        /// Adds a `WHERE` condition.
        ///
        /// Combine conditions with a tuple (`AND`), `and`/`or`, or `|`. An `Option`
        /// element of a tuple that is `None` is left out, which makes optional
        /// filters easy. Columns in the condition must come from the query's tables;
        /// this is checked when the query runs.
        pub fn r#where<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectWhereSet,
                T,
                <M as drizzle_core::HasScope>::With<E::Sources>,
                R,
                G,
            >,
            SelectWhereSet,
        >
        where
            M: drizzle_core::HasScope,
            E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            self.map(|builder| builder.r#where(condition))
        }
    };
    (group_by) => {
        /// Adds a `GROUP BY` clause. Pass a column, an expression, or a tuple.
        ///
        /// Each column in a selected tuple must then be grouped or aggregated; this
        /// is checked when the query runs.
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectGroupSet,
                T,
                <M as drizzle_core::HasScope>::With<Gr::Sources>,
                R,
                Gr::Columns,
            >,
            SelectGroupSet,
        >
        where
            M: drizzle_core::HasScope,
            Gr: drizzle_core::IntoGroupBy<'q, MySQLValue<'q>>,
        {
            self.map(|builder| builder.group_by(columns))
        }
    };
    (having) => {
        /// Adds a `HAVING` condition, which filters groups after `GROUP BY`.
        pub fn having<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectHavingSet,
                T,
                <M as drizzle_core::HasScope>::With<E::Sources>,
                R,
                G,
            >,
            SelectHavingSet,
        >
        where
            M: drizzle_core::HasScope,
            E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            self.map(|builder| builder.having(condition))
        }
    };
    (order_by) => {
        /// Adds an `ORDER BY` clause.
        ///
        /// Pass a column (ascending), `asc(..)`/`desc(..)` from the MySQL prelude,
        /// or a tuple of them.
        pub fn order_by<O>(
            self,
            order: O,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectOrderSet,
                T,
                <M as drizzle_core::HasScope>::With<O::Sources>,
                R,
                G,
            >,
            SelectOrderSet,
        >
        where
            M: drizzle_core::HasScope,
            O: ToSQL<'q, MySQLValue<'q>> + drizzle_core::expr::ExprSources,
        {
            self.map(|builder| builder.order_by(order))
        }
    };
    (limit) => {
        /// Returns at most `limit` rows (`LIMIT ?`).
        ///
        /// Integers are sent as bound parameters, so the SQL text stays the same
        /// from page to page. An integer placeholder is bound when the query runs.
        ///
        /// # Panics
        ///
        /// Panics when an integer `limit` is negative or does not fit in `usize`.
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
        /// Skips the first `offset` rows (`OFFSET ?`). MySQL has no bare
        /// `OFFSET`, so without a `LIMIT` this renders
        /// `LIMIT 9223372036854775807 OFFSET ?`.
        ///
        /// # Panics
        ///
        /// Panics when an integer `offset` is negative or does not fit in `usize`.
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
        /// Adds a `JOIN` (an inner join).
        ///
        /// Pass a table to join on its foreign key to the previous table (the
        /// `FROM` table, or the table joined last), or a `(table, condition)` pair
        /// to give the `ON` condition yourself. The joined table's columns come
        /// into scope.
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
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::InnerJoin,
                    J::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::InnerJoin,
                    J::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>,
        {
            self.map(|builder| builder.join(arg))
        }

        /// Adds an `INNER JOIN`. Takes the same arguments as [`join`](Self::join).
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
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::InnerJoin,
                    J::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::InnerJoin,
                    J::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>,
        {
            self.map(|builder| builder.inner_join(arg))
        }

        /// Adds a `CROSS JOIN`: every row paired with every row of `arg`, with no
        /// `ON` condition.
        pub fn cross_join<Arg>(
            self,
            arg: Arg,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                Arg::JoinedTable,
                <M as drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::InnerJoin,
                    Arg::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::InnerJoin,
                    Arg::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            Arg: drizzle_mysql::helpers::CrossJoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>,
        {
            self.map(|builder| builder.cross_join(arg))
        }

        /// Adds `INNER JOIN LATERAL (subquery) AS name ON condition`. The subquery
        /// can read columns of the tables before it. Requires MySQL 8.0.14 or later.
        pub fn inner_join_lateral<Arg>(
            self,
            arg: Arg,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                Arg::JoinedTable,
                <M as drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::InnerJoin>,
                    Arg::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::InnerJoin>,
                    Arg::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            Arg: drizzle_core::LateralArg<'q, MySQLValue<'q>>,
            M: drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::InnerJoin>,
                    Arg::OnSources,
                >,
        {
            self.map(|builder| builder.inner_join_lateral(arg))
        }

        /// Adds `LEFT JOIN LATERAL (subquery) AS name ON condition`, which keeps
        /// rows with no match. The selection must allow the lateral columns to be
        /// missing (`NULL`).
        pub fn left_join_lateral<Arg, SelectionProof>(
            self,
            arg: Arg,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                Arg::JoinedTable,
                <M as drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::LeftJoin>,
                    Arg::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::LeftJoin>,
                    Arg::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            Arg: drizzle_core::LateralArg<'q, MySQLValue<'q>>,
            M: drizzle_core::JoinStep<
                    R,
                    Arg::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::LeftJoin>,
                    Arg::OnSources,
                > + drizzle_core::LeftLateralSelection<SelectionProof>,
        {
            self.map(|builder| builder.left_join_lateral(arg))
        }

        /// Adds `CROSS JOIN LATERAL (subquery) AS name`, with no `ON` condition.
        pub fn cross_join_lateral<Source>(
            self,
            source: Source,
        ) -> DrizzleBuilder<
            'db,
            Runner,
            Schema,
            SelectBuilder<
                'q,
                Schema,
                SelectJoinSet,
                Source::JoinedTable,
                <M as drizzle_core::JoinStep<
                    R,
                    Source::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::InnerJoin>,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    Source::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::InnerJoin>,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            Source: drizzle_core::LateralSource<'q, MySQLValue<'q>>,
            M: drizzle_core::JoinStep<
                    R,
                    Source::JoinedTable,
                    drizzle_core::Lateral<drizzle_core::InnerJoin>,
                >,
        {
            self.map(|builder| builder.cross_join_lateral(source))
        }

        /// Adds a `LEFT JOIN`. Takes the same arguments as [`join`](Self::join);
        /// the joined table's columns decode as `Option<T>`, which is checked when
        /// the query runs.
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
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::LeftJoin,
                    J::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::LeftJoin,
                    J::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::LeftJoin, J::OnSources>,
        {
            self.map(|builder| builder.left_join(arg))
        }

        /// Adds a `LEFT OUTER JOIN`, the same join as
        /// [`left_join`](Self::left_join).
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
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::LeftJoin,
                    J::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::LeftJoin,
                    J::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::LeftJoin, J::OnSources>,
        {
            self.map(|builder| builder.left_outer_join(arg))
        }

        /// Adds a `RIGHT JOIN`. Takes the same arguments as [`join`](Self::join);
        /// the columns of the tables before it decode as `Option<T>`, which is
        /// checked when the query runs.
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
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::RightJoin,
                    J::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::RightJoin,
                    J::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::RightJoin, J::OnSources>,
        {
            self.map(|builder| builder.right_join(arg))
        }

        /// Adds a `RIGHT OUTER JOIN`, the same join as
        /// [`right_join`](Self::right_join).
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
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::RightJoin,
                    J::OnSources,
                >>::Marker,
                <M as drizzle_core::JoinStep<
                    R,
                    J::JoinedTable,
                    drizzle_core::RightJoin,
                    J::OnSources,
                >>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_mysql::helpers::JoinArg<'q, T>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::RightJoin, J::OnSources>,
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
    /// Orders a compound query (`UNION`, `INTERSECT`, ...) by its output
    /// columns.
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
    /// Adds `USE INDEX (index)` to the `FROM` table, which asks MySQL to pick
    /// among only these indexes.
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

    /// Adds `FORCE INDEX (index)` to the `FROM` table, which tells MySQL to
    /// scan the table only when this index cannot be used.
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

    /// Adds `IGNORE INDEX (index)` to the `FROM` table, which keeps MySQL from
    /// using this index.
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

impl<'db, 'q, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    /// Combines this query's rows with `other`'s and drops duplicates
    /// (`UNION`).
    ///
    /// Both queries must select the same row type. Use
    /// [`union_all`](Self::union_all) to keep duplicates.
    #[allow(clippy::type_complexity)]
    pub fn union<O>(
        self,
        other: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >,
        SelectSetOpSet,
    >
    where
        O: IntoSelectQuery<'q, Schema, R>,
        M: drizzle_core::SetOperand<O::Marker>,
    {
        self.map(|builder| builder.union(other))
    }

    /// Combines this query's rows with `other`'s, keeping duplicates
    /// (`UNION ALL`).
    #[allow(clippy::type_complexity)]
    pub fn union_all<O>(
        self,
        other: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >,
        SelectSetOpSet,
    >
    where
        O: IntoSelectQuery<'q, Schema, R>,
        M: drizzle_core::SetOperand<O::Marker>,
    {
        self.map(|builder| builder.union_all(other))
    }

    /// Keeps only rows that `other` also returns (`INTERSECT`). Requires MySQL
    /// 8.0.31 or later.
    #[allow(clippy::type_complexity)]
    pub fn intersect<O>(
        self,
        other: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >,
        SelectSetOpSet,
    >
    where
        O: IntoSelectQuery<'q, Schema, R>,
        M: drizzle_core::SetOperand<O::Marker>,
    {
        self.map(|builder| builder.intersect(other))
    }

    /// Keeps rows that `other` also returns, with duplicates (`INTERSECT ALL`).
    /// Requires MySQL 8.0.31 or later.
    #[allow(clippy::type_complexity)]
    pub fn intersect_all<O>(
        self,
        other: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >,
        SelectSetOpSet,
    >
    where
        O: IntoSelectQuery<'q, Schema, R>,
        M: drizzle_core::SetOperand<O::Marker>,
    {
        self.map(|builder| builder.intersect_all(other))
    }

    /// Keeps only rows that `other` does not return (`EXCEPT`). Requires MySQL
    /// 8.0.31 or later.
    #[allow(clippy::type_complexity)]
    pub fn except<O>(
        self,
        other: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >,
        SelectSetOpSet,
    >
    where
        O: IntoSelectQuery<'q, Schema, R>,
        M: drizzle_core::SetOperand<O::Marker>,
    {
        self.map(|builder| builder.except(other))
    }

    /// Keeps rows that `other` does not return, with duplicates
    /// (`EXCEPT ALL`). Requires MySQL 8.0.31 or later.
    #[allow(clippy::type_complexity)]
    pub fn except_all<O>(
        self,
        other: O,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >,
        SelectSetOpSet,
    >
    where
        O: IntoSelectQuery<'q, Schema, R>,
        M: drizzle_core::SetOperand<O::Marker>,
    {
        self.map(|builder| builder.except_all(other))
    }
}

impl<'q, Runner, Schema, State, T, M, R, G> IntoSelectQuery<'q, Schema, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    SelectBuilder<'q, Schema, State, T, M, R, G>:
        CompletedSelect<'q, Schema, R, Marker = M, Grouped = G>,
{
    type Marker = M;
    type Grouped = G;
    type Select = SelectBuilder<'q, Schema, State, T, M, R, G>;

    fn into_select_query(self) -> Self::Select {
        self.builder
    }
}

impl<Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, QueryBuilder<'_, Schema, State, T, M, R, G>, State>
{
    /// Adds a free-form [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment in front of the query.
    pub fn comment(self, text: impl AsRef<str>) -> Self
    where
        State: builder::ExecutableState,
    {
        self.map(|builder| builder.comment(text))
    }

    /// Adds a key-value [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment, such as `/*route='users'*/`, in front of the query.
    pub fn comment_tags<I, K, V>(self, pairs: I) -> Self
    where
        State: builder::ExecutableState,
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> + builder::ExecutableState,
    T: SQLTable<'q, MySQLSchemaType, MySQLValue<'q>>,
{
    /// Turns this query into a common table expression named after `Tag`.
    ///
    /// Pass the result to the handle's `with` and select from it. Its columns
    /// are reachable as fields, like a table's.
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

impl<'db, 'q, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
{
    /// Names this query so it can be used as a table: `(SELECT ...) AS name`.
    ///
    /// `tag` is a tag type (see the `tag!` macro). `.fields()` on the result
    /// returns its output columns.
    ///
    /// # Panics
    ///
    /// Panics when the projection contains duplicate output names. Name a
    /// computed expression with [`drizzle_core::expr::AliasExt::named`] to
    /// make each output unique.
    #[must_use]
    pub fn alias<Tag, AggProof>(
        self,
        tag: Tag,
    ) -> drizzle_core::Derived<
        'q,
        MySQLValue<'q>,
        Tag,
        <M as drizzle_core::DerivedSelection<'q, MySQLValue<'q>, MySQLSchemaType, T>>::Projection,
        SelectBuilder<'q, Schema, State, T, M, R, G>,
    >
    where
        Tag: drizzle_core::Tag,
        M: drizzle_core::DerivedSelection<'q, MySQLValue<'q>, MySQLSchemaType, T>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
        <M as drizzle_core::DerivedSelection<'q, MySQLValue<'q>, MySQLSchemaType, T>>::Projection:
            drizzle_core::DerivedProjection<Tag>,
    {
        self.builder.alias(tag)
    }
}

macro_rules! insert_sources {
    ($state:ty) => {
        impl<'db, 'q, Runner, Schema, Table>
            DrizzleBuilder<'db, Runner, Schema, InsertBuilder<'q, Schema, $state, Table>, $state>
        where
            Table: MySQLTable<'q>,
        {
            /// Inserts one row from an `Insert*` model.
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

            /// Inserts several rows from `Insert*` models in one statement.
            ///
            /// Every row must set the same optional fields (the same `with_*` calls);
            /// mixing them does not compile, because all rows share one column list.
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

            /// Inserts the rows of a `SELECT` query: `INSERT INTO t SELECT ...`.
            ///
            /// The query's columns must match the table's insert columns in order and
            /// type; this is checked at compile time.
            pub fn select<Q, R, ScopeProof, AggProof>(
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
                Table: drizzle_core::InsertSelectTable,
                Q: IntoSelectQuery<'q, Schema, R>,
                Q::Marker: drizzle_core::InsertSelectCompatible<'q, MySQLValue<'q>, Table, R>
                    + drizzle_core::MarkerScopeValidFor<ScopeProof>
                    + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
            {
                self.map(|builder| builder.select(query))
            }

            /// Inserts the rows of any SQL value, such as a raw `sql!` query, with no
            /// compile-time column checks.
            pub fn select_raw<Q>(
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
                Q: ToSQL<'q, MySQLValue<'q>>,
            {
                self.map(|builder| builder.select_raw(query))
            }

            /// Names the columns an `INSERT ... SELECT` fills, before `select(..)`.
            ///
            /// The list must include every required column (`NOT NULL` with no
            /// default); this is checked by the following `select(..)`.
            ///
            /// # Panics
            ///
            /// Panics when the same target column appears more than once.
            pub fn columns<Columns>(
                self,
                columns: Columns,
            ) -> DrizzleBuilder<
                'db,
                Runner,
                Schema,
                InsertBuilder<'q, Schema, InsertColumnsSet<Columns::Columns>, Table>,
                InsertColumnsSet<Columns::Columns>,
            >
            where
                Columns: drizzle_core::InsertTargetColumns<'q, MySQLValue<'q>, Table>,
            {
                self.map(|builder| builder.columns(columns))
            }
        }
    };
}

insert_sources!(InsertInitial);
insert_sources!(InsertIgnoreSet);

impl<'db, 'q, Runner, Schema, Table, Targets>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        InsertBuilder<'q, Schema, InsertColumnsSet<Targets>, Table>,
        InsertColumnsSet<Targets>,
    >
where
    Table: MySQLTable<'q>,
{
    /// Inserts the rows of a `SELECT` query into the columns named by
    /// `columns(..)`.
    ///
    /// The query's output must match those columns in order and type, and the
    /// columns must include every required one; both are checked at compile
    /// time.
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
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
        Table: drizzle_core::InsertSelectTable,
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: IntoSelectQuery<'q, Schema, R>,
        Q::Marker: drizzle_core::PartialInsertSelectCompatible<'q, MySQLValue<'q>, Targets>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        self.map(|builder| builder.select(query))
    }

    /// Inserts the rows of any SQL value into the columns named by
    /// `columns(..)`, with no compile-time check on the query's output.
    pub fn select_raw<Q, RequiredProof>(
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
        Table: drizzle_core::InsertSelectTable,
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: ToSQL<'q, MySQLValue<'q>>,
    {
        self.map(|builder| builder.select_raw(query))
    }
}

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
    /// Makes this an `INSERT IGNORE`, which skips rows that would duplicate a
    /// primary or unique key instead of failing (MySQL also downgrades some
    /// other errors to warnings).
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
    /// Adds `ON DUPLICATE KEY UPDATE`, which updates the existing row when the
    /// new one would duplicate any primary or unique key.
    ///
    /// `values` is the table's `Update*` model. MySQL picks the conflicting key
    /// itself, so there is no conflict target.
    ///
    /// # Panics
    ///
    /// Panics when `values` sets no column.
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
    /// Sets the columns to change, from an `Update*` model.
    ///
    /// Only the fields set with `with_*` are written.
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

macro_rules! mutation_method {
    ($builder:ident, $state:ty, where => $next:ty) => {
        impl<'db, 'q, Runner, Schema, Table>
            DrizzleBuilder<'db, Runner, Schema, $builder<'q, Schema, $state, Table>, $state>
        {
            /// Changes or deletes only rows matching `condition`.
            pub fn r#where<E, ScopeProof>(
                self,
                condition: E,
            ) -> DrizzleBuilder<'db, Runner, Schema, $builder<'q, Schema, $next, Table>, $next>
            where
                E: drizzle_core::expr::ExprSources,
                E::Sources: drizzle_core::scope::SourcesIn<
                        drizzle_core::Cons<Table, drizzle_core::Nil>,
                        ScopeProof,
                    >,
                E: drizzle_core::expr::Expr<'q, MySQLValue<'q>>,
                E::SQLType: drizzle_core::types::BooleanLike,
            {
                self.map(|builder| builder.r#where(condition))
            }
        }
    };
    ($builder:ident, $state:ty, order_by => $next:ty) => {
        impl<'db, 'q, Runner, Schema, Table>
            DrizzleBuilder<'db, Runner, Schema, $builder<'q, Schema, $state, Table>, $state>
        {
            /// Adds an `ORDER BY`, which decides which rows a following `.limit(..)`
            /// reaches.
            pub fn order_by<O, ScopeProof>(
                self,
                order: O,
            ) -> DrizzleBuilder<'db, Runner, Schema, $builder<'q, Schema, $next, Table>, $next>
            where
                O: drizzle_core::expr::ExprSources,
                O::Sources: drizzle_core::scope::SourcesIn<
                        drizzle_core::Cons<Table, drizzle_core::Nil>,
                        ScopeProof,
                    >,
                O: ToSQL<'q, MySQLValue<'q>>,
            {
                self.map(|builder| builder.order_by(order))
            }
        }
    };
    ($builder:ident, $state:ty, limit => $next:ty) => {
        impl<'db, 'q, Runner, Schema, Table>
            DrizzleBuilder<'db, Runner, Schema, $builder<'q, Schema, $state, Table>, $state>
        {
            /// Changes or deletes at most `limit` rows (`LIMIT ?`).
            ///
            /// # Panics
            ///
            /// Panics when an integer `limit` is negative or does not fit in `usize`.
            pub fn limit<P>(
                self,
                limit: P,
            ) -> DrizzleBuilder<'db, Runner, Schema, $builder<'q, Schema, $next, Table>, $next>
            where
                P: drizzle_core::PaginationArg<'q, MySQLValue<'q>>,
            {
                self.map(|builder| builder.limit(limit))
            }
        }
    };
}

mutation_method!(UpdateBuilder, UpdateSetClauseSet, where => UpdateWhereSet);
mutation_method!(UpdateBuilder, UpdateWhereSet, order_by => UpdateOrderSet);
mutation_method!(UpdateBuilder, UpdateWhereSet, limit => UpdateLimitSet);
mutation_method!(UpdateBuilder, UpdateOrderSet, limit => UpdateLimitSet);

mutation_method!(DeleteBuilder, DeleteInitial, where => DeleteWhereSet);
mutation_method!(DeleteBuilder, DeleteWhereSet, order_by => DeleteOrderSet);
mutation_method!(DeleteBuilder, DeleteWhereSet, limit => DeleteLimitSet);
mutation_method!(DeleteBuilder, DeleteOrderSet, limit => DeleteLimitSet);

impl<'db, 'q, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'db, Runner, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
{
    /// Locks the selected rows against changes by other transactions until
    /// this one ends (`FOR UPDATE`).
    pub fn for_update(
        self,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectForSet<ForUpdate>, T, M, R, G>,
        SelectForSet<ForUpdate>,
    > {
        self.map(|builder| builder.for_update())
    }

    /// Takes shared locks on the selected rows: other transactions can read
    /// them but not change them (`FOR SHARE`).
    pub fn for_share(
        self,
    ) -> DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectForSet<ForShare>, T, M, R, G>,
        SelectForSet<ForShare>,
    > {
        self.map(|builder| builder.for_share())
    }
}

impl<'db, 'q, Runner, Schema, Strength, T, M, R, G>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        SelectBuilder<'q, Schema, SelectForSet<Strength, Wait>, T, M, R, G>,
        SelectForSet<Strength, Wait>,
    >
{
    /// Fails right away instead of waiting when a row is already locked
    /// (`NOWAIT`).
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
    /// Leaves out rows that are already locked instead of waiting for them
    /// (`SKIP LOCKED`).
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
