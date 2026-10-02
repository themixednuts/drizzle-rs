use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use postgres::fallible_iterator::FallibleIterator;
use postgres::{Row, Transaction as PgTransaction};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;

use crate::transaction::savepoint::{AbortState, sync_savepoint_tracking};

/// Returns an error indicating the transaction has already been consumed.
fn tx_consumed_error() -> DrizzleError {
    DrizzleError::TransactionError("Transaction already consumed".into())
}

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::transaction::{IsolationLevel, TransactionConfig};
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

#[cfg(feature = "query")]
use crate::builder::postgres::common;
use crate::builder::postgres::postgres_sync::{
    Rows, postgres_sync_materialize_params as materialize_params, prepared::StatementCache,
};
#[cfg(feature = "query")]
use drizzle_core::query::{DeserializeStore, FromJsonObject as _};

/// A query being built inside a [`Transaction`]. It has the same clause
/// methods as the connection's builder; run it with `.execute()`, `.all()`,
/// `.get()`, or `.rows()`.
pub type TransactionBuilder<'tx, 'conn, Schema, Builder, State> =
    crate::transaction::postgres::typestate::TransactionBuilder<
        'tx,
        &'tx Transaction<'conn, Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::postgres::postgres_sync::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!('conn);

/// An open PostgreSQL transaction, passed to the closure given to
/// `transaction`.
///
/// It has the same query methods as the database handle (`select`,
/// `insert`, `update`, `delete`, `with`), plus `savepoint` for nested
/// rollback points. It commits or rolls back when the closure returns.
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<PgTransaction<'conn>>>,
    config: TransactionConfig,
    savepoint_depth: AtomicU32,
    aborted: AbortState,
    schema: Schema,
    client_id: u64,
    statement_cache: StatementCache,
}

