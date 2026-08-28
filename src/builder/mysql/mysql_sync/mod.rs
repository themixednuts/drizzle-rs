//! Blocking MySQL adapter using the [`mysql`] crate.
//!
//! [`Drizzle`] owns the exact connection passed to [`Drizzle::new`]. To use a
//! pool, check out a [`mysql::PooledConn`] first and pass that connection to
//! Drizzle. Dropping the wrapper then returns the connection to its original
//! pool through the upstream driver's normal ownership rules.
//!
//! The adapter executes every statement through MySQL's prepared binary
//! protocol, including statements without parameters. Before its first query,
//! it fixes the session time zone at UTC and removes
//! `NO_UNSIGNED_SUBTRACTION` and `REAL_AS_FLOAT` so typed numeric expressions
//! and `REAL` columns have stable Rust types.
//!
//! # Quick start
//!
//! Add `mysql = "28"` beside `drizzle` in the application manifest, then pass
//! an upstream connection into Drizzle:
//!
//! ```no_run
//! use drizzle::mysql::{mysql_sync::Drizzle, prelude::*};
//!
//! #[MySQLTable]
//! struct User {
//!     #[column(PRIMARY, AUTO_INCREMENT)]
//!     id: u64,
//!     #[column(VARCHAR(255))]
//!     name: String,
//! }
//!
//! #[derive(MySQLSchema)]
//! struct AppSchema {
//!     user: User,
//! }
//!
//! fn main() -> drizzle::Result<()> {
//!     let opts = ::mysql::Opts::from_url("mysql://root:mysql@localhost/app")
//!         .map_err(|error| drizzle::error::DrizzleError::driver("MySQL", error))?;
//!     let connection = ::mysql::Conn::new(opts)
//!         .map_err(|error| drizzle::error::DrizzleError::driver("MySQL", error))?;
//!     let (mut db, AppSchema { user }) = Drizzle::new(connection, AppSchema::new());
//!     db.create()?;
//!
//!     db.insert(user).value(InsertUser::new("Alice")).execute()?;
//!     let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//!     assert_eq!(users.len(), 1);
//!     Ok(())
//! }
//! ```

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
use mysql::{Row, Value, prelude::Queryable};

use crate::builder::mysql::{
    common,
    driver_common::{QueryOutput, positional, render},
    introspect,
};
use crate::transaction::{
    mysql::mysql_sync::{Transaction, TransactionConnection},
    savepoint::sync_transaction,
};

/// A MySQL query attached to this blocking adapter.
pub type DrizzleBuilder<'db, Connection, Schema, Builder, State> =
    common::DrizzleBuilder<'db, &'db mut Drizzle<Connection, Schema>, Schema, Builder, State>;

/// Decoded MySQL rows from a fully materialized query result.
///
/// The iterator owns its result rows and does not retain a connection or
/// transaction borrow while callers decode them.
pub type Rows<R> = crate::builder::mysql::driver_common::Rows<R>;

pub(crate) fn driver_error(error: mysql::Error) -> DrizzleError {
    DrizzleError::driver("MySQL", error)
}

pub(crate) fn execute_request_observing<C: Queryable>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    mut observe_error: impl FnMut(&mysql::Error),
) -> Result<MySQLMutationResult> {
    drizzle_core::drizzle_trace_query!(sql, values.len());
    let context_values = values.iter().collect::<Vec<_>>();
    let params = positional(values.iter().cloned().map(Value::from));
    let result = connection
        .exec_iter(sql, params)
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    let mutation = MySQLMutationResult::new(result.affected_rows(), result.last_insert_id());
    drop(result);
    Ok(mutation)
}

pub(crate) fn execute_request<C: Queryable>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
) -> Result<MySQLMutationResult> {
    execute_request_observing(connection, sql, values, |_| {})
}

pub(crate) fn initialize_session(connection: &mut impl Queryable) -> Result<()> {
    initialize_session_observing(connection, |_| {})
}

pub(crate) fn initialize_session_observing(
    connection: &mut impl Queryable,
    observe_error: impl FnMut(&mysql::Error),
) -> Result<()> {
    let sql = "SET time_zone = '+00:00', sql_mode = REPLACE(REPLACE(@@SESSION.sql_mode, 'NO_UNSIGNED_SUBTRACTION', ''), 'REAL_AS_FLOAT', '')";
    execute_request_observing(connection, sql, &[], observe_error).map(|_| ())
}

