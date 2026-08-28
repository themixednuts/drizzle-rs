//! Tokio MySQL adapter using the [`mysql_async`] crate.
//!
//! [`Drizzle`] accepts either an owned [`mysql_async::Conn`] or a lazy
//! [`mysql_async::Pool`]. A connection-backed wrapper keeps exclusive mutable
//! ownership visible. A pool-backed wrapper checks out one connection per
//! operation and exposes [`Drizzle::disconnect`] for graceful shutdown.
//!
//! Pools are bound by `mysql_async` to the first Tokio runtime that checks out
//! a connection. Create one pool per independent runtime and call
//! `disconnect().await` before that runtime stops.

mod migration;
pub mod prepared;

use drizzle_core::{
    error::{DrizzleError, QueryContext, Result, ResultExt},
    row::{
        DecodeSelectedRef, FromDrizzleRow, MarkerAggValidFor, MarkerColumnCountValid,
        MarkerScopeValidFor, StrictDecodeMarker,
    },
    traits::ToSQL,
};
use drizzle_mysql::{
    MySQLMutationResult, MySQLRow,
    builder::{
        self, DeleteBuilder, DeleteInitial, InsertBuilder, InsertInitial, QueryBuilder,
        SelectBuilder, SelectInitial, UpdateBuilder, UpdateInitial,
    },
    traits::MySQLTable,
    values::MySQLValue,
};
use mysql_async::{Conn, Pool, Row, Value, prelude::Queryable};

use crate::{
    builder::mysql::{
        common,
        driver_common::{QueryOutput, positional, render},
        introspect,
    },
    transaction::mysql::mysql_async::{Transaction, options},
};

/// A MySQL query attached to an async connection, pool, or transaction.
pub type DrizzleBuilder<'db, Runner, Schema, Builder, State> =
    common::DrizzleBuilder<'db, Runner, Schema, Builder, State>;

/// Decoded MySQL rows from a fully materialized query result.
///
/// The iterator owns its result rows and does not retain a connection or
/// transaction borrow while callers decode them.
pub type Rows<R> = crate::builder::mysql::driver_common::Rows<R>;

pub(crate) fn driver_error(error: mysql_async::Error) -> DrizzleError {
    DrizzleError::driver("MySQL", error)
}

pub(crate) async fn execute_request_observing<C: Queryable + ?Sized>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    mut observe_error: impl FnMut(&mysql_async::Error),
) -> Result<MySQLMutationResult> {
    drizzle_core::drizzle_trace_query!(sql, values.len());
    let context_values = values.iter().collect::<Vec<_>>();
    let params = positional(values.iter().cloned().map(Value::from));
    let result = connection
        .exec_iter(sql, params)
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    let mutation = MySQLMutationResult::new(result.affected_rows(), result.last_insert_id());
    result
        .drop_result()
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    Ok(mutation)
}

pub(crate) async fn execute_request<C: Queryable + ?Sized>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
) -> Result<MySQLMutationResult> {
    execute_request_observing(connection, sql, values, |_| {}).await
}

pub(crate) async fn initialize_session(connection: &mut (impl Queryable + ?Sized)) -> Result<()> {
    initialize_session_observing(connection, |_| {}).await
}

pub(crate) async fn initialize_session_observing(
    connection: &mut (impl Queryable + ?Sized),
    observe_error: impl FnMut(&mysql_async::Error),
) -> Result<()> {
    let sql = "SET time_zone = '+00:00', sql_mode = REPLACE(REPLACE(@@SESSION.sql_mode, 'NO_UNSIGNED_SUBTRACTION', ''), 'REAL_AS_FLOAT', '')";
    execute_request_observing(connection, sql, &[], observe_error)
        .await
        .map(|_| ())
}

