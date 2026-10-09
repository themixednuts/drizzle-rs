#![allow(clippy::type_complexity)]

use core::marker::PhantomData;

use crate::drizzle_pg_builder_join_impl;
use crate::drizzle_pg_builder_join_using_impl;
use drizzle_core::traits::{SQLModel, SQLTable, ToSQL};
use drizzle_core::{ConflictTarget, NamedConstraint};
use drizzle_postgres::builder::{
    self, CTEView, DeleteInitial, DeleteReturningSet, DeleteWhereSet, InsertColumnsSet,
    InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
    OnConflictBuilder, QueryBuilder, SelectForSet, SelectFromSet, SelectGroupSet, SelectInitial,
    SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectWhereSet, UpdateFromSet,
    UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
    delete::DeleteBuilder,
    insert::InsertBuilder,
    select::{CompletedSelect, IntoSelect, IntoSelectQuery, SelectBuilder, SelectSetOpSet},
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresSchemaType;
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;

/// A query being built against a PostgreSQL `Drizzle` handle.
///
/// Start one with `select`, `insert`, `update`, or `delete` on the handle,
/// chain clauses, then run it with the driver's `.execute()`, `.all()`,
/// `.get()`, or `.rows()`. Each clause method is only available where it is
/// valid SQL (for example, `.having()` only after `.group_by()`). The builder
/// implements `ToSQL`, so `.to_sql().sql()` shows the SQL it will run.
#[derive(Debug)]
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleBuilder<'a, Runner, Schema, Builder, State> {
    pub(crate) runner: Runner,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State, &'a ())>,
}

/// A relational query (`db.query(table)`), which loads rows together with
/// their related rows in one SQL statement.
///
/// Add relations with [`with`](Self::with), narrow it with
/// [`r#where`](Self::r#where), [`order_by`](Self::order_by),
/// [`limit`](Self::limit), and [`offset`](Self::offset), then run it with
/// the driver's `find_many()` or `find_first()`. Each clause can be set once.
#[cfg(feature = "query")]
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
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

/// A relational query rendered once, made by
/// [`DrizzleQueryBuilder::prepare`].
///
/// It is detached from the connection: the driver's `find_many` and
/// `find_first` take the client and the placeholder bindings on each call.
#[cfg(feature = "query")]
#[derive(Debug, Clone)]
pub struct DrizzlePreparedQuery<'a, Driver, T, Rels, Cols> {
    pub(crate) inner: drizzle_core::prepared::PreparedStatement<'a, PostgresValue<'a>>,
    pub(crate) _marker: PhantomData<(Driver, T, Rels, Cols)>,
}

#[cfg(feature = "query")]
impl<'a, Driver, T, Rels, Cols> DrizzlePreparedQuery<'a, Driver, T, Rels, Cols> {
    /// Returns the rendered SQL, with `$n` placeholders.
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
impl<Driver, T, Rels, Cols> core::fmt::Display for DrizzlePreparedQuery<'_, Driver, T, Rels, Cols> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.sql())
    }
}

#[cfg(feature = "query")]
pub use crate::builder::RelationalPreparedDriver;