pub(crate) fn query_request_observing<C: Queryable>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    mut observe_error: impl FnMut(&mysql::Error),
) -> Result<Vec<Row>> {
    drizzle_core::drizzle_trace_query!(sql, values.len());
    let context_values = values.iter().collect::<Vec<_>>();
    let params = positional(values.iter().cloned().map(Value::from));
    let result = connection
        .exec_iter(sql, params)
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;

    result
        .map(|row| {
            row.map_err(|error| {
                observe_error(&error);
                driver_error(error)
            })
            .with_query(|| QueryContext::new(sql, &context_values))
        })
        .collect::<Result<Vec<_>>>()
}

pub(crate) fn query_request<C: Queryable>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
) -> Result<Vec<Row>> {
    query_request_observing(connection, sql, values, |_| {})
}

pub(crate) fn query_first_request_observing<C: Queryable>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    mut observe_error: impl FnMut(&mysql::Error),
) -> Result<Option<Row>> {
    drizzle_core::drizzle_trace_query!(sql, values.len());
    let context_values = values.iter().collect::<Vec<_>>();
    let params = positional(values.iter().cloned().map(Value::from));
    let mut result = connection
        .exec_iter(sql, params)
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    let row = result
        .next()
        .transpose()
        .map_err(|error| {
            observe_error(&error);
            driver_error(error)
        })
        .with_query(|| QueryContext::new(sql, &context_values))?;
    drop(result);
    Ok(row)
}

pub(crate) fn query_first_request<C: Queryable>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
) -> Result<Option<Row>> {
    query_first_request_observing(connection, sql, values, |_| {})
}

fn catalog_query<C, T>(
    connection: &mut C,
    sql: &str,
    values: &[MySQLValue<'_>],
    decode: impl FnOnce(Vec<Row>) -> Result<T>,
) -> Result<T>
where
    C: Queryable,
{
    let context_values = values.iter().collect::<Vec<_>>();
    decode(query_request(connection, sql, values)?)
        .with_query(|| QueryContext::new(sql, &context_values))
}

fn catalog<C: Queryable>(connection: &mut C) -> Result<introspect::Catalog> {
    use drizzle_migrations::mysql::introspect::{
        RawIntrospection, apply_show_create_view, queries,
    };

    let database = catalog_query(connection, queries::DATABASE, &[], introspect::database)?;
    let selected_database = database.name.clone();
    let values = [MySQLValue::from(selected_database.as_str())];
    let mut raw = RawIntrospection {
        database,
        tables: catalog_query(connection, queries::TABLES, &values, introspect::tables)?,
        columns: catalog_query(connection, queries::COLUMNS, &values, introspect::columns)?,
        indexes: catalog_query(connection, queries::INDEXES, &values, introspect::indexes)?,
        primary_keys: catalog_query(
            connection,
            queries::PRIMARY_KEYS,
            &values,
            introspect::primary_keys,
        )?,
        foreign_keys: catalog_query(
            connection,
            queries::FOREIGN_KEYS,
            &values,
            introspect::foreign_keys,
        )?,
        checks: catalog_query(connection, queries::CHECKS, &values, introspect::checks)?,
        views: catalog_query(connection, queries::VIEWS, &values, introspect::views)?,
    };

    for view in &mut raw.views {
        let sql = introspect::view_sql(&view.database, &view.name);
        let statement = catalog_query(connection, &sql, &[], |rows| {
            introspect::view_statement(rows, &view.name)
        })?;
        apply_show_create_view(view, &statement);
    }

    introspect::Catalog::assemble(raw)
}

fn apply<C, S>(connection: &mut C, schema: &S) -> Result<()>
where
    C: Queryable,
    S: drizzle_migrations::Schema,
{
    let catalog = catalog(connection)?;
    let desired = schema.to_snapshot();
    for statement in catalog.plan(&desired)?.statements {
        if !statement.trim().is_empty() {
            execute_request(connection, &statement, &[])?;
        }
    }
    Ok(())
}

/// Blocking MySQL database wrapper.
///
/// `Connection` is the exact upstream resource supplied by the caller. A
/// [`mysql::PooledConn`] therefore returns to its pool when this wrapper is
/// dropped; Drizzle does not hide checkout or replacement behind its API.
pub struct Drizzle<Connection, Schema = ()> {
    connection: Connection,
    schema: Schema,
    session_ready: bool,
}

impl<Connection, Schema> core::fmt::Debug for Drizzle<Connection, Schema> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Drizzle")
            .field("session_ready", &self.session_ready)
            .finish_non_exhaustive()
    }
}

impl<Connection> Drizzle<Connection> {
    /// Attaches a schema to an upstream MySQL connection or checked-out pool
    /// connection. Construction performs no I/O.
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
    /// Borrows the exact upstream connection supplied at construction.
    #[must_use]
    pub const fn conn(&self) -> &Connection {
        &self.connection
    }

