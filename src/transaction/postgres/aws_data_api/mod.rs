//! AWS Aurora Serverless Data API transaction.
//!
//! The Data API has first-class server-side transactions:
//!
//! * [`Drizzle::transaction`](crate::postgres::aws::Drizzle::transaction)
//!   issues a `BeginTransaction` call; the returned
//!   `transactionId` is threaded into every subsequent `ExecuteStatement` via
//!   the `transactionId` field.
//! * Commit / rollback go through `CommitTransaction` / `RollbackTransaction`
//!   (not raw SQL).
//! * Savepoints use regular `SAVEPOINT` / `RELEASE SAVEPOINT` /
//!   `ROLLBACK TO SAVEPOINT` SQL that runs inside the transaction context.

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::transaction::savepoint::{AsyncSavepointState, async_savepoint};

use aws_sdk_rdsdata::Client;
use drizzle_core::dialect::ParamStyle;
use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::aws_data_api::Row;
use drizzle_postgres::builder::{
    self, DeleteInitial, InsertInitial, QueryBuilder, SelectInitial, UpdateInitial,
    delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder, update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::transaction::{IsolationLevel, TransactionConfig};
use drizzle_postgres::values::PostgresValue;

use crate::builder::postgres::aws_data_api::{
    Rows, aws_error, decode_rows, encode_params, execute_statement_raw,
};

/// Returns an error indicating the transaction has already been consumed.
fn tx_consumed_error() -> DrizzleError {
    DrizzleError::TransactionError("Transaction already consumed".into())
}

/// A query being built inside a [`Transaction`]. It has the same clause
/// methods as the connection's builder; run it with `.execute()`, `.all()`,
/// `.get()`, or `.rows()`.
pub type TransactionBuilder<'tx, Schema, Builder, State> =
    crate::transaction::postgres::typestate::TransactionBuilder<
        'tx,
        &'tx Transaction<Schema>,
        Schema,
        Builder,
        State,
    >;

/// An open Aurora Data API transaction, passed to the closure given to
/// `transaction`.
///
/// It has the same query methods as the database handle (`select`,
/// `insert`, `update`, `delete`, `with`), plus `savepoint`. Every request it
/// sends carries the `transactionId` returned by `BeginTransaction`.
///
/// You do not commit or roll back by hand:
/// [`Drizzle::transaction`](crate::postgres::aws::Drizzle::transaction) ends
/// the transaction with `CommitTransaction` when the callback returns `Ok` and
/// with `RollbackTransaction` when it returns `Err`. If the transaction is
/// dropped while still open, for example because the future returned by
/// `transaction` was dropped before it finished, `Drop` spawns a best-effort
/// `RollbackTransaction` on the current Tokio runtime.
pub struct Transaction<Schema = ()> {
    client: Client,
    resource_arn: Arc<str>,
    secret_arn: Arc<str>,
    database: Option<Arc<str>>,
    tx_id: Mutex<Option<String>>,
    config: TransactionConfig,
    savepoints: AsyncSavepointState,
    schema: Schema,
}

impl<Schema> Drop for Transaction<Schema> {
    fn drop(&mut self) {
        let Some(transaction_id) = self.tx_id.get_mut().ok().and_then(Option::take) else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };

        let client = self.client.clone();
        let resource_arn = Arc::clone(&self.resource_arn);
        let secret_arn = Arc::clone(&self.secret_arn);
        let _rollback = runtime.spawn(async move {
            let _ = client
                .rollback_transaction()
                .resource_arn(resource_arn.as_ref())
                .secret_arn(secret_arn.as_ref())
                .transaction_id(transaction_id)
                .send()
                .await;
        });
    }
}

impl<Schema> std::fmt::Debug for Transaction<Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let is_active = self.tx_id.lock().is_ok_and(|g| g.is_some());
        f.debug_struct("Transaction")
            .field("config", &self.config)
            .field("is_active", &is_active)
            .field("savepoints", &self.savepoints)
            .field("resource_arn", &self.resource_arn)
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