pub(crate) async fn query_request_observing<C: Queryable + ?Sized>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    mut observe_error: impl FnMut(&mysql_async::Error),
) -> Result<Vec<Row>> {
    drizzle_core::drizzle_trace_query!(sql, values.len());
    let context_values = values.iter().collect::<Vec<_>>();
    let params = positional(values.iter().cloned().map(Value::from));
    connection
        .exec_iter(sql, params)
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?
        .collect_and_drop::<Row>()
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))
}

pub(crate) async fn query_request<C: Queryable + ?Sized>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
) -> Result<Vec<Row>> {
    query_request_observing(connection, sql, values, |_| {}).await
}

pub(crate) async fn query_first_request_observing<C: Queryable + ?Sized>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    mut observe_error: impl FnMut(&mysql_async::Error),
) -> Result<Option<Row>> {
    drizzle_core::drizzle_trace_query!(sql, values.len());
    let context_values = values.iter().collect::<Vec<_>>();
    let params = positional(values.iter().cloned().map(Value::from));
    let mut result = connection
        .exec_iter(sql, params)
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    let row = result
        .next()
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    result
        .drop_result()
        .await
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    Ok(row)
}

pub(crate) async fn query_first_request<C: Queryable + ?Sized>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
) -> Result<Option<Row>> {
    query_first_request_observing(connection, sql, values, |_| {}).await
}

async fn catalog_query<C, T>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    decode: impl FnOnce(Vec<Row>) -> Result<T>,
) -> Result<T>
where
    C: Queryable + ?Sized,
{
    let context_values = values.iter().collect::<Vec<_>>();
    decode(query_request(connection, sql, values).await?)
        .with_query(|| QueryContext::new(sql, &context_values))
}

async fn catalog(connection: &mut (impl Queryable + ?Sized)) -> Result<introspect::Catalog> {
    use drizzle_migrations::mysql::introspect::{
        RawIntrospection, apply_show_create_view, queries,
    };

    let database = catalog_query(connection, queries::DATABASE, &[], introspect::database).await?;
    let selected_database = database.name.clone();
    let values = [MySQLValue::from(selected_database.as_str())];
    let mut raw = RawIntrospection {
        database,
        tables: catalog_query(connection, queries::TABLES, &values, introspect::tables).await?,
        columns: catalog_query(connection, queries::COLUMNS, &values, introspect::columns).await?,
        indexes: catalog_query(connection, queries::INDEXES, &values, introspect::indexes).await?,
        primary_keys: catalog_query(
            connection,
            queries::PRIMARY_KEYS,
            &values,
            introspect::primary_keys,
        )
        .await?,
        foreign_keys: catalog_query(
            connection,
            queries::FOREIGN_KEYS,
            &values,
            introspect::foreign_keys,
        )
        .await?,
        checks: catalog_query(connection, queries::CHECKS, &values, introspect::checks).await?,
        views: catalog_query(connection, queries::VIEWS, &values, introspect::views).await?,
    };

    for view in &mut raw.views {
        let sql = introspect::view_sql(&view.database, &view.name);
        let statement = catalog_query(connection, &sql, &[], |rows| {
            introspect::view_statement(rows, &view.name)
        })
        .await?;
        apply_show_create_view(view, &statement);
    }

    introspect::Catalog::assemble(raw)
}

async fn apply(
    connection: &mut (impl Queryable + ?Sized),
    schema: &impl drizzle_migrations::Schema,
) -> Result<()> {
    let catalog = catalog(connection).await?;
    let desired = schema.to_snapshot();
    for statement in catalog.plan(&desired)?.statements {
        if !statement.trim().is_empty() {
            execute_request(connection, &statement, &[]).await?;
        }
    }
    Ok(())
}

/// Async MySQL database wrapper over an exact upstream connection or pool.
pub struct Drizzle<Connection, Schema = ()> {
    connection: Connection,
    schema: Schema,
    session_ready: bool,
}

impl<Connection, Schema> core::fmt::Debug for Drizzle<Connection, Schema>
where
    Connection: core::fmt::Debug,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Drizzle")
            .field("connection", &self.connection)
            .field("session_ready", &self.session_ready)
            .finish_non_exhaustive()
    }
}