#[cfg(feature = "query")]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, Cols, Cl>
{
    /// Loads a relation with each row, such as `users.posts()`.
    ///
    /// Relation handles can nest their own `.with(..)` and clauses. Call `with`
    /// again to load more relations.
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
    /// Renders this relational query once into a reusable
    /// [`DrizzlePreparedQuery`].
    ///
    /// Put column placeholders in the clauses, then run it with the driver's
    /// prepared `find_many`/`find_first`, binding each placeholder by name.
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
            T::TABLE,
            T::COLUMN_NAMES,
            T::BLOB_COLUMNS,
            T::JSON_PROJECTIONS,
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
    /// Renders this partial-column relational query once into a reusable
    /// [`DrizzlePreparedQuery`].
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
            T::TABLE,
            &col_refs,
            T::BLOB_COLUMNS,
            T::JSON_PROJECTIONS,
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
    /// Filters the root rows. Combine conditions with a tuple (`AND`), `and`,
    /// or `or`; this can be called once.
    ///
    /// The condition may only read the queried table's columns.
    pub fn r#where<E, ScopeProof>(
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
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
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
    /// Orders the root rows. This can be called once; pass a tuple to order by
    /// several columns.
    pub fn order_by<E, ScopeProof>(
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
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
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
    /// Returns at most `n` root rows. This can be called once.
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
    /// Skips the first `n` root rows. Call [`limit`](Self::limit) first.
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
    /// Loads only the listed columns.
    ///
    /// Rows then use the table's `PartialSelect*` model, where every field is an
    /// `Option` and unselected ones are `None`.
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

    /// Loads every column except the listed ones.
    ///
    /// Rows then use the table's `PartialSelect*` model, where every field is an
    /// `Option` and omitted ones are `None`.
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

/// The `ON CONFLICT` step of an insert, made by `.on_conflict(target)` or
/// `.on_conflict_on_constraint(name)`.
///
/// Finish it with [`do_nothing`](Self::do_nothing) or
/// [`do_update`](Self::do_update).
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
    runner: Runner,
    builder: OnConflictBuilder<'b, Schema, Table>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, 'b, Runner, Schema, Table> DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
    /// Restricts the conflict target to rows matching `condition`, to match a
    /// partial unique index: `ON CONFLICT (cols) WHERE condition`.
    pub fn r#where<E, ScopeProof>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// Skips rows that conflict on the target: `ON CONFLICT (target) DO NOTHING`.
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

    /// Updates the existing row instead: `ON CONFLICT (target) DO UPDATE SET ...`.
    ///
    /// `set` is usually an `Update*` model. Chain `.r#where(..)` to update only
    /// some conflicting rows.
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

impl<Runner, S, T: drizzle_core::expr::SelectQuery, State> drizzle_core::expr::SelectQuery
    for DrizzleBuilder<'_, Runner, S, T, State>
{
}

impl<Runner, S, T, State> drizzle_core::expr::ExprSources
    for DrizzleBuilder<'_, Runner, S, T, State>
where
    T: drizzle_core::expr::ExprSources,
{
    type Sources = T::Sources;
}

impl<'d, 'a, Runner, Schema>
    DrizzleBuilder<'d, Runner, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    /// Starts the `SELECT` that follows the `WITH` clause.
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

    /// Starts the `SELECT DISTINCT` that follows the `WITH` clause.
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

    /// Starts the `SELECT DISTINCT ON (on) ...` that follows the `WITH` clause.
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

    /// Adds another common table expression to the `WITH` clause.
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
    /// Sets the table (or other source) the query reads from.
    ///
    /// The source decides the row type for `select(())` and brings its columns
    /// into scope for the rest of the query. It can be a table, a table alias, a
    /// CTE, or a derived table made with `.alias(..)`.
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
            drizzle_core::FromMarker<M, T>,
            <M as drizzle_core::ResolveRow<T>>::Row,
        >,
        SelectFromSet,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::ScopeEntry,
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
        /// Adds a `WHERE` condition.
        ///
        /// Combine conditions with a tuple (`AND`), `and`/`or`, or `|`. An `Option`
        /// element of a tuple that is `None` is left out, which makes optional
        /// filters easy. Columns in the condition must come from the query's tables;
        /// this is checked when the query runs.
        #[inline]
        pub fn r#where<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T, <M as drizzle_core::HasScope>::With<E::Sources>, R, G>, SelectWhereSet>
        where
            M: drizzle_core::HasScope,
            E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.r#where(condition);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method group_by) => {
        /// Adds a `GROUP BY` clause. Pass a column, an expression, or a tuple.
        ///
        /// Each column in a selected tuple must then be grouped or aggregated; this
        /// is checked when the query runs.
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, <M as drizzle_core::HasScope>::With<Gr::Sources>, R, Gr::Columns>, SelectGroupSet>
        where
            M: drizzle_core::HasScope,
            Gr: drizzle_core::IntoGroupBy<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.group_by(columns);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method having) => {
        /// Adds a `HAVING` condition, which filters groups after `GROUP BY`.
        pub fn having<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, <M as drizzle_core::HasScope>::With<E::Sources>, R, G>, SelectGroupSet>
        where
            M: drizzle_core::HasScope,
            E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.having(condition);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method order_by) => {
        /// Adds an `ORDER BY` clause.
        ///
        /// Pass a column (ascending), [`asc`](drizzle_core::asc)/[`desc`](drizzle_core::desc),
        /// or a tuple of them.
        pub fn order_by<TOrderBy>(
            self,
            expressions: TOrderBy,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T, <M as drizzle_core::HasScope>::With<TOrderBy::Sources>, R, G>, SelectOrderSet>
        where
            M: drizzle_core::HasScope,
            TOrderBy: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>> + drizzle_core::expr::ExprSources,
        {
            let builder = self.builder.order_by(expressions);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method set_order_by) => {
        /// Orders a compound query (`UNION`, `INTERSECT`, ...) by its output
        /// columns.
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
        /// Returns at most `limit` rows (`LIMIT $n`).
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
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T, M, R, G>, SelectLimitSet>
        where
            P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
        {
            let builder = self.builder.limit(limit);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method offset) => {
        /// Skips the first `offset` rows (`OFFSET $n`).
        ///
        /// # Panics
        ///
        /// Panics when an integer `offset` is negative or does not fit in `usize`.
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
        /// Adds a `JOIN` (an inner join).
        ///
        /// Pass a table to join on its foreign key to the previous table (the
        /// `FROM` table, or the table joined last), or a `(table, condition)` pair
        /// to give the `ON` condition yourself. The joined table's columns come
        /// into scope. See also `left_join`, `right_join`, `full_join`, their
        /// `natural_`, `_outer`, and `_using` forms, and the lateral joins.
        #[inline]
        pub fn join<J: drizzle_postgres::helpers::JoinArg<'a, T, Via>, Via>(
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
                <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Marker,
                <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>,
        {
            let builder = self.builder.join(arg);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }

        crate::drizzle_pg_builder_join_impl!();
        crate::drizzle_pg_builder_join_using_impl!();

        /// Adds a `CROSS JOIN`: every row paired with every row of `arg`, with no
        /// `ON` condition.
        #[inline]
        pub fn cross_join<Arg: drizzle_postgres::helpers::CrossJoinArg<'a, T>>(
            self,
            arg: Arg,
        ) -> DrizzleBuilder<
            'd,
            Runner,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                Arg::JoinedTable,
                <M as drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>>::Marker,
                <M as drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            M: drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>,
        {
            let builder = self.builder.cross_join(arg);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
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
    SelectSetOpSet => [set_order_by, limit, offset],
}

//------------------------------------------------------------------------------
// IntoSelect for DrizzleBuilder
//------------------------------------------------------------------------------

impl<'a, Runner, Schema, State, T, M, R, G> IntoSelect<'a, Schema, M, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
    SelectBuilder<'a, Schema, State, T, M, R, G>:
        CompletedSelect<'a, Schema, R, Marker = M, Grouped = G>,
{
    type State = State;
    type Table = T;
    fn into_select(self) -> SelectBuilder<'a, Schema, State, T, M, R> {
        self.builder.into_select()
    }
}

impl<'a, Runner, Schema, State, T, M, R, G> IntoSelectQuery<'a, Schema, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    SelectBuilder<'a, Schema, State, T, M, R, G>:
        CompletedSelect<'a, Schema, R, Marker = M, Grouped = G>,
{
    type Marker = M;
    type Grouped = G;
    type Select = SelectBuilder<'a, Schema, State, T, M, R, G>;

    fn into_select_query(self) -> Self::Select {
        self.builder
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
{
    /// Adds a free-form [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment in front of the query. See [`QueryBuilder::comment`] for how the
    /// text is escaped.
    #[inline]
    pub fn comment(self, text: impl AsRef<str>) -> Self
    where
        State: drizzle_postgres::builder::ExecutableState,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.comment(text),
            state: PhantomData,
        }
    }

    /// Adds a key-value [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment, such as `/*route='users'*/`, in front of the query. See
    /// [`QueryBuilder::comment_tags`] for the encoding.
    #[inline]
    pub fn comment_tags<I, K, V>(self, pairs: I) -> Self
    where
        State: drizzle_postgres::builder::ExecutableState,
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    /// Combines this query's rows with `other`'s and drops duplicates
    /// (`UNION`).
    ///
    /// Both queries must select the same row type. Use
    /// [`union_all`](Self::union_all) to keep duplicates.
    #[allow(clippy::type_complexity)]
    pub fn union<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.union(other),
            state: PhantomData,
        }
    }

    /// Combines this query's rows with `other`'s, keeping duplicates
    /// (`UNION ALL`).
    #[allow(clippy::type_complexity)]
    pub fn union_all<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.union_all(other),
            state: PhantomData,
        }
    }

    /// Keeps only rows that `other` also returns (`INTERSECT`).
    #[allow(clippy::type_complexity)]
    pub fn intersect<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.intersect(other),
            state: PhantomData,
        }
    }

    /// Keeps rows that `other` also returns, with duplicates (`INTERSECT ALL`).
    #[allow(clippy::type_complexity)]
    pub fn intersect_all<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.intersect_all(other),
            state: PhantomData,
        }
    }

    /// Keeps only rows that `other` does not return (`EXCEPT`).
    #[allow(clippy::type_complexity)]
    pub fn except<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.except(other),
            state: PhantomData,
        }
    }

    /// Keeps rows that `other` does not return, with duplicates (`EXCEPT ALL`).
    #[allow(clippy::type_complexity)]
    pub fn except_all<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
    T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    /// Turns this query into a common table expression named after `Tag`.
    ///
    /// Pass the result to the handle's `with` and select from it. Its columns
    /// are reachable as fields, like a table's.
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