impl<Schema> Transaction<Schema> {
    /// Construct a new transaction handle.
    pub(crate) fn new(
        client: Client,
        resource_arn: Arc<str>,
        secret_arn: Arc<str>,
        database: Option<Arc<str>>,
        transaction_id: String,
        config: TransactionConfig,
        schema: Schema,
    ) -> Self {
        Self {
            client,
            resource_arn,
            secret_arn,
            database,
            tx_id: Mutex::new(Some(transaction_id)),
            config,
            savepoints: AsyncSavepointState::new(),
            schema,
        }
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

    /// Returns the Data API transaction ID, or `None` once the transaction
    /// has ended.
    pub fn transaction_id(&self) -> Option<String> {
        self.tx_id.lock().ok().and_then(|g| g.clone())
    }

    /// Runs `f` inside a savepoint nested in this transaction.
    ///
    /// If `f` returns `Ok`, the savepoint is released; if it returns `Err`, the
    /// savepoint is rolled back and released. The outer transaction stays
    /// usable either way.
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or an error when `SAVEPOINT`, `RELEASE`, or
    /// `ROLLBACK TO` fails. When cleanup after an `Err` also fails, both
    /// errors are reported together.
    pub async fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: AsyncFnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        async_savepoint(
            &self.savepoints,
            |sql| async move { self.execute(sql.as_str()).await.map(|_| ()) },
            f(self),
        )
        .await
    }

    postgres_transaction_constructors!();

    // Inline execution methods.

    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, inside
    /// the transaction and returns the number of rows it changed.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or the database
    /// rejects the statement.
    pub async fn execute<'q, T>(&self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx.execute");
            let (sql_str, params) = sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.run_statement(&sql_str, sql_params).await?;
        Ok(out.number_of_records_updated.max(0).cast_unsigned())
    }

    /// Runs any SQL value inside the transaction and collects its rows into
    /// `C` (for example `Vec<R>`).
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or a row cannot be
    /// decoded into `R`.
    pub async fn all<'q, T, R, C>(&self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
        C: core::iter::FromIterator<R>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx.all");
            let (sql_str, params) = sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.run_statement(&sql_str, sql_params).await?;
        let rows = decode_rows(out);
        rows.into_iter()
            .map(|row| R::try_from(&row).map_err(Into::into))
            .collect()
    }

    /// Runs any SQL value inside the transaction and returns its rows, decoded
    /// into `R` as you iterate.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails. Decoding errors
    /// surface per row.
    pub async fn rows<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx.rows");
            let (sql_str, params) = sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.run_statement(&sql_str, sql_params).await?;
        Ok(Rows::new(decode_rows(out)))
    }

    /// Runs any SQL value inside the transaction and decodes its first row
    /// into `R`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and an error
    /// when the Data API request fails or the row cannot be decoded into `R`.
    pub async fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx.get");
            let (sql_str, params) = sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.run_statement(&sql_str, sql_params).await?;
        let row = decode_rows(out)
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)?;
        R::try_from(&row).map_err(Into::into)
    }

    /// Commit via the service-level `CommitTransaction` call.
    pub(crate) async fn commit(&self) -> drizzle_core::error::Result<()> {
        if let Err(error) = self.savepoints.ensure_usable() {
            self.rollback().await?;
            return Err(error);
        }
        if self.savepoints.aborted().is_aborted() {
            self.rollback().await?;
            return Err(crate::transaction::savepoint::aborted_transaction_error());
        }
        let tx_id = self
            .tx_id
            .lock()
            .map_err(|_| tx_consumed_error())?
            .clone()
            .ok_or_else(tx_consumed_error)?;
        // CommitTransaction doesn't take a database — transaction id is enough.
        self.client
            .commit_transaction()
            .resource_arn(self.resource_arn.as_ref())
            .secret_arn(self.secret_arn.as_ref())
            .transaction_id(tx_id)
            .send()
            .await
            .map_err(|e| aws_error("commit_transaction", &e))?;
        self.tx_id.lock().map_err(|_| tx_consumed_error())?.take();
        Ok(())
    }

    /// Roll back via the service-level `RollbackTransaction` call.
    pub(crate) async fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx_id = self
            .tx_id
            .lock()
            .map_err(|_| tx_consumed_error())?
            .clone()
            .ok_or_else(tx_consumed_error)?;
        // RollbackTransaction doesn't take a database — transaction id is enough.
        self.client
            .rollback_transaction()
            .resource_arn(self.resource_arn.as_ref())
            .secret_arn(self.secret_arn.as_ref())
            .transaction_id(tx_id)
            .send()
            .await
            .map_err(|e| aws_error("rollback_transaction", &e))?;
        self.tx_id.lock().map_err(|_| tx_consumed_error())?.take();
        Ok(())
    }

    /// Internal helper — runs a statement with this transaction's id threaded in.
    pub(crate) async fn run_statement(
        &self,
        sql: &str,
        params: Vec<aws_sdk_rdsdata::types::SqlParameter>,
    ) -> drizzle_core::error::Result<
        aws_sdk_rdsdata::operation::execute_statement::ExecuteStatementOutput,
    > {
        self.savepoints.ensure_usable()?;
        // Clone the id out of the `Mutex` so no guard is held across the `.await` below.
        // Holding a `MutexGuard` over an await would make this future `!Send` (on older
        // compilers) and risks lock contention stalls.
        let tx_id = self
            .tx_id
            .lock()
            .map_err(|_| tx_consumed_error())?
            .clone()
            .ok_or_else(tx_consumed_error)?;
        let result = execute_statement_raw(
            &self.client,
            &self.resource_arn,
            &self.secret_arn,
            self.database.as_deref(),
            sql,
            params,
            Some(&tx_id),
        )
        .await;
        // A failed statement aborts the PostgreSQL transaction behind the
        // Data API: every later statement fails and COMMIT rolls back.
        if result.is_err() {
            self.savepoints.aborted().mark();
        }
        result
    }
}