impl<Connection> Drizzle<Connection> {
    /// Attaches a schema without performing I/O.
    #[must_use]
    pub fn new<Schema: Copy>(
        connection: Connection,
        schema: Schema,
    ) -> (Drizzle<Connection, Schema>, Schema) {
        (
            Drizzle {
                connection,
                schema,
                session_ready: false,
            },
            schema,
        )
    }
}

impl<Connection, Schema> Drizzle<Connection, Schema> {
    /// Borrows the exact upstream connection or pool supplied at construction.
    #[must_use]
    pub const fn conn(&self) -> &Connection {
        &self.connection
    }

    /// Mutably borrows the exact upstream resource supplied at construction.
    pub fn conn_mut(&mut self) -> &mut Connection {
        self.session_ready = false;
        &mut self.connection
    }

    /// Borrows the attached schema.
    #[must_use]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Consumes the wrapper and returns the upstream resource.
    #[must_use]
    pub fn into_inner(self) -> Connection {
        self.connection
    }
}

impl<Schema> Drizzle<Conn, Schema> {
    async fn ensure_session(&mut self) -> Result<()> {
        if !self.session_ready {
            initialize_session(&mut self.connection).await?;
            self.session_ready = true;
        }
        Ok(())
    }

    /// Introspects the selected MySQL database.
    ///
    /// # Errors
    ///
    /// Returns DrizzleError when no database is selected, a catalog query
    /// fails, or MySQL reports metadata that cannot be represented losslessly.
    pub async fn introspect(&mut self) -> Result<drizzle_migrations::schema::Snapshot> {
        self.ensure_session().await?;
        catalog(&mut self.connection)
            .await
            .map(introspect::Catalog::into_snapshot)
    }

    /// Brings the selected MySQL database in sync with the desired schema.
    ///
    /// MySQL can implicitly commit DDL. If a statement fails, earlier
    /// statements from this push may already be committed.
    ///
    /// # Errors
    ///
    /// Returns DrizzleError if introspection, planning, or applying a generated
    /// statement fails.
    pub async fn push<S: drizzle_migrations::Schema>(&mut self, schema: &S) -> Result<()> {
        self.ensure_session().await?;
        apply(&mut self.connection, schema).await
    }

    pub(crate) async fn execute_rendered<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        self.ensure_session().await?;
        let (sql, values) = render(query);
        execute_request(&mut self.connection, &sql, &values).await
    }

    pub(crate) async fn query_rendered<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session().await?;
        let (sql, values) = render(query);
        let rows = query_request(&mut self.connection, &sql, &values).await?;
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub(crate) async fn query_first_rendered<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session().await?;
        let (sql, values) = render(query);
        let rows = query_first_request(&mut self.connection, &sql, &values)
            .await?
            .into_iter()
            .collect();
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub async fn execute<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        let result = self.execute_rendered(query).await;
        self.session_ready = false;
        result
    }

    pub async fn all<'q, R>(&mut self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<Vec<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.rows::<_, R>(query).await?.collect()
    }

    /// Executes typed MySQL SQL and returns a decoded iterator over its
    /// materialized rows.
    ///
    /// The database result is fully consumed before this method returns.
    pub async fn rows<'q, T, R>(&mut self, query: T) -> Result<Rows<R>>
    where
        T: ToSQL<'q, MySQLValue<'q>>,
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self.query_rendered(query).await?.rows::<R>())
    }

    pub async fn get<'q, R>(&mut self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query).await?.decode_first_row()
    }

    /// Applies pending embedded migrations in MySQL autocommit mode.
    ///
    /// Each migration is marked dirty before its first statement and marked
    /// complete only after its last statement succeeds. MySQL DDL cannot be
    /// made migration-wide atomic, so an interrupted run must be reconciled
    /// manually before this method will continue.
    ///
    /// The connection must have autocommit enabled. Finish transactions begun
    /// through raw SQL or the raw driver before calling this method because
    /// the client does not expose their protocol transaction state.
    pub async fn migrate(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> Result<drizzle_migrations::MigrateOutcome> {
        let result = migration::Runner::new(&mut self.connection, migrations, tracking)
            .run()
            .await;
        self.session_ready = false;
        result
    }

    #[cfg(feature = "query")]
    pub fn query<'db, 'q, Table>(
        &'db mut self,
        _table: Table,
    ) -> common::DrizzleQueryBuilder<'db, 'q, &'db mut Self, Schema, Table>
    where
        Table: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            state: core::marker::PhantomData,
        }
    }

    mysql_builder_constructors!(&'db mut Drizzle<Conn, Schema>, [&'db mut self], self);
}

