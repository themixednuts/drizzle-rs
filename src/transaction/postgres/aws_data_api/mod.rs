//! AWS Aurora Serverless Data API transaction.
//!
//! The Data API has first-class server-side transactions:
//!
//! * [`Drizzle::transaction`] issues a `BeginTransaction` call; the returned
//!   `transactionId` is threaded into every subsequent `ExecuteStatement` via
//!   the `transactionId` field.
//! * Commit / rollback go through `CommitTransaction` / `RollbackTransaction`
//!   (not raw SQL).
//! * Savepoints use regular `SAVEPOINT` / `RELEASE SAVEPOINT` /
//!   `ROLLBACK TO SAVEPOINT` SQL that runs inside the transaction context.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

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
use drizzle_postgres::values::PostgresValue;

use crate::builder::postgres::aws_data_api::{
    Rows, aws_error, decode_rows, encode_params, execute_statement_raw,
};
use crate::builder::postgres::rows::DecodeRows as _;

/// Returns an error indicating the transaction has already been consumed.
fn tx_consumed_error() -> DrizzleError {
    DrizzleError::TransactionError("Transaction already consumed".into())
}

/// AWS Data API transaction builder wrapper.
#[derive(Debug)]
pub struct TransactionBuilder<'a, Schema, Builder, State> {
    transaction: &'a Transaction<Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Active AWS Aurora Data API transaction.