impl<Schema> std::fmt::Debug for Transaction<'_, Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("config", &self.config)
            .field("is_active", &self.tx.borrow().is_some())
            .finish()
    }
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Wraps a driver transaction that has already begun.
    pub(crate) const fn new(
        tx: PgTransaction<'conn>,
        config: TransactionConfig,
        schema: Schema,
        client_id: u64,
        statement_cache: StatementCache,
    ) -> Self {
        Self {
            tx: RefCell::new(Some(tx)),
            config,
            savepoint_depth: AtomicU32::new(0),
            aborted: AbortState::new(),
            schema,
            client_id,
            statement_cache,
        }
    }

    /// Resolves a cached `Statement` for a query running inside this transaction.
    fn cached_statement(
        &self,
        tx: &mut PgTransaction<'_>,
        sql: &str,
        param_types: &[postgres::types::Type],
    ) -> Result<postgres::Statement, postgres::Error> {
        self.statement_cache
            .transaction_statement(self.client_id, tx, sql, param_types)
    }

    /// Returns the schema value the database handle was created with.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the isolation level as the older `PostgresTransactionType`.
    ///
    /// This cannot distinguish server-default isolation from explicit
    /// `READ COMMITTED`. Use [`Self::config`] when that distinction matters.
    #[deprecated(since = "0.2.0", note = "use config()")]
    #[inline]
    pub const fn tx_type(&self) -> PostgresTransactionType {
        match self.config.isolation() {
            None | Some(IsolationLevel::ReadCommitted) => PostgresTransactionType::ReadCommitted,
            Some(IsolationLevel::ReadUncommitted) => PostgresTransactionType::ReadUncommitted,
            Some(IsolationLevel::RepeatableRead) => PostgresTransactionType::RepeatableRead,
            Some(IsolationLevel::Serializable) => PostgresTransactionType::Serializable,
        }
    }

    /// Returns the configuration this transaction was started with.
    #[inline]
    pub const fn config(&self) -> TransactionConfig {
        self.config
    }

    /// Converts a driver error from a statement run in this transaction.
    ///
    /// A server error aborts a PostgreSQL transaction (every later statement
    /// fails and `COMMIT` rolls back), so it is recorded: committing then
    /// reports the rollback instead of success.
    fn statement_error(&self, error: postgres::Error) -> DrizzleError {
        if error.as_db_error().is_some() {
            self.aborted.mark();
        }
        // A stale cached statement cannot be retried here (the error aborted
        // the transaction), but later work must not reuse it.
        if crate::builder::postgres::postgres_sync::prepared::is_stale_statement(&error) {
            self.statement_cache.clear_client(self.client_id);
        }
        DrizzleError::from(error)
    }

    /// Executes a raw SQL string with no parameters.
    fn execute_raw(&self, sql: &str) -> drizzle_core::error::Result<()> {
        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        tx.execute(sql, &[]).map_err(DrizzleError::from)?;
        Ok(())
    }

    /// Runs `f` inside a savepoint nested in this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released.
    /// If it returns `Err` or panics, the savepoint is rolled back.
    /// The outer transaction is unaffected either way.
    ///
    /// A savepoint whose callback returns `Ok` after one of its statements
    /// failed on the server is rolled back too, and the call fails: the
    /// failed statement aborted the transaction, and rolling back to the
    /// savepoint is what recovers it.
    ///
    /// Savepoints can be nested — each level gets its own savepoint name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # use drizzle::postgres::TransactionConfig;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
    /// # let (mut db, S { user }) = Drizzle::new(client);
    /// db.transaction(TransactionConfig::default(), |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///
    ///     // This savepoint fails — only its changes roll back
    ///     let _: Result<(), _> = tx.savepoint(|stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute()?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     });
    ///
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     assert_eq!(users.len(), 1); // only Alice
    ///     Ok(())
    /// })?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or an error when `SAVEPOINT`, `RELEASE`, or
    /// `ROLLBACK TO` fails. When cleanup after an `Err` also fails, both
    /// errors are reported together.
    pub fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        sync_savepoint_tracking(
            &self.savepoint_depth,
            &self.aborted,
            |sql| self.execute_raw(sql),
            || f(self),
        )
    }

    postgres_transaction_constructors!('conn);

    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, inside
    /// the transaction and returns the number of rows it changed.
    ///
    /// Prefer the builder's own `.execute()`. This method skips the builder's
    /// compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the statement.
    /// A server error aborts a PostgreSQL transaction: later statements fail,
    /// and it rolls back instead of committing.
    pub fn execute<'q, T>(&self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute");
        let query_sql = query.to_sql();
        let (sql, params) = query_sql.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());

        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute.db");
        let statement = self
            .cached_statement(tx, &sql, &param_types)
            .map_err(|error| self.statement_error(error))?;
        Ok(tx
            .execute(&statement, &param_refs[..])
            .map_err(|error| self.statement_error(error))?)
    }

    /// Runs any SQL value inside the transaction and decodes every row into
    /// `R` with `TryFrom<&Row>`, skipping the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded into
    /// `R`.
    pub fn all<'q, T, R, C>(&self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs any SQL value inside the transaction and returns its rows, fetched
    /// up front and decoded into `R` as you iterate.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails. Decoding errors surface per row.
    pub fn rows<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.all");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.all.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        let statement = self
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(|error| self.statement_error(error))?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .map_err(|error| self.statement_error(error))?;

        Ok(Rows::new(rows))
    }

    /// Runs any SQL value inside the transaction and decodes its single row
    /// into `R`.
    ///
    /// # Errors
    ///
    /// Returns an error when the query does not return exactly one row (none
    /// or several), when it fails, or when the row cannot be decoded into `R`.
    pub fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.get");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.get.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        let statement = self
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(|error| self.statement_error(error))?;
        let row = tx
            .query_one(&statement, &param_refs[..])
            .map_err(|error| self.statement_error(error))?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Starts a relational query inside this transaction, like the database
    /// handle's `query`. It sees the transaction's uncommitted writes.
    #[cfg(feature = "query")]
    pub fn query<'a, T>(&self, _table: T) -> common::DrizzleQueryBuilder<'_, 'a, &Self, Schema, T>
    where
        T: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            _schema: PhantomData,
        }
    }

    #[cfg(feature = "query")]
    fn relational_rows<'q>(
        &self,
        query: &drizzle_core::sql::SQL<'q, PostgresValue<'q>>,
    ) -> drizzle_core::error::Result<Vec<Row>> {
        let (sql, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());

        let (param_types, param_refs) = materialize_params(&params);
        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        let statement = self
            .cached_statement(tx, &sql, &param_types)
            .map_err(|error| self.statement_error(error))?;

        tx.query(&statement, &param_refs[..])
            .map_err(|error| self.statement_error(error))
    }

    /// Commits the transaction.
    pub(crate) fn commit(self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        // PostgreSQL answers COMMIT in an aborted transaction by rolling back
        // without an error; report it instead of returning `Ok`.
        if self.aborted.is_aborted() {
            tx.rollback().map_err(DrizzleError::from)?;
            return Err(crate::transaction::savepoint::aborted_transaction_error());
        }
        tx.commit().map_err(DrizzleError::from)
    }

    /// Rolls back the transaction.
    pub(crate) fn rollback(self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.rollback().map_err(DrizzleError::from)
    }
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = postgres::Client;
}