impl<Schema> Drizzle<Pool, Schema> {
    async fn checkout(&self) -> Result<Conn> {
        let mut connection = self.connection.get_conn().await.map_err(driver_error)?;
        initialize_session(&mut connection).await?;
        Ok(connection)
    }

    /// Introspects the selected MySQL database on one checked-out connection.
    ///
    /// # Errors
    ///
    /// Returns DrizzleError when checkout or catalog introspection fails.
    pub async fn introspect(&self) -> Result<drizzle_migrations::schema::Snapshot> {
        let mut connection = self.checkout().await?;
        catalog(&mut connection)
            .await
            .map(introspect::Catalog::into_snapshot)
    }

    /// Brings the selected MySQL database in sync on one checked-out connection.
    ///
    /// MySQL can implicitly commit DDL. If a statement fails, earlier
    /// statements from this push may already be committed.
    ///
    /// # Errors
    ///
    /// Returns DrizzleError if checkout, introspection, planning, or applying a
    /// generated statement fails.
    pub async fn push<S: drizzle_migrations::Schema>(&self, schema: &S) -> Result<()> {
        let mut connection = self.checkout().await?;
        apply(&mut connection, schema).await
    }

    pub(crate) async fn execute_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        let (sql, values) = render(query);
        execute_request(&mut self.checkout().await?, &sql, &values).await
    }

    pub(crate) async fn query_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        let (sql, values) = render(query);
        let rows = query_request(&mut self.checkout().await?, &sql, &values).await?;
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub(crate) async fn query_first_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        let (sql, values) = render(query);
        let rows = query_first_request(&mut self.checkout().await?, &sql, &values)
            .await?
            .into_iter()
            .collect();
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub async fn execute<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        self.execute_rendered(query).await
    }

    pub async fn all<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<Vec<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.rows::<_, R>(query).await?.collect()
    }

    /// Executes typed MySQL SQL and returns a decoded iterator over its
    /// materialized rows.
    ///
    /// The database result is fully consumed before this method returns.
    pub async fn rows<'q, T, R>(&self, query: T) -> Result<Rows<R>>
    where
        T: ToSQL<'q, MySQLValue<'q>>,
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self.query_rendered(query).await?.rows::<R>())
    }

    pub async fn get<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query).await?.decode_first_row()
    }

    /// Applies pending embedded migrations in MySQL autocommit mode.
    ///
    /// This holds one pool checkout for the advisory lock's entire lifetime.
    /// Interrupted migrations remain dirty and require manual reconciliation
    /// before this method will continue.
    ///
    /// The checked-out connection must have autocommit enabled and must not be
    /// inside a transaction begun through raw SQL or the raw driver.
    pub async fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> Result<drizzle_migrations::MigrateOutcome> {
        let mut connection = self.connection.get_conn().await.map_err(driver_error)?;
        migration::Runner::new(&mut connection, migrations, tracking)
            .run()
            .await
    }

    #[cfg(feature = "query")]
    pub fn query<'db, 'q, Table>(
        &'db self,
        _table: Table,
    ) -> common::DrizzleQueryBuilder<'db, 'q, &'db Self, Schema, Table>
    where
        Table: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            state: core::marker::PhantomData,
        }
    }

    /// Gracefully closes the pool after all checked-out connections return.
    pub async fn disconnect(self) -> Result<()> {
        self.connection.disconnect().await.map_err(driver_error)
    }

    mysql_builder_constructors!(&'db Drizzle<Pool, Schema>, [&'db self], self);
}