impl<'a, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
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
    #[inline]
    #[must_use]
    pub fn alias<Tag, AggProof>(
        self,
        tag: Tag,
    ) -> drizzle_core::Derived<
        'a,
        PostgresValue<'a>,
        Tag,
        <M as drizzle_core::DerivedSelection<
            'a,
            PostgresValue<'a>,
            PostgresSchemaType,
            T,
        >>::Projection,
        SelectBuilder<'a, Schema, State, T, M, R, G>,
    >
    where
        Tag: drizzle_core::Tag,
        M: drizzle_core::DerivedSelection<'a, PostgresValue<'a>, PostgresSchemaType, T>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
        <M as drizzle_core::DerivedSelection<
            'a,
            PostgresValue<'a>,
            PostgresSchemaType,
            T,
        >>::Projection: drizzle_core::DerivedProjection<Tag>,
{
        self.builder.alias(tag)
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
    /// Inserts one row from an `Insert*` model.
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

    /// Inserts several rows from `Insert*` models in one statement.
    ///
    /// Every row must set the same optional fields (the same `with_*` calls);
    /// mixing them does not compile, because all rows share one column list.
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

    /// Names the columns an `INSERT ... SELECT` fills, before
    /// [`select`](Self::select).
    ///
    /// The list must include every required column (`NOT NULL` with no
    /// default); this is checked by the following `select(..)`.
    #[inline]
    pub fn columns<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertColumnsSet<Columns::Columns>, Table>,
        InsertColumnsSet<Columns::Columns>,
    >
    where
        Table: PostgresTable<'b>,
        Columns: drizzle_core::InsertTargetColumns<'b, PostgresValue<'b>, Table>,
    {
        let builder = self.builder.columns(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Inserts the rows of a `SELECT` query: `INSERT INTO t SELECT ...`.
    ///
    /// The query's columns must match the table's insert columns in order and
    /// type; this is checked at compile time.
    #[inline]
    pub fn select<Q, R, ScopeProof, AggProof>(
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
        Table: PostgresTable<'b> + drizzle_core::InsertSelectTable,
        Q: IntoSelectQuery<'b, Schema, R>,
        Q::Marker: drizzle_core::InsertSelectCompatible<'b, PostgresValue<'b>, Table, R>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Inserts the rows of any SQL value, such as a raw `sql!` query, with no
    /// compile-time column checks.
    #[inline]
    pub fn select_raw<Q>(
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
        let builder = self.builder.select_raw(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table, Targets>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertColumnsSet<Targets>, Table>,
        InsertColumnsSet<Targets>,
    >
where
    Table: PostgresTable<'b> + drizzle_core::InsertSelectTable,
{
    /// Inserts the rows of a `SELECT` query into the columns named by
    /// [`columns`](Self::columns).
    ///
    /// The query's output must match those columns in order and type; this is
    /// checked at compile time.
    #[inline]
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
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
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: IntoSelectQuery<'b, Schema, R>,
        Q::Marker: drizzle_core::PartialInsertSelectCompatible<'b, PostgresValue<'b>, Targets>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Inserts the rows of any SQL value into the columns named by
    /// [`columns`](Self::columns), with no compile-time check on the query's
    /// output.
    #[inline]
    pub fn select_raw<Q, RequiredProof>(
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
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: ToSQL<'b, PostgresValue<'b>>,
    {
        let builder = self.builder.select_raw(query);
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
    /// Starts an `ON CONFLICT (target)` clause; finish it with
    /// [`do_nothing`](DrizzleOnConflictBuilder::do_nothing) or
    /// [`do_update`](DrizzleOnConflictBuilder::do_update).
    ///
    /// `target` is a column or tuple of columns covered by a primary key or
    /// unique constraint.
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

    /// Starts an `ON CONFLICT ON CONSTRAINT name` clause, naming a primary
    /// key or unique constraint of the table.
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

    /// Skips any row that would violate a constraint: `ON CONFLICT DO NOTHING`.
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

    /// Returns columns of the inserted rows: `RETURNING ...`.
    ///
    /// Run it with `.all()` or `.get()` to read them.
    pub fn returning<Columns, ScopeProof>(
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
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
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
    /// Returns columns of the inserted rows: `RETURNING ...`.
    pub fn returning<Columns, ScopeProof>(
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
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
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
    /// Updates only conflicting rows that match `condition`:
    /// `DO UPDATE SET ... WHERE condition`.
    pub fn r#where<E, ScopeProof>(
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
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'b, PostgresValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.r#where(condition),
            state: PhantomData,
        }
    }

    /// Returns columns of the inserted or updated rows: `RETURNING ...`.
    pub fn returning<Columns, ScopeProof>(
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
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
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
    /// Sets the columns to change, from an `Update*` model.
    ///
    /// Only the fields set with `with_*` are written.
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
    /// Adds a `FROM` source to the update (`UPDATE t SET ... FROM source`), so
    /// the `WHERE` condition can read its columns.
    pub fn from<F>(
        self,
        source: F,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table, drizzle_core::Cons<F, drizzle_core::Nil>>,
        UpdateFromSet,
    >
    where
        F: ToSQL<'b, PostgresValue<'b>> + drizzle_core::ScopeEntry,
    {
        let builder = self.builder.from(source);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Updates only rows matching `condition`.
    pub fn r#where<E, ScopeProof>(
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
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
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
}

impl<'a, 'b, Runner, Schema, Table, M>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table, M>,
        UpdateFromSet,
    >
{
    /// Updates only rows matching `condition`, which may read the `FROM`
    /// source's columns.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table, M>,
        UpdateWhereSet,
    >
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, M>, ScopeProof>,
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
}

impl<'a, 'b, Runner, Schema, Table, M>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table, M>,
        UpdateWhereSet,
    >
{
    /// Returns columns of the updated rows: `RETURNING ...`.
    pub fn returning<Columns, ScopeProof>(
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
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, M>, ScopeProof>,
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
    /// Deletes only rows matching `condition`.
    pub fn r#where<E, ScopeProof>(
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
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
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
    /// Returns columns of the deleted rows: `RETURNING ...`.
    pub fn returning<Columns, ScopeProof>(
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
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
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
                /// Locks the selected rows against updates and deletes by other
                /// transactions until this one ends (`FOR UPDATE`).
                pub fn for_update(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_update();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Locks the selected rows against changes, while letting other
                /// transactions take shared locks too (`FOR SHARE`).
                pub fn for_share(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_share();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Like [`for_update`](Self::for_update), but does not block
                /// `FOR KEY SHARE` locks (`FOR NO KEY UPDATE`).
                pub fn for_no_key_update(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_no_key_update();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Blocks only changes to the selected rows' keys
                /// (`FOR KEY SHARE`).
                pub fn for_key_share(self) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_key_share();
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Like [`for_update`](Self::for_update), but locks only the rows
                /// of `table` (`FOR UPDATE OF table`).
                pub fn for_update_of<U: PostgresTable<'a>>(self, table: U) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectForSet, T, M, R>, SelectForSet> {
                    let builder = self.builder.for_update_of(table);
                    DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
                }

                /// Like [`for_share`](Self::for_share), but locks only the rows of
                /// `table` (`FOR SHARE OF table`).
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
    /// Fails right away instead of waiting when a row is already locked
    /// (`NOWAIT`).
    pub fn nowait(self) -> Self {
        let builder = self.builder.nowait();
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Leaves out rows that are already locked instead of waiting for them
    /// (`SKIP LOCKED`).
    pub fn skip_locked(self) -> Self {
        let builder = self.builder.skip_locked();
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}