///
/// Owns the `transactionId` returned by `BeginTransaction` and threads it into
/// every `ExecuteStatement` until `commit()` or `rollback()` consumes it.
/// Cloning a `Client` is cheap (internal `Arc`), so a transaction can freely
/// reuse the ambient client.
pub struct Transaction<Schema = ()> {
    client: Client,
    resource_arn: Arc<str>,
    secret_arn: Arc<str>,
    database: Option<Arc<str>>,
    transaction_id: RefCell<Option<String>>,
    tx_type: PostgresTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<Schema> std::fmt::Debug for Transaction<Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("tx_type", &self.tx_type)
            .field("is_active", &self.transaction_id.borrow().is_some())
            .finish()
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
        tx_type: PostgresTransactionType,
        schema: Schema,
    ) -> Self {
        Self {
            client,
            resource_arn,
            secret_arn,
            database,
            transaction_id: RefCell::new(Some(transaction_id)),
            tx_type,
            savepoint_depth: AtomicU32::new(0),
            schema,
        }
    }

    /// Schema handle.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Isolation / transaction type configured on begin.
    #[inline]
    pub fn tx_type(&self) -> PostgresTransactionType {
        self.tx_type
    }

    /// Current transaction id, if the transaction is still open.
    pub fn transaction_id(&self) -> Option<String> {
        self.transaction_id.borrow().clone()
    }

    /// Run a nested savepoint block.
    ///
    /// On `Ok`: `RELEASE SAVEPOINT`.
    /// On `Err`: `ROLLBACK TO SAVEPOINT` + `RELEASE SAVEPOINT`.
    /// The outer transaction stays live either way.
    pub async fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: AsyncFnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        let depth = self.savepoint_depth.load(Ordering::Relaxed);
        let sp = format!("drizzle_sp_{}", depth);
        self.savepoint_depth.store(depth + 1, Ordering::Relaxed);

        self.execute(format!("SAVEPOINT {}", sp).as_str()).await?;

        let result = f(self).await;

        self.savepoint_depth.store(depth, Ordering::Relaxed);

        match result {
            Ok(v) => {
                self.execute(format!("RELEASE SAVEPOINT {}", sp).as_str())
                    .await?;
                Ok(v)
            }
            Err(e) => {
                let _ = self
                    .execute(format!("ROLLBACK TO SAVEPOINT {}", sp).as_str())
                    .await;
                let _ = self
                    .execute(format!("RELEASE SAVEPOINT {}", sp).as_str())
                    .await;
                Err(e)
            }
        }
    }

    // Builder constructors — mirror postgres_transaction_constructors! but
    // avoid the `'conn` lifetime (AWS Data API transactions don't borrow).

    /// Start a SELECT inside this transaction.
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        Schema,
        SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
    {
        let builder = QueryBuilder::new::<Schema>().select(query);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Start a SELECT DISTINCT inside this transaction.
    pub fn select_distinct<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        Schema,
        SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
    {
        let builder = QueryBuilder::new::<Schema>().select_distinct(query);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Start an INSERT inside this transaction.
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertInitial, Table>,
        InsertInitial,
    >
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Start an UPDATE inside this transaction.
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Start a DELETE inside this transaction.
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> TransactionBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteInitial, T>, DeleteInitial>
    where
        T: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Start a CTE (WITH) query inside this transaction.
    pub fn with<'a, C>(
        &'a self,
        cte: C,
    ) -> TransactionBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    // Inline execution methods.

    /// Run a raw SQL / built query and return affected row count.
    pub async fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
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
        Ok(out.number_of_records_updated.max(0) as u64)
    }

    /// Run a query and collect all rows into `C`.
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
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

    /// Run a query and return a single row (errors if empty).
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
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
        let tx_id = self
            .transaction_id
            .borrow_mut()
            .take()
            .ok_or_else(tx_consumed_error)?;
        // CommitTransaction doesn't take a database — transaction id is enough.
        self.client
            .commit_transaction()
            .resource_arn(self.resource_arn.as_ref())
            .secret_arn(self.secret_arn.as_ref())
            .transaction_id(tx_id)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| aws_error("commit_transaction", e))
    }

    /// Roll back via the service-level `RollbackTransaction` call.
    pub(crate) async fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx_id = self
            .transaction_id
            .borrow_mut()
            .take()
            .ok_or_else(tx_consumed_error)?;
        // RollbackTransaction doesn't take a database — transaction id is enough.
        self.client
            .rollback_transaction()
            .resource_arn(self.resource_arn.as_ref())
            .secret_arn(self.secret_arn.as_ref())
            .transaction_id(tx_id)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| aws_error("rollback_transaction", e))
    }

    /// Internal helper — runs a statement with this transaction's id threaded in.
    pub(crate) async fn run_statement(
        &self,
        sql: &str,
        params: Vec<aws_sdk_rdsdata::types::SqlParameter>,
    ) -> drizzle_core::error::Result<
        aws_sdk_rdsdata::operation::execute_statement::ExecuteStatementOutput,
    > {
        let tx_id = self.transaction_id.borrow();
        let tx_id = tx_id.as_deref().ok_or_else(tx_consumed_error)?;
        execute_statement_raw(
            &self.client,
            &self.resource_arn,
            &self.secret_arn,
            self.database.as_deref(),
            sql,
            params,
            Some(tx_id),
        )
        .await
    }
}

// =============================================================================
// TransactionBuilder trailing-impls (execute / all / get)
// =============================================================================

impl<'a, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'a, Schema, QueryBuilder<'a, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Run the builder and return affected row count.
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "tx_builder.execute");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self.transaction.run_statement(&sql_str, sql_params).await?;
        Ok(out.number_of_records_updated.max(0) as u64)
    }

    /// Run the builder and collect all rows using the builder's row type.
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
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
        let out = self.transaction.run_statement(&sql_str, sql_params).await?;
        let rows = decode_rows(out);
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(R::try_from(row).map_err(Into::into)?);
        }
        Ok(decoded)
    }

    /// Run the builder and return a single row.
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
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
        let out = self.transaction.run_statement(&sql_str, sql_params).await?;
        let row = decode_rows(out)
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)?;
        R::try_from(&row).map_err(Into::into)
    }
}

impl<'a, T, State> ToSQL<'a, PostgresValue<'a>> for TransactionBuilder<'_, (), T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}