#[cfg(feature = "query")]
#[doc(hidden)]
pub struct RelationalPrepared;

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &mut Drizzle<Conn, Schema> {
    type PreparedDriver = RelationalPrepared;
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Drizzle<Pool, Schema> {
    type PreparedDriver = RelationalPrepared;
}

pub(crate) trait AsyncRunner {
    async fn execute_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult>;

    async fn query_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>>;

    async fn query_first_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>>;
}

impl<Schema> AsyncRunner for &mut Drizzle<Conn, Schema> {
    async fn execute_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        Drizzle::<Conn, Schema>::execute_rendered(self, query).await
    }

    async fn query_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        Drizzle::<Conn, Schema>::query_rendered(self, query).await
    }

    async fn query_first_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        Drizzle::<Conn, Schema>::query_first_rendered(self, query).await
    }
}

impl<Schema> AsyncRunner for &Drizzle<Pool, Schema> {
    async fn execute_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        Drizzle::<Pool, Schema>::execute_rendered(self, query).await
    }

    async fn query_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        Drizzle::<Pool, Schema>::query_rendered(self, query).await
    }

    async fn query_first_rendered<'q>(
        self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        Drizzle::<Pool, Schema>::query_first_rendered(self, query).await
    }
}

#[cfg(feature = "query")]
#[allow(private_bounds)]
impl<'db, 'q, Runner, Schema, Table, Relations, Clauses>
    common::DrizzleQueryBuilder<
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
    Runner: AsyncRunner,
{
    pub async fn find_many(
        self,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        let query = common::render_relational_all(self.builder);
        self.runner
            .query_rendered(query)
            .await?
            .decode_relational_all::<Table, Relations>()
    }
}

#[cfg(feature = "query")]
#[allow(private_bounds)]
impl<'db, 'q, Runner, Schema, Table, Relations, Where, Order>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
where
    Runner: AsyncRunner,
{
    pub async fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
#[allow(private_bounds)]
impl<'db, 'q, Runner, Schema, Table, Relations, Clauses>
    common::DrizzleQueryBuilder<
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
    Runner: AsyncRunner,
{
    pub async fn find_many(
        self,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        let query = common::render_relational_partial(self.builder);
        self.runner
            .query_rendered(query)
            .await?
            .decode_relational_partial::<Table, Relations>()
    }
}

#[cfg(feature = "query")]
#[allow(private_bounds)]
impl<'db, 'q, Runner, Schema, Table, Relations, Where, Order>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        Runner,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
where
    Runner: AsyncRunner,
{
    pub async fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'q, Table, Relations>
    common::DrizzlePreparedQuery<
        'q,
        RelationalPrepared,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
    >
{
    pub async fn find_many<'connection, 'transaction, Connection>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        'transaction: 'connection,
        Connection: mysql_async::prelude::ToConnection<'connection, 'transaction>,
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        let mut connection = connection
            .to_connection()
            .resolve()
            .await
            .map_err(driver_error)?;
        initialize_session(&mut connection).await?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_request(&mut connection, sql, &values).await?;
        QueryOutput::new(sql.to_owned(), values, rows).decode_relational_all::<Table, Relations>()
    }

    pub async fn find_first<'connection, 'transaction, Connection>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        'transaction: 'connection,
        Connection: mysql_async::prelude::ToConnection<'connection, 'transaction>,
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(connection, params).await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'q, Table, Relations>
    common::DrizzlePreparedQuery<
        'q,
        RelationalPrepared,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
    >
{
    pub async fn find_many<'connection, 'transaction, Connection>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        'transaction: 'connection,
        Connection: mysql_async::prelude::ToConnection<'connection, 'transaction>,
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        let mut connection = connection
            .to_connection()
            .resolve()
            .await
            .map_err(driver_error)?;
        initialize_session(&mut connection).await?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_request(&mut connection, sql, &values).await?;
        QueryOutput::new(sql.to_owned(), values, rows)
            .decode_relational_partial::<Table, Relations>()
    }

    pub async fn find_first<'connection, 'transaction, Connection>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        'transaction: 'connection,
        Connection: mysql_async::prelude::ToConnection<'connection, 'transaction>,
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(connection, params).await?.into_iter().next())
    }
}