// AllColumns: read base from individual row columns via TryFrom<Row>
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        Cl,
    >
{
    /// Runs the relational query and returns every root row with its loaded
    /// relations.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded.
    pub fn find_many(
        self,
    ) -> drizzle_core::error::Result<
        Vec<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::Select,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let num_base_cols = T::COLUMN_NAMES.len();
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
        let rows = self.runner.relational_rows(&query_sql)?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
    }
}

// AllColumns find_first: requires no LIMIT set yet (internally adds LIMIT 1)
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Runs the relational query with `LIMIT 1` and returns the first root row,
    /// or `None` when nothing matches.
    ///
    /// Available only while no `.limit(..)` is set.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or the row cannot be decoded.
    pub fn find_first(
        self,
    ) -> drizzle_core::error::Result<
        Option<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::Select,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

// PartialColumns: read base from a single JSON "__base" column via FromJsonObject
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    >
{
    /// Runs the relational query and returns every root row with its loaded
    /// relations, in the table's `PartialSelect*` shape.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded.
    pub fn find_many(
        self,
    ) -> drizzle_core::error::Result<
        Vec<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::PartialSelect>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let builder = self.builder;
        let column_names = &builder.cols.columns;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = column_names.clone();
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
        let rows = self.runner.relational_rows(&query_sql)?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            let base_json: String = row.get(0);
            let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
                &base_json, "base",
            )?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
    }
}

// PartialColumns find_first: requires no LIMIT set yet
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Runs the relational query with `LIMIT 1` and returns the first root row,
    /// or `None` when nothing matches.
    ///
    /// Available only while no `.limit(..)` is set.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or the row cannot be decoded.
    pub fn find_first(
        self,
    ) -> drizzle_core::error::Result<
        Option<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::PartialSelect>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

// `TransactionBuilder<CTEInit>::select` and `.with` are now provided by
// the shared `DrizzleBuilder` typestate impls (see
// `crate::builder::postgres::common`).

impl<'tx, 'q, S, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, '_, S, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the statement inside the transaction and returns the number of
    /// rows it changed.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the statement.
    /// A server error aborts a PostgreSQL transaction: later statements fail,
    /// and it rolls back instead of committing.
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute.db");
        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(|error| self.runner.statement_error(error))?;
        Ok(tx
            .execute(&statement, &param_refs[..])
            .map_err(|error| self.runner.statement_error(error))?)
    }

    /// Runs the query inside the transaction and decodes every row into `R`.
    ///
    /// Reads see the transaction's own uncommitted writes.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded into
    /// `R`.
    ///
    /// # Compile-time checks
    ///
    /// The call does not compile unless every column the query reads belongs to
    /// a table in its `FROM`/`JOIN` list, `R` matches the selection (with
    /// `Option<T>` wherever a value can be `NULL`), and, with `GROUP BY`, each
    /// column in a selected tuple is grouped or aggregated.
    pub fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(|error| self.runner.statement_error(error))?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .map_err(|error| self.runner.statement_error(error))?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query inside the transaction and returns its rows, decoded into
    /// the row type the query infers from its selection as you iterate.
    ///
    /// Every row is fetched before this returns. Unlike [`all`](Self::all),
    /// this method does not check scope or grouping at compile time.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails. Decoding errors surface per row.
    pub fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.rows");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.rows.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(|error| self.runner.statement_error(error))?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .map_err(|error| self.runner.statement_error(error))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query inside the transaction and decodes its single row into
    /// `R`.
    ///
    /// # Errors
    ///
    /// Returns an error when the query does not return exactly one row (none
    /// or several), when it fails, or when the row cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The same checks as [`all`](Self::all).
    pub fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(|error| self.runner.statement_error(error))?;
        let row = tx
            .query_one(&statement, &param_refs[..])
            .map_err(|error| self.runner.statement_error(error))?;

        <Mk as drizzle_core::row::DecodeSelectedRef<&::postgres::Row, R>>::decode(&row)
    }
}

// `ToSQL for TransactionBuilder` is now provided by the shared `DrizzleBuilder`
// impl in `crate::builder::postgres::common`.