// =============================================================================
// TransactionBuilder trailing-impls (execute / all / get)
// =============================================================================

impl<'tx, 'q, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, Schema, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
{
    /// Runs the statement inside the transaction and returns the number of
    /// rows it changed.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or the database
    /// rejects the statement.
    pub async fn execute(self) -> drizzle_core::error::Result<u64>
    where
        State: builder::ExecutableState,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx_builder.execute");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.runner.run_statement(&sql_str, sql_params).await?;
        Ok(out.number_of_records_updated.max(0).cast_unsigned())
    }

    /// Runs the query inside the transaction and decodes every row into `R`.
    ///
    /// The scope and grouping checks of the database handle's `.all()` do not
    /// apply here, and `R` only needs `TryFrom<&Row>`: a row that does not fit
    /// is a runtime decode error.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or a row cannot be
    /// decoded into `R`.
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        State: builder::ExecutableState,
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx_builder.all");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.runner.run_statement(&sql_str, sql_params).await?;
        let rows = decode_rows(out);
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(R::try_from(row).map_err(Into::into)?);
        }
        Ok(decoded)
    }

    /// Runs the query inside the transaction and returns its rows, decoded
    /// into the row type the query infers from its selection as you iterate.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails. Decoding errors
    /// surface per row.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        State: builder::ExecutableState,
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx_builder.rows");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.runner.run_statement(&sql_str, sql_params).await?;
        Ok(Rows::new(decode_rows(out)))
    }

    /// Runs the query inside the transaction and decodes its first row into
    /// `R`, with the same (runtime) row checks as [`all`](Self::all).
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and an error
    /// when the Data API request fails or the row cannot be decoded into `R`.
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        State: builder::ExecutableState,
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx_builder.get");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.runner.run_statement(&sql_str, sql_params).await?;
        let row = decode_rows(out)
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)?;
        R::try_from(&row).map_err(Into::into)
    }
}

// `ToSQL for TransactionBuilder` is now provided by the shared `DrizzleBuilder`
// impl in `crate::builder::postgres::common`.
