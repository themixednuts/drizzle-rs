//! AWS Aurora Serverless Data API driver.
//!
//! Builds on [`aws_sdk_rdsdata::Client`] (HTTP-based, not Postgres wire
//! protocol). Rows are returned as pre-decoded [`Field`] enums; each request
//! threads `resourceArn`/`secretArn` (and optionally `database` + a
//! `transactionId` from [`Transaction`]). See [`drizzle_postgres::aws_data_api`]
//! for `Row` decode details.
//!
//! # Quick start
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::aws::Drizzle;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! let config = ::aws_config::load_from_env().await;
//! let client = ::aws_sdk_rdsdata::Client::new(&config);
//!
//! let (db, S { user }) = Drizzle::new(
//!     client,
//!     "arn:aws:rds:us-east-1:123:cluster:my-cluster",
//!     "arn:aws:secretsmanager:us-east-1:123:secret:my-secret",
//!     Some("mydb"),
//!     S::new(),
//! );
//!
//! db.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//! let users: Vec<SelectUser> = db.select(()).from(user).all().await?;
//! # Ok(()) }
//! ```

use std::sync::Arc;

use aws_sdk_rdsdata::Client;
use aws_sdk_rdsdata::types::{ColumnMetadata, SqlParameter};
use drizzle_core::dialect::ParamStyle;
use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::aws_data_api::{Row, encode_param, row_from_parts};
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};

use crate::builder::postgres::common;
use crate::builder::postgres::rows::DecodeRows;
use crate::transaction::postgres::aws_data_api::Transaction;

/// AWS Data API drizzle builder type alias.
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

/// Lazy decoded row cursor for AWS Data API queries.
pub type Rows<R> = DecodeRows<Row, R>;

// =============================================================================
// Drizzle
// =============================================================================

/// Async AWS Aurora Serverless Data API database wrapper.
///
/// Holds an `aws_sdk_rdsdata::Client` plus the target cluster/secret ARNs.
/// The SDK `Client` is cheaply cloneable (wraps an internal `Arc`), and this
/// wrapper stores its static inputs behind `Arc<str>` so `Drizzle` itself is
/// cheaply cloneable for sharing across tasks.
#[derive(Debug, Clone)]
pub struct Drizzle<Schema = ()> {
    client: Client,
    resource_arn: Arc<str>,
    secret_arn: Arc<str>,
    database: Option<Arc<str>>,
    schema: Schema,
}

impl Drizzle {
    /// Create a new AWS Data API drizzle instance.
    ///
    /// Returns a tuple of `(Drizzle, Schema)` so callers can destructure the
    /// schema handle on the same line (mirrors every other driver).
    #[inline]
    pub fn new<S: Copy>(
        client: Client,
        resource_arn: impl Into<Arc<str>>,
        secret_arn: impl Into<Arc<str>>,
        database: Option<impl Into<Arc<str>>>,
        schema: S,
    ) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            client,
            resource_arn: resource_arn.into(),
            secret_arn: secret_arn.into(),
            database: database.map(Into::into),
            schema,
        };
        (drizzle, schema)
    }
}

