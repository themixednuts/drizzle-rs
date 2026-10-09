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
//! );
//!
//! db.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//! let users: Vec<SelectUser> = db.select(()).from(user).all().await?;
//! # Ok(()) }
//! ```
//!
//! # Statement caching
//!
//! Not applicable. The RDS Data API has no client-side prepare: its whole
//! operation set is `ExecuteStatement`, `BatchExecuteStatement`, and the three
//! transaction verbs. Every call ships SQL text over HTTP and there is no
//! statement handle to hold on to, so unlike the wire-protocol Postgres
//! drivers there is no Parse round trip for a cache to remove.
//!
//! This driver also has no `prepare()`, for the same reason — there would be
//! nothing for the returned statement to reuse beyond the rendered SQL.

use std::sync::Arc;

use aws_sdk_rdsdata::Client;
use aws_sdk_rdsdata::types::{ColumnMetadata, SqlParameter};
use drizzle_core::dialect::ParamStyle;
use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::aws_data_api::{Row, encode_param, row_from_parts};
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::transaction::TransactionConfig;
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};

use crate::builder::postgres::common;
use crate::builder::postgres::rows::DecodeRows;
use crate::transaction::postgres::aws_data_api::Transaction;

/// A query attached to a [`Drizzle`] handle, ready to run with `.execute()`,
/// `.all()`, `.get()`, or `.rows()`.
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

/// Rows returned by `.rows()`: fetched up front, decoded as you iterate.
pub type Rows<R> = DecodeRows<Row, R>;

// =============================================================================
// Drizzle
// =============================================================================

/// The Aurora Data API database handle: an [`aws_sdk_rdsdata::Client`], the
/// cluster and secret ARNs every request names, and the schema's table
/// handles.
///
/// Create it with [`Drizzle::new`], then build queries with `select`,
/// `insert`, `update`, and `delete`. It is cheap to clone and share across
/// tasks.
#[derive(Debug, Clone)]
pub struct Drizzle<Schema = ()> {
    client: Client,
    resource_arn: Arc<str>,
    secret_arn: Arc<str>,
    database: Option<Arc<str>>,
    schema: Schema,
}

impl<Schema: Default> Drizzle<Schema> {
    /// Creates a handle that sends requests through `client` to the cluster
    /// `resource_arn`, authenticating with the Secrets Manager secret
    /// `secret_arn`, and optionally naming a default `database`.
    ///
    /// Returns `(Drizzle, Schema)` like every other driver, with the schema
    /// built by `Default`. The pattern that destructures the schema usually
    /// names its type; when nothing else does, put it on the call:
    /// `Drizzle::<Schema>::new(...)`.
    #[inline]
    pub fn new(
        client: Client,
        resource_arn: impl Into<Arc<str>>,
        secret_arn: impl Into<Arc<str>>,
        database: Option<impl Into<Arc<str>>>,
    ) -> (Self, Schema) {
        let drizzle = Self {
            client,
            resource_arn: resource_arn.into(),
            secret_arn: secret_arn.into(),
            database: database.map(Into::into),
            schema: Schema::default(),
        };
        (drizzle, Schema::default())
    }
}