impl<Schema: Copy> Drizzle<Conn, Schema> {
    /// Begins a transaction on this owned connection.
    ///
    /// The returned transaction borrows the connection until it is committed,
    /// rolled back, or dropped. Dropping delegates delayed rollback to
    /// `mysql_async`; the adapter restores its session invariants before the
    /// next attached query.
    pub async fn begin(
        &mut self,
        config: drizzle_mysql::TransactionConfig,
    ) -> Result<Transaction<'_, Schema>> {
        self.ensure_session().await?;
        drizzle_core::drizzle_trace_tx!("begin", "mysql.async");
        let transaction = self
            .connection
            .start_transaction(options(config))
            .await
            .map_err(driver_error)?;
        // The transaction has its own ready flag. Force the parent wrapper to
        // restore invariants after commit, rollback, panic, or delayed drop.
        self.session_ready = false;
        Ok(Transaction::new(transaction, self.schema, true))
    }

    /// Backwards-compatible alias for [`Self::begin`].
    #[deprecated(since = "0.1.17", note = "renamed to `begin`")]
    pub async fn begin_transaction(
        &mut self,
        config: drizzle_mysql::TransactionConfig,
    ) -> Result<Transaction<'_, Schema>> {
        self.begin(config).await
    }

    /// Runs `body` in a transaction, committing its value on success and
    /// rolling back on error.
    ///
    /// A callback error is returned unchanged when rollback succeeds. If
    /// rollback also fails, [`DrizzleError::TransactionError`] reports both
    /// failures; commit failures are returned directly. Cancelling the future
    /// drops the transaction and delegates delayed rollback to `mysql_async`.
    pub async fn transaction<F, R>(
        &mut self,
        config: drizzle_mysql::TransactionConfig,
        body: F,
    ) -> Result<R>
    where
        F: AsyncFnOnce(&Transaction<'_, Schema>) -> Result<R>,
    {
        let transaction = self.begin(config).await?;
        match body(&transaction).await {
            Ok(value) => {
                drizzle_core::drizzle_trace_tx!("commit", "mysql.async");
                transaction.commit().await?;
                Ok(value)
            }
            Err(error) => {
                drizzle_core::drizzle_trace_tx!("rollback", "mysql.async");
                match transaction.rollback().await {
                    Ok(()) => Err(error),
                    Err(rollback) => Err(DrizzleError::TransactionError(
                        format!(
                            "transaction callback failed: {error}; rollback failed: {rollback}"
                        )
                        .into(),
                    )),
                }
            }
        }
    }
}