    /// Mutably borrows the exact upstream connection supplied at construction.
    pub fn conn_mut(&mut self) -> &mut Connection {
        // Raw access can change session variables. Force the adapter to restore
        // its decoding invariants before the next attached query.
        self.session_ready = false;
        &mut self.connection
    }

    /// Borrows the schema attached at construction.
    #[must_use]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Consumes the wrapper and returns the upstream connection.
    #[must_use]
    pub fn into_inner(self) -> Connection {
        self.connection
    }
}

impl<Connection: Queryable, Schema> Drizzle<Connection, Schema> {
    fn ensure_session(&mut self) -> Result<()> {
        if self.session_ready {
            return Ok(());
        }

        initialize_session(&mut self.connection)?;
        self.session_ready = true;
        Ok(())
    }

    /// Introspects the selected MySQL database.
    ///
    /// # Errors
    ///
    /// Returns DrizzleError when no database is selected, a catalog query
    /// fails, or MySQL reports metadata that cannot be represented losslessly.
    pub fn introspect(&mut self) -> Result<drizzle_migrations::schema::Snapshot> {
        self.ensure_session()?;
        catalog(&mut self.connection).map(introspect::Catalog::into_snapshot)
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
    pub fn push<S: drizzle_migrations::Schema>(&mut self, schema: &S) -> Result<()> {
        self.ensure_session()?;
        apply(&mut self.connection, schema)
    }

    pub(crate) fn execute_rendered<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        self.ensure_session()?;
        let (sql, values) = render(query);
        execute_request(&mut self.connection, &sql, &values)
    }