impl<S> AsRef<Self> for Drizzle<S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    /// Returns the wrapped AWS SDK client.
    #[inline]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the ARN of the Aurora cluster requests go to.
    #[inline]
    pub fn resource_arn(&self) -> &str {
        &self.resource_arn
    }

    /// Returns the ARN of the Secrets Manager secret holding the credentials.
    #[inline]
    pub fn secret_arn(&self) -> &str {
        &self.secret_arn
    }

    /// Returns the default database name, if one was given.
    #[inline]
    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    /// Returns the schema value this handle was created with.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    postgres_builder_constructors!();

    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, and
    /// returns the number of rows it changed (`numberOfRecordsUpdated`).
    ///
    /// Prefer the builder's own `.execute()`. This method takes anything that
    /// renders to SQL, so it skips the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or the database
    /// rejects the statement.
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
        Ok(out.number_of_records_updated.max(0).cast_unsigned())
    }

    /// Runs any SQL value and collects its rows into `C` (for example
    /// `Vec<R>`).
    ///
    /// Each row is decoded with `R: TryFrom<&Row>`. This skips the builder's
    /// compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or a row cannot be
    /// decoded into `R`.
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

    /// Runs any SQL value and returns its rows, decoded into `R` as you
    /// iterate.
    ///
    /// Every row arrives in the one Data API response. Like
    /// [`all`](Self::all), this skips the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails. Decoding errors
    /// surface per row.
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

    /// Runs any SQL value and decodes its first row into `R`.
    ///
    /// Rows after the first are ignored. Like [`all`](Self::all), this skips
    /// the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and an error
    /// when the Data API request fails or the row cannot be decoded into `R`.
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let mut rows = self.rows::<T, R>(query).await?;
        rows.next().transpose()?.ok_or(DrizzleError::NotFound)
    }

    /// Runs the async closure `f` inside a transaction and returns its value.
    ///
    /// The transaction commits when `f` returns `Ok` and rolls back when it
    /// returns `Err`. It starts with the Data API's `BeginTransaction` call;
    /// the isolation level, access mode, and `DEFERRABLE` from `config` are
    /// applied with a `SET TRANSACTION` statement. If the future is dropped
    /// before it finishes, dropping the transaction schedules a best-effort
    /// rollback on the current Tokio runtime.
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or an error when beginning, configuring,
    /// committing, or rolling back fails. When the rollback after an `Err`
    /// also fails, both errors are reported together.
    pub async fn transaction<F, R>(
        &self,
        config: impl Into<TransactionConfig>,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: AsyncFnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let config = config.into();
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
            .map_err(|e| aws_error("begin_transaction", &e))?;

        let tx_id = begin_out.transaction_id.ok_or_else(|| {
            DrizzleError::TransactionError("AWS Data API: missing transaction_id".into())
        })?;

        let tx = Transaction::new(
            self.client.clone(),
            Arc::clone(&self.resource_arn),
            Arc::clone(&self.secret_arn),
            self.database.as_ref().map(Arc::clone),
            tx_id,
            config,
            self.schema,
        );

        let mut options = Vec::new();
        if let Some(isolation) = config.isolation() {
            options.push(format!("ISOLATION LEVEL {isolation}"));
        }
        if let Some(access) = config.access() {
            options.push(access.to_string());
        }
        if config.is_deferrable() {
            options.push("DEFERRABLE".to_owned());
        }
        if !options.is_empty() {
            let preamble = format!("SET TRANSACTION {}", options.join(" "));
            if let Err(error) = tx.execute(preamble.as_str()).await {
                return Err(match tx.rollback().await {
                    Ok(()) => error,
                    Err(rollback) => crate::transaction::savepoint::cleanup_error(
                        "transaction configuration",
                        error,
                        "rollback",
                        rollback,
                    ),
                });
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
                // Report the callback's error, with a failed rollback attached.
                match tx.rollback().await {
                    Ok(()) => Err(e),
                    Err(rollback) => Err(crate::transaction::savepoint::cleanup_error(
                        "transaction",
                        e,
                        "rollback",
                        rollback,
                    )),
                }
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
    /// Creates every table, index, and view in the schema.
    ///
    /// The statements run one by one, outside a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error when one of the statements fails; earlier statements
    /// stay applied.
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
    /// Applies the migrations that have not run yet.
    ///
    /// Creates the tracking table (and its schema) if needed, then runs every
    /// pending migration and its tracking row in one Data API transaction,
    /// under `pg_advisory_xact_lock` so concurrent runs do not overlap.
    /// `CREATE/DROP INDEX CONCURRENTLY` statements cannot run in a transaction
    /// and are sent outside it, so they are not rolled back with the rest.
    ///
    /// # Errors
    ///
    /// Returns an error when a statement or Data API request fails (the
    /// transaction is rolled back), or when the tracking table holds an
    /// unfinished ("dirty") row left by an interrupted run.
    pub async fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome>
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

        let outcome = self
            .transaction(TransactionConfig::default(), async |tx| {
                tx.execute(
                    format!(
                        "SELECT pg_advisory_xact_lock({})",
                        set.postgres_advisory_lock_key()
                    )
                    .as_str(),
                )
                .await?;
                // A migration left half-applied by a non-transactional runner
                // is recorded with a NULL `applied_at`; stacking more DDL on
                // top of it would re-run statements that already landed.
                let dirty_rows = tx.run_statement(&set.dirty_names_sql(), Vec::new()).await?;
                let dirty_names = decode_rows(dirty_rows)
                    .into_iter()
                    .map(|row| row.try_get::<String>(0))
                    .collect::<drizzle_core::error::Result<Vec<_>>>()?;
                if let Some(error) = set.interrupted_migration_error(&dirty_names) {
                    return Err(DrizzleError::Other(error.to_string().into()));
                }

                let applied_rows = tx
                    .run_statement(&set.applied_names_sql(), Vec::new())
                    .await?;
                let applied_names = decode_rows(applied_rows)
                    .into_iter()
                    .map(|row| row.try_get::<String>(0))
                    .collect::<drizzle_core::error::Result<Vec<_>>>()?;
                let pending: Vec<_> = set.pending(&applied_names).collect();
                if pending.is_empty() {
                    return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
                }

                let mut applied = Vec::with_capacity(pending.len());
                for migration in &pending {
                    for statement in migration.statements() {
                        if statement.trim().is_empty() {
                            continue;
                        }
                        if drizzle_migrations::is_postgres_concurrent_index_statement(statement) {
                            self.run_statement(statement, Vec::new(), None::<&str>)
                                .await?;
                        } else {
                            tx.execute(statement).await?;
                        }
                    }
                    tx.execute(set.record_migration_sql(migration).as_str())
                        .await?;
                    applied.push(migration.tag().to_string());
                }
                Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied })
            })
            .await?;
        Ok(outcome)
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Encode a flat slice of `&PostgresValue` references as the AWS Data API's
/// [`SqlParameter`] list, using stringified 1-indexed ordinals as parameter
/// names (`"1"`, `"2"`, ...). These line up with the `:1`, `:2`, ... that the
/// builder emits via [`ParamStyle::ColonNumbered`].
pub fn encode_params(params: &[&PostgresValue<'_>]) -> Vec<SqlParameter> {
    params
        .iter()
        .enumerate()
        .map(|(i, v)| encode_param((i + 1).to_string(), v))
        .collect()
}

/// Decode rows from an `ExecuteStatementOutput` into a `Vec<Row>`, sharing the
/// column metadata via an Arc.
pub fn decode_rows(
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
pub async fn execute_statement_raw(
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
        .map_err(|e| aws_error("execute_statement", &e))
}

/// Convert an AWS SDK error into a `DrizzleError`. We preserve the service
/// message when available for easier debugging.
pub fn aws_error<E, R>(op: &str, err: &aws_sdk_rdsdata::error::SdkError<E, R>) -> DrizzleError
where
    E: std::error::Error,
{
    use aws_sdk_rdsdata::error::SdkError;
    let msg = match err {
        SdkError::ServiceError(service) => format!("aws {op}: {}", service.err()),
        other => format!("aws {op}: {other}"),
    };
    DrizzleError::Other(msg.into())
}

// =============================================================================
// Builder trailing-impls (execute / all / rows / get via DrizzleBuilder)
// =============================================================================

impl<S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'_, S, QueryBuilder<'_, Schema, State, Table, Mk, Rw, Grouped>, State>
{
    /// Runs the statement and returns the number of rows it changed
    /// (`numberOfRecordsUpdated`).
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
            drizzle_core::drizzle_profile_scope!("postgres.aws_data_api", "builder.execute");
            let (sql_str, params) = self.builder.sql.build_with(ParamStyle::ColonNumbered);
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let sql_params = encode_params(params.as_slice());
        let out = self
            .runner
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        Ok(out.number_of_records_updated.max(0).cast_unsigned())
    }

    /// Runs the query and decodes every row into `R`.
    ///
    /// The scope and grouping checks apply as on the other drivers, but `R`
    /// is not checked against the selection at compile time: it only needs
    /// `TryFrom<&Row>`, so a mismatch, including a non-`Option` field for a
    /// `NULL` value, is a runtime decode error.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails or a row cannot be
    /// decoded into `R`.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        State: builder::ExecutableState,
        Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
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
            .runner
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        let rows = decode_rows(out);
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(R::try_from(row).map_err(Into::into)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns its rows, decoded into the row type the
    /// query infers from its selection as you iterate.
    ///
    /// # Errors
    ///
    /// Returns an error when the Data API request fails. Decoding errors
    /// surface per row.
    pub async fn rows<Proof, AggProof>(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        State: builder::ExecutableState,
        Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
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
            .runner
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        Ok(Rows::new(decode_rows(out)))
    }

    /// Runs the query and decodes its first row into `R`.
    ///
    /// Rows after the first are ignored. `R` is checked as in
    /// [`all`](Self::all).
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and an error
    /// when the Data API request fails or the row cannot be decoded into `R`.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        State: builder::ExecutableState,
        Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
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
            .runner
            .run_statement(&sql_str, sql_params, None::<&str>)
            .await?;
        let rows = decode_rows(out);
        let row = rows.into_iter().next().ok_or(DrizzleError::NotFound)?;
        R::try_from(&row).map_err(Into::into)
    }
}