impl<Schema: Copy> Drizzle<Pool, Schema> {
    /// Checks out a pooled connection and begins a transaction on it.
    ///
    /// The owned transaction returns its connection to the pool after explicit
    /// completion or the upstream driver's delayed-drop cleanup.
    pub async fn begin(
        &self,
        config: drizzle_mysql::TransactionConfig,
    ) -> Result<Transaction<'static, Schema>> {
        drizzle_core::drizzle_trace_tx!("begin", "mysql.async.pool");
        let transaction = self
            .connection
            .start_transaction(options(config))
            .await
            .map_err(driver_error)?;
        let transaction = Transaction::new(transaction, self.schema, false);
        if let Err(error) = transaction.initialize().await {
            return match transaction.rollback().await {
                Ok(()) => Err(error),
                Err(rollback) => Err(DrizzleError::TransactionError(
                    format!("pool transaction initialization failed: {error}; rollback failed: {rollback}")
                        .into(),
                )),
            };
        }
        Ok(transaction)
    }

    /// Backwards-compatible alias for [`Self::begin`].
    #[deprecated(since = "0.1.17", note = "renamed to `begin`")]
    pub async fn begin_transaction(
        &self,
        config: drizzle_mysql::TransactionConfig,
    ) -> Result<Transaction<'static, Schema>> {
        self.begin(config).await
    }

    /// Runs `body` on one pooled connection, committing its value on success
    /// and rolling back on error.
    ///
    /// A callback error is returned unchanged when rollback succeeds. If
    /// rollback also fails, [`DrizzleError::TransactionError`] reports both
    /// failures; commit failures are returned directly. Cancelling the future
    /// drops the transaction so `mysql_async` can roll it back before returning
    /// the connection to the pool.
    pub async fn transaction<F, R>(
        &self,
        config: drizzle_mysql::TransactionConfig,
        body: F,
    ) -> Result<R>
    where
        F: AsyncFnOnce(&Transaction<'static, Schema>) -> Result<R>,
    {
        let transaction = self.begin(config).await?;
        match body(&transaction).await {
            Ok(value) => {
                drizzle_core::drizzle_trace_tx!("commit", "mysql.async.pool");
                transaction.commit().await?;
                Ok(value)
            }
            Err(error) => {
                drizzle_core::drizzle_trace_tx!("rollback", "mysql.async.pool");
                match transaction.rollback().await {
                    Ok(()) => Err(error),
                    Err(rollback) => Err(DrizzleError::TransactionError(
                        format!(
                            "transaction callback failed: {error}; rollback failed: {rollback}"
                        )
                        .into(),
                    )),
                }
            }
        }
    }
}

impl<Schema> Drizzle<Conn, Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    pub async fn create(&mut self) -> Result<()> {
        for statement in Schema::default().create_statements()? {
            self.ensure_session().await?;
            execute_request(&mut self.connection, &statement, &[]).await?;
        }
        Ok(())
    }
}

impl<Schema> Drizzle<Pool, Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    pub async fn create(&self) -> Result<()> {
        let mut connection = self.checkout().await?;
        for statement in Schema::default().create_statements()? {
            execute_request(&mut connection, &statement, &[]).await?;
        }
        Ok(())
    }
}

#[allow(private_bounds)]
impl<'db, 'q, Runner, Schema, State, Table, Marker, DecodedRow, Grouped>
    DrizzleBuilder<
        'db,
        Runner,
        Schema,
        QueryBuilder<'q, Schema, State, Table, Marker, DecodedRow, Grouped>,
        State,
    >
where
    Runner: AsyncRunner,
    State: builder::ExecutableState,
{
    pub async fn execute(self) -> Result<MySQLMutationResult> {
        self.runner.execute_rendered(self.builder).await
    }

    pub async fn all<R, ScopeProof, AggProof>(self) -> Result<Vec<R>>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_rendered(self.builder)
            .await?
            .decode_all::<Marker, R>()
    }

    /// Executes this query and returns a decoded iterator over its
    /// materialized rows.
    pub async fn rows(self) -> Result<Rows<DecodedRow>>
    where
        for<'row> DecodedRow: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self
            .runner
            .query_rendered(self.builder)
            .await?
            .rows::<DecodedRow>())
    }

    pub async fn get<R, ScopeProof, AggProof>(self) -> Result<R>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_first_rendered(self.builder)
            .await?
            .decode_first::<Marker, R>()
    }

    #[must_use]
    pub fn prepare(self) -> prepared::PreparedStatement<'q, Marker, DecodedRow, Grouped> {
        prepared::PreparedStatement::new(drizzle_core::prepared::prepare_render(
            &self.builder.into_sql(),
        ))
    }
}
