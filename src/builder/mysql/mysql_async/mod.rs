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
    },
    transaction::mysql::mysql_async::{Transaction, options},
};

/// A MySQL query attached to an async connection, pool, or transaction.
pub type DrizzleBuilder<'db, Runner, Schema, Builder, State> =
    common::DrizzleBuilder<'db, Runner, Schema, Builder, State>;

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
    let sql = "SET time_zone = '+00:00', sql_mode = REPLACE(@@SESSION.sql_mode, 'NO_UNSIGNED_SUBTRACTION', '')";
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

macro_rules! mysql_async_conn_constructors {
    () => {
        pub fn select<'db, 'q, T>(
            &'db mut self,
            columns: T,
        ) -> DrizzleBuilder<
            'db,
            &'db mut Drizzle<Conn, Schema>,
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().select(columns))
        }

        pub fn select_distinct<'db, 'q, T>(
            &'db mut self,
            columns: T,
        ) -> DrizzleBuilder<
            'db,
            &'db mut Drizzle<Conn, Schema>,
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().select_distinct(columns))
        }

        pub fn insert<'db, 'q, Table>(
            &'db mut self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            &'db mut Drizzle<Conn, Schema>,
            Schema,
            InsertBuilder<'q, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().insert(table))
        }

        pub fn update<'db, 'q, Table>(
            &'db mut self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            &'db mut Drizzle<Conn, Schema>,
            Schema,
            UpdateBuilder<'q, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().update(table))
        }

        pub fn delete<'db, 'q, Table>(
            &'db mut self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            &'db mut Drizzle<Conn, Schema>,
            Schema,
            DeleteBuilder<'q, Schema, DeleteInitial, Table>,
            DeleteInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().delete(table))
        }

        pub fn with<'db, 'q, C>(
            &'db mut self,
            cte: &C,
        ) -> DrizzleBuilder<
            'db,
            &'db mut Drizzle<Conn, Schema>,
            Schema,
            QueryBuilder<'q, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().with(cte))
        }
    };
}

impl<Schema> Drizzle<Conn, Schema> {
    async fn ensure_session(&mut self) -> Result<()> {
        if !self.session_ready {
            initialize_session(&mut self.connection).await?;
            self.session_ready = true;
        }
        Ok(())
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
        self.query_rendered(query).await?.decode_all_rows()
    }

    pub async fn get<'q, R>(&mut self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query).await?.decode_first_row()
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

    mysql_async_conn_constructors!();
}

impl<Schema> Drizzle<Pool, Schema> {
    async fn checkout(&self) -> Result<Conn> {
        let mut connection = self.connection.get_conn().await.map_err(driver_error)?;
        initialize_session(&mut connection).await?;
        Ok(connection)
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
        self.query_rendered(query).await?.decode_all_rows()
    }

    pub async fn get<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query).await?.decode_first_row()
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

    mysql_shared_builder_constructors!(&'db Drizzle<Pool, Schema>);
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

macro_rules! async_relational_terminals {
    ($runner:ty) => {
        #[cfg(feature = "query")]
        impl<'db, 'q, Schema, Table, Relations, Clauses>
            common::DrizzleQueryBuilder<
                'db,
                'q,
                $runner,
                Schema,
                Table,
                Relations,
                drizzle_core::query::AllColumns,
                Clauses,
            >
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
        impl<'db, 'q, Schema, Table, Relations, Where, Order>
            common::DrizzleQueryBuilder<
                'db,
                'q,
                $runner,
                Schema,
                Table,
                Relations,
                drizzle_core::query::AllColumns,
                drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
            >
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
        impl<'db, 'q, Schema, Table, Relations, Clauses>
            common::DrizzleQueryBuilder<
                'db,
                'q,
                $runner,
                Schema,
                Table,
                Relations,
                drizzle_core::query::PartialColumns,
                Clauses,
            >
        {
            pub async fn find_many(
                self,
            ) -> Result<
                Vec<
                    <Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row,
                >,
            >
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
        impl<'db, 'q, Schema, Table, Relations, Where, Order>
            common::DrizzleQueryBuilder<
                'db,
                'q,
                $runner,
                Schema,
                Table,
                Relations,
                drizzle_core::query::PartialColumns,
                drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
            >
        {
            pub async fn find_first(
                self,
            ) -> Result<
                Option<
                    <Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row,
                >,
            >
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
    };
}

async_relational_terminals!(&'db mut Drizzle<Conn, Schema>);
async_relational_terminals!(&'db Drizzle<Pool, Schema>);

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
    pub async fn begin_transaction(
        &mut self,
        config: drizzle_mysql::MySQLTransactionConfig,
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

    pub async fn transaction<F, R>(
        &mut self,
        config: drizzle_mysql::MySQLTransactionConfig,
        body: F,
    ) -> Result<R>
    where
        F: AsyncFnOnce(&Transaction<'_, Schema>) -> Result<R>,
    {
        let transaction = self.begin_transaction(config).await?;
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
    pub async fn begin_transaction(
        &self,
        config: drizzle_mysql::MySQLTransactionConfig,
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

    pub async fn transaction<F, R>(
        &self,
        config: drizzle_mysql::MySQLTransactionConfig,
        body: F,
    ) -> Result<R>
    where
        F: AsyncFnOnce(&Transaction<'static, Schema>) -> Result<R>,
    {
        let transaction = self.begin_transaction(config).await?;
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

macro_rules! async_builder_terminals {
    ($runner:ty) => {
        impl<'db, 'q, Schema, State, Table, Marker, DecodedRow, Grouped>
            DrizzleBuilder<
                'db,
                $runner,
                Schema,
                QueryBuilder<'q, Schema, State, Table, Marker, DecodedRow, Grouped>,
                State,
            >
        where
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
    };
}

async_builder_terminals!(&'db mut Drizzle<Conn, Schema>);
async_builder_terminals!(&'db Drizzle<Pool, Schema>);