    pub(crate) fn query_rendered<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session()?;
        let (sql, values) = render(query);
        let rows = query_request(&mut self.connection, &sql, &values)?;
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub(crate) fn query_first_rendered<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session()?;
        let (sql, values) = render(query);
        let rows = query_first_request(&mut self.connection, &sql, &values)?
            .into_iter()
            .collect();
        Ok(QueryOutput::new(sql, values, rows))
    }

    /// Executes arbitrary typed MySQL SQL through the prepared/binary protocol.
    pub fn execute<'q>(
        &mut self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        let result = self.execute_rendered(query);
        // Arbitrary SQL can change the session settings owned by this adapter.
        self.session_ready = false;
        result
    }

    /// Executes typed MySQL SQL and decodes every returned row.
    pub fn all<'q, R>(&mut self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<Vec<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.rows::<_, R>(query)?.collect()
    }

    /// Executes typed MySQL SQL and returns a decoded iterator over its
    /// materialized rows.
    ///
    /// The database result is fully consumed before this method returns.
    pub fn rows<'q, T, R>(&mut self, query: T) -> Result<Rows<R>>
    where
        T: ToSQL<'q, MySQLValue<'q>>,
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self.query_rendered(query)?.rows::<R>())
    }

    /// Executes typed MySQL SQL and decodes the first row.
    pub fn get<'q, R>(&mut self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query)?.decode_first_row()
    }

    /// Creates a typed relational query for `table`.
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

    mysql_builder_constructors!(Connection, [&'db mut self], self);
}

#[cfg(feature = "query")]
#[doc(hidden)]
pub struct RelationalPrepared;

#[cfg(feature = "query")]
impl<Connection, Schema> common::RelationalPreparedDriver for &mut Drizzle<Connection, Schema> {
    type PreparedDriver = RelationalPrepared;
}

#[cfg(feature = "query")]
impl<'db, 'q, Connection, Schema, Table, Relations, Clauses>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db mut Drizzle<Connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        Clauses,
    >
where
    Connection: Queryable,
{
    pub fn find_many(
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
            .query_rendered(query)?
            .decode_relational_all::<Table, Relations>()
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Connection, Schema, Table, Relations, Where, Order>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db mut Drizzle<Connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
where
    Connection: Queryable,
{
    pub fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Connection, Schema, Table, Relations, Clauses>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db mut Drizzle<Connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        Clauses,
    >
where
    Connection: Queryable,
{
    pub fn find_many(
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
            .query_rendered(query)?
            .decode_relational_partial::<Table, Relations>()
    }
}

#[cfg(feature = "query")]
impl<'db, 'q, Connection, Schema, Table, Relations, Where, Order>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db mut Drizzle<Connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
where
    Connection: Queryable,
{
    pub fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
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
    pub fn find_many(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        initialize_session(connection)?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_request(connection, sql, &values)?;
        QueryOutput::new(sql.to_owned(), values, rows).decode_relational_all::<Table, Relations>()
    }

    pub fn find_first(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(connection, params)?.into_iter().next())
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
    pub fn find_many(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        initialize_session(connection)?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_request(connection, sql, &values)?;
        QueryOutput::new(sql.to_owned(), values, rows)
            .decode_relational_partial::<Table, Relations>()
    }

    pub fn find_first(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(connection, params)?.into_iter().next())
    }
}

#[allow(private_bounds)]
impl<Connection: TransactionConnection, Schema: Copy> Drizzle<Connection, Schema> {
    fn start(
        &mut self,
        config: drizzle_mysql::TransactionConfig,
    ) -> Result<Transaction<'_, Schema>> {
        self.ensure_session()?;
        let transaction =
            crate::transaction::mysql::mysql_sync::start_transaction(&mut self.connection, config)
                .map_err(driver_error)?;
        // Raw transaction queries can change session variables. Restore the
        // adapter invariants when the parent becomes available again.
        self.session_ready = false;
        Ok(Transaction::new(transaction, self.schema))
    }

    /// Runs a transaction, committing on `Ok` and rolling back on `Err` or
    /// panic.
    pub fn transaction<R>(
        &mut self,
        config: drizzle_mysql::TransactionConfig,
        body: impl FnOnce(&Transaction<Schema>) -> Result<R>,
    ) -> Result<R> {
        let transaction = self.start(config)?;
        sync_transaction(
            transaction,
            "mysql.sync",
            || {
                drizzle_core::drizzle_trace_tx!("commit", "mysql.sync");
            },
            || {
                drizzle_core::drizzle_trace_tx!("rollback", "mysql.sync");
            },
            |transaction| body(transaction),
            |transaction| transaction.commit(),
            |transaction| transaction.rollback(),
        )
    }
}

impl<Schema> Drizzle<mysql::Conn, Schema> {
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
    pub fn migrate(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> Result<drizzle_migrations::MigrateOutcome> {
        let result = migration::Runner::new(&mut self.connection, migrations, tracking).run();
        self.session_ready = false;
        result
    }
}

impl<Schema> Drizzle<mysql::PooledConn, Schema> {
    /// Applies pending embedded migrations in MySQL autocommit mode.
    ///
    /// This holds the checked-out connection for the advisory lock's entire
    /// lifetime. Interrupted migrations remain dirty and require manual
    /// reconciliation before this method will continue.
    ///
    /// The checked-out connection must have autocommit enabled and must not be
    /// inside a transaction begun through raw SQL or the raw driver.
    pub fn migrate(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> Result<drizzle_migrations::MigrateOutcome> {
        let result = migration::Runner::new(&mut self.connection, migrations, tracking).run();
        self.session_ready = false;
        result
    }
}

impl<Connection: Queryable, Schema> Drizzle<Connection, Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Creates every schema object in dependency order.
    pub fn create(&mut self) -> Result<()> {
        for statement in Schema::default().create_statements()? {
            self.ensure_session()?;
            execute_request(&mut self.connection, &statement, &[])?;
        }
        Ok(())
    }
}

impl<'db, 'q, Connection, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<
        'db,
        Connection,
        Schema,
        QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>,
        State,
    >
where
    Connection: Queryable,
    State: builder::ExecutableState,
{
    /// Executes this statement through MySQL's prepared/binary protocol.
    pub fn execute(self) -> Result<MySQLMutationResult> {
        self.runner.execute_rendered(self.builder)
    }

    /// Executes this query and decodes every row using its inferred marker.
    pub fn all<R, ScopeProof, AggProof>(self) -> Result<Vec<R>>
    where
        for<'row> Mk: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, Rw, R>,
        Mk: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_rendered(self.builder)?
            .decode_all::<Mk, R>()
    }

    /// Executes this query and returns a decoded iterator over its
    /// materialized rows.
    pub fn rows(self) -> Result<Rows<Rw>>
    where
        for<'row> Rw: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self.runner.query_rendered(self.builder)?.rows::<Rw>())
    }

    /// Executes this query and decodes its first row.
    pub fn get<R, ScopeProof, AggProof>(self) -> Result<R>
    where
        for<'row> Mk: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, Rw, R>,
        Mk: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_first_rendered(self.builder)?
            .decode_first::<Mk, R>()
    }

    /// Detaches a reusable prepared query from this runner.
    #[must_use]
    pub fn prepare(self) -> prepared::PreparedStatement<'q, Mk, Rw, Grouped> {
        prepared::PreparedStatement::new(drizzle_core::prepared::prepare_render(
            &self.builder.into_sql(),
        ))
    }
}