impl<S> AsRef<Drizzle<S>> for Drizzle<S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    /// Reference to the underlying AWS SDK client.
    #[inline]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Target cluster resource ARN.
    #[inline]
    pub fn resource_arn(&self) -> &str {
        &self.resource_arn
    }

    /// Secrets Manager secret ARN providing credentials.
    #[inline]
    pub fn secret_arn(&self) -> &str {
        &self.secret_arn
    }

    /// Optional default database name.
    #[inline]
    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    /// Schema handle.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    postgres_builder_constructors!();

    /// Run the query and return the number of affected rows.
    pub async fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "drizzle.execute");
            let (sql_str, params) = sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        Ok(out.number_of_records_updated.max(0) as u64)
    }

    /// Run the query and collect all rows into `C`.
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: core::iter::FromIterator<R>,
    {
        self.rows(query)
            .await?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Run the query and return a lazy row cursor.
    pub async fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "drizzle.rows");
            let (sql_str, params) = sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        Ok(Rows::new(decode_rows(out)))
    }

    /// Run the query and return a single row.
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let mut rows = self.rows::<T, R>(query).await?;
        rows.next()?.ok_or(DrizzleError::NotFound)
    }

    /// Run a transaction. Returns `Ok(value)` to commit, `Err(...)` to rollback.
    ///
    /// `tx_type` selects the `ISOLATION LEVEL`. The Data API implicitly starts
    /// each transaction via the service-level `BeginTransaction` call; the
    /// isolation level is communicated as a preamble statement.
    pub async fn transaction<F, R>(
        &self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: AsyncFnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        drizzle_core::drizzle_trace_tx!("begin", "postgres.aws_data_api");

        let mut begin = self
            .client
            .begin_transaction()
            .resource_arn(self.resource_arn.as_ref())
            .secret_arn(self.secret_arn.as_ref());
        if let Some(db) = self.database.as_deref() {
            begin = begin.database(db);
        }
        let begin_out = begin
            .send()
            .await
            .map_err(|e| aws_error("begin_transaction", e))?;

        let tx_id = begin_out.transaction_id.ok_or_else(|| {
            DrizzleError::TransactionError("AWS Data API: missing transaction_id".into())
        })?;

        let tx = Transaction::new(
            self.client.clone(),
            Arc::clone(&self.resource_arn),
            Arc::clone(&self.secret_arn),
            self.database.as_ref().map(Arc::clone),
            tx_id,
            tx_type,
            self.schema,
        );

        if let Some(preamble) = isolation_preamble(tx_type) {
            // Failure to set isolation level should abort.
            if let Err(e) = tx.execute(preamble).await {
                let _ = tx.rollback().await;
                return Err(e);
            }
        }

        match f(&tx).await {
            Ok(value) => {
                drizzle_core::drizzle_trace_tx!("commit", "postgres.aws_data_api");
                tx.commit().await?;
                Ok(value)
            }
            Err(e) => {
                drizzle_core::drizzle_trace_tx!("rollback", "postgres.aws_data_api");
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    /// Internal helper used by both `Drizzle` and [`Transaction`]: issues a
    /// single `ExecuteStatement` request and returns the raw response.
    pub(crate) async fn run_statement(
        &self,
        sql: &str,
        params: Vec<SqlParameter>,
        transaction_id: Option<&str>,
    ) -> drizzle_core::error::Result<
        aws_sdk_rdsdata::operation::execute_statement::ExecuteStatementOutput,
    > {
        execute_statement_raw(
            &self.client,
            &self.resource_arn,
            &self.secret_arn,
            self.database.as_deref(),
            sql,
            params,
            transaction_id,
        )
        .await
    }
}

// =============================================================================
// Schema bootstrap / migrations
// =============================================================================

impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create all schema objects (tables, indexes, ...) from `SQLSchemaImpl`.
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements()?;
        for statement in statements {
            self.run_statement(&statement, Vec::new(), None::<&str>)
                .await?;
        }
        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// Migrations run inside a single Data API transaction so a mid-run failure
    /// rolls back cleanly. Each migration can contain multiple statements.
    pub async fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<()>
    where
        Schema: Copy,
    {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::PostgreSQL,
            tracking,
        );

        if let Some(schema_sql) = set.create_schema_sql() {
            self.run_statement(&schema_sql, Vec::new(), None::<&str>)
                .await?;
        }

        self.run_statement(&set.create_table_sql(), Vec::new(), None::<&str>)
            .await?;

        let applied_rows = self
            .run_statement(&set.applied_names_sql(), Vec::new(), None::<&str>)
            .await?;
        let applied_names: Vec<String> = decode_rows(applied_rows)
            .into_iter()
            .filter_map(|row| row.try_get::<String>(0).ok())
            .collect();
        let pending: Vec<_> = set.pending(&applied_names).collect();

        if pending.is_empty() {
            return Ok(());
        }

        // Run all pending migrations in a single transaction.
        self.transaction(PostgresTransactionType::default(), async |tx| {
            for migration in &pending {
                for stmt in migration.statements() {
                    if !stmt.trim().is_empty() {
                        tx.execute(stmt).await?;
                    }
                }
                tx.execute(set.record_migration_sql(migration).as_str())
                    .await?;
            }
            Ok(())
        })
        .await
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Encode a flat slice of `&PostgresValue` references as the AWS Data API's
/// [`SqlParameter`] list, using stringified 1-indexed ordinals as parameter
/// names (`"1"`, `"2"`, ...). These line up with the `:1`, `:2`, ... that the
/// builder emits via [`ParamStyle::ColonNumbered`].
pub(crate) fn encode_params(params: &[&PostgresValue<'_>]) -> Vec<SqlParameter> {
    params
        .iter()
        .enumerate()
        .map(|(i, v)| encode_param((i + 1).to_string(), v))
        .collect()
}

fn isolation_preamble(tx_type: PostgresTransactionType) -> Option<&'static str> {
    match tx_type {
        PostgresTransactionType::ReadUncommitted => {
            Some("SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED")
        }
        PostgresTransactionType::ReadCommitted => {
            Some("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        }
        PostgresTransactionType::RepeatableRead => {
            Some("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        }
        PostgresTransactionType::Serializable => {
            Some("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        }
    }
}

/// Decode rows from an ExecuteStatementOutput into a Vec<Row>, sharing the
/// column metadata via an Arc.
pub(crate) fn decode_rows(
    out: aws_sdk_rdsdata::operation::execute_statement::ExecuteStatementOutput,
) -> Vec<Row> {
    let metadata: Arc<[ColumnMetadata]> = out
        .column_metadata
        .unwrap_or_default()
        .into_boxed_slice()
        .into();
    out.records
        .unwrap_or_default()
        .into_iter()
        .map(|fields| row_from_parts(fields, Arc::clone(&metadata)))
        .collect()
}

/// Shared low-level executor — used by both the top-level driver and
/// [`Transaction`]. Packages the request building boilerplate and maps SDK
/// errors into [`DrizzleError`].
pub(crate) async fn execute_statement_raw(
    client: &Client,
    resource_arn: &str,
    secret_arn: &str,
    database: Option<&str>,
    sql: &str,
    params: Vec<SqlParameter>,
    transaction_id: Option<&str>,
) -> drizzle_core::error::Result<
    aws_sdk_rdsdata::operation::execute_statement::ExecuteStatementOutput,
> {
    let mut req = client
        .execute_statement()
        .resource_arn(resource_arn)
        .secret_arn(secret_arn)
        .sql(sql);
    if let Some(db) = database {
        req = req.database(db);
    }
    if let Some(tx_id) = transaction_id {
        req = req.transaction_id(tx_id);
    }
    if !params.is_empty() {
        req = req.set_parameters(Some(params));
    }
    // Ask for column metadata on every response so Row::column_name works.
    req = req.include_result_metadata(true);

    req.send()
        .await
        .map_err(|e| aws_error("execute_statement", e))
}

/// Convert an AWS SDK error into a `DrizzleError`. We preserve the service
/// message when available for easier debugging.
pub(crate) fn aws_error<E, R>(op: &str, err: aws_sdk_rdsdata::error::SdkError<E, R>) -> DrizzleError
where
    E: std::error::Error,
{
    use aws_sdk_rdsdata::error::SdkError;
    let msg = match &err {
        SdkError::ServiceError(service) => format!("aws {op}: {}", service.err()),
        other => format!("aws {op}: {other}"),
    };
    DrizzleError::Other(msg.into())
}

// =============================================================================
// Builder trailing-impls (execute / all / rows / get via DrizzleBuilder)
// =============================================================================

impl<'a, 'b, S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Run the builder and return the number of affected rows.
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "builder.execute");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .drizzle
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
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
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "builder.all");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .drizzle
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        let rows = decode_rows(out);
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(R::try_from(row).map_err(Into::into)?);
        }
        Ok(decoded)
    }

    /// Run the builder and return a lazy row cursor.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "builder.rows");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .drizzle
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        Ok(Rows::new(decode_rows(out)))
    }

    /// Run the builder and return a single row.
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "builder.get");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .drizzle
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        let rows = decode_rows(out);
        let row = rows.into_iter().next().ok_or(DrizzleError::NotFound)?;
        R::try_from(&row).map_err(Into::into)
    }
}
