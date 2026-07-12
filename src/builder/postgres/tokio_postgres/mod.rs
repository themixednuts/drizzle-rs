//! Async `PostgreSQL` driver using [`tokio_postgres`].
//!
//! # Quick start
//!
//! ```no_run
//! use drizzle::postgres::prelude::*;
//! use drizzle::postgres::tokio::Drizzle;
//!
//! #[PostgresTable]
//! struct User {
//!     #[column(serial, primary)]
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(PostgresSchema)]
//! struct AppSchema {
//!     user: User,
//! }
//!
//! #[tokio::main]
//! async fn main() -> drizzle::Result<()> {
//!     let (client, connection) = ::tokio_postgres::connect(
//!         "host=localhost user=postgres", ::tokio_postgres::NoTls,
//!     ).await?;
//!     tokio::spawn(async move { connection.await.unwrap() });
//!
//!     let (db, AppSchema { user }) = Drizzle::new(client, AppSchema::new());
//!     db.create().await?;
//!
//!     // Insert
//!     db.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!
//!     // Select
//!     let users: Vec<SelectUser> = db.select(()).from(user).all().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Transactions
//!
//! Return `Ok(value)` to commit, `Err(...)` to rollback.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! use drizzle::postgres::common::PostgresTransactionType;
//!
//! let count = db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     Ok(users.len())
//! }).await?;
//! # Ok(()) }
//! ```
//!
//! # Savepoints
//!
//! Savepoints nest inside transactions — a failed savepoint rolls back
//! without aborting the outer transaction.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # use drizzle::postgres::common::PostgresTransactionType;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!
//!     // This savepoint fails — only its changes roll back
//!     let _: Result<(), _> = tx.savepoint(async |stx| {
//!         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
//!         Err(drizzle::error::DrizzleError::Other("oops".into()))
//!     }).await;
//!
//!     // Alice is still there
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     assert_eq!(users.len(), 1);
//!     Ok(())
//! }).await?;
//! # Ok(()) }
//! ```
//!
//! # Cloning for `tokio::spawn`
//!
//! `Drizzle` is cheaply cloneable (the underlying client is behind an
//! [`Arc`]). Move a clone into spawned tasks for concurrent queries.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (db, S { user }) = Drizzle::new(client, S::new());
//! let db_clone = db.clone();
//! tokio::spawn(async move {
//!     db_clone
//!         .insert(user)
//!         .values([InsertUser::new("Bob")])
//!         .execute()
//!         .await
//!         .expect("insert from task");
//! }).await.unwrap();
//! # Ok(()) }
//! ```
//!
//! # Prepared statements
//!
//! Build a query once and execute it many times with different parameters.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # use drizzle::core::expr::eq;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (db, S { user }) = Drizzle::new(client, S::new());
//!
//! let find_name = user.name.placeholder("find_name");
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, find_name))
//!     .prepare();
//!
//! let alice: Vec<SelectUser> = find_user
//!     .all(db.conn(), [find_name.bind("Alice")])
//!     .await?;
//! # Ok(()) }
//! ```

mod prepared;

use std::sync::Arc;

use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use smallvec::SmallVec;
use tokio_postgres::{
    Client, IsolationLevel, Row, Statement,
    types::{ToSql, Type},
};

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;

use crate::builder::postgres::common;
use crate::builder::postgres::rows::DecodeRows;

/// Tokio-postgres-specific drizzle builder
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

use crate::transaction::postgres::tokio_postgres::Transaction;

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Drizzle<Schema> {
    type PreparedDriver = Client;
}

crate::drizzle_prepare_impl!();

/// Async `PostgreSQL` database wrapper using [`tokio_postgres::Client`].
///
/// Provides query building methods (`select`, `insert`, `update`, `delete`)
/// and execution methods (`execute`, `all`, `get`, `transaction`).
///
/// The client is stored behind an [`Arc`], making `Drizzle` cheaply cloneable
/// for sharing across tasks (e.g. with [`tokio::spawn`]).
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    client: Arc<Client>,
    schema: Schema,
    statement_cache: prepared::StatementCache,
}

impl<S: Clone> Clone for Drizzle<S> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            schema: self.schema.clone(),
            statement_cache: self.statement_cache.clone(),
        }
    }
}

/// Lazy decoded row cursor for tokio-postgres queries.
pub type Rows<R> = DecodeRows<Row, R>;

fn tokio_postgres_materialize_params<'p>(
    params: &[&'p PostgresValue<'_>],
) -> (SmallVec<[Type; 8]>, SmallVec<[&'p (dyn ToSql + Sync); 8]>) {
    let mut param_types = SmallVec::with_capacity(params.len());
    let mut param_refs = SmallVec::with_capacity(params.len());
    let mut collect_types = true;

    for &param in params {
        param_refs.push(param as &(dyn ToSql + Sync));
        if collect_types {
            if let Some(ty) =
                crate::builder::postgres::prepared_common::tokio_postgres_param_type(param)
            {
                param_types.push(ty);
            } else {
                param_types.clear();
                collect_types = false;
            }
        }
    }

    (param_types, param_refs)
}

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub fn new<S: Copy>(client: Client, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            client: Arc::new(client),
            schema,
            statement_cache: prepared::StatementCache::default(),
        };
        (drizzle, schema)
    }
}

impl<S> AsRef<Self> for Drizzle<S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    /// Gets a reference to the underlying connection.
    #[inline]
    pub fn conn(&self) -> &Client {
        &self.client
    }

    /// Gets a mutable reference to the underlying connection.
    ///
    /// Returns `None` if there are outstanding clones of this `Drizzle` instance.
    #[inline]
    pub fn conn_mut(&mut self) -> Option<&mut Client> {
        Arc::get_mut(&mut self.client)
    }

    /// Gets a reference to the schema.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    async fn cached_statement(
        &self,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, tokio_postgres::Error> {
        self.statement_cache
            .statement(self.client.as_ref(), sql, param_types)
            .await
    }

    postgres_builder_constructors!();

    /// Execute a statement and return the number of affected rows.
    ///
    /// # Errors
    ///
    /// Returns a [`tokio_postgres::Error`] if the database connection fails or the SQL is invalid.
    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, tokio_postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let query = query.to_sql();
        let (sql, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.execute");
            let (sql, params) = query.build();
            drizzle_core::drizzle_trace_query!(&sql, params.len());
            (sql, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self.cached_statement(&sql, &param_types).await?;
        self.client.execute(&statement, &param_refs[..]).await
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails or row decoding fails.
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)
            .await?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails.
    pub async fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.all");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self
            .cached_statement(&sql_str, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let rows = self
            .client
            .query(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails, no rows match, or row decoding fails.
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.get");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self
            .cached_statement(&sql_str, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let row = self
            .client
            .query_one(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Creates a relational query builder for the given table.
    #[cfg(feature = "query")]
    pub fn query<'a, T>(&self, _table: T) -> common::DrizzleQueryBuilder<'_, 'a, &Self, Schema, T>
    where
        T: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            _schema: std::marker::PhantomData,
        }
    }

    /// Executes a transaction with the given callback.
    ///
    /// The transaction is committed when the callback returns `Ok` and
    /// rolled back on `Err`. Requires `&mut self` because the underlying
    /// client must not be shared during a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if there are outstanding clones of this `Drizzle` instance,
    /// since exclusive access to the underlying client is required for transactions.
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::tokio::Drizzle;
    /// # use drizzle::postgres::common::PostgresTransactionType;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # #[tokio::main] async fn main() -> drizzle::Result<()> {
    /// # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
    /// # tokio::spawn(async move { conn.await.unwrap() });
    /// # let (mut db, S { user }) = Drizzle::new(client, S::new());
    /// let count = db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     Ok(users.len())
    /// }).await?;
    /// # Ok(()) }
    /// ```
    pub async fn transaction<F, R>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: AsyncFnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let client = Arc::get_mut(&mut self.client).ok_or_else(|| {
            DrizzleError::Other("cannot start transaction: outstanding Drizzle clones exist".into())
        })?;
        let builder = client.build_transaction();
        let builder = if tx_type == PostgresTransactionType::default() {
            builder
        } else {
            let isolation = match tx_type {
                PostgresTransactionType::ReadUncommitted => IsolationLevel::ReadUncommitted,
                PostgresTransactionType::ReadCommitted => IsolationLevel::ReadCommitted,
                PostgresTransactionType::RepeatableRead => IsolationLevel::RepeatableRead,
                PostgresTransactionType::Serializable => IsolationLevel::Serializable,
            };
            builder.isolation_level(isolation)
        };
        drizzle_core::drizzle_trace_tx!("begin", "postgres.tokio");
        let tx = builder.start().await?;

        // Cancellation safety: tokio-postgres 0.7.17 rolls back an unfinished
        // Transaction in Drop by queuing ROLLBACK on the client. If the user
        // future below is dropped, this wrapper is dropped with it and the
        // inner transaction's Drop handles rollback.
        let transaction = Transaction::new(tx, tx_type, self.schema);

        match f(&transaction).await {
            Ok(value) => {
                drizzle_core::drizzle_trace_tx!("commit", "postgres.tokio");
                transaction.commit().await?;
                Ok(value)
            }
            Err(e) => {
                drizzle_core::drizzle_trace_tx!("rollback", "postgres.tokio");
                transaction.rollback().await?;
                Err(e)
            }
        }
    }
}

impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects from `SQLSchemaImpl`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if any CREATE statement fails to execute.
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements()?;

        for statement in statements {
            self.client.execute(&statement, &[]).await?;
        }

        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// Creates the drizzle schema if needed and runs pending migrations in a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if there are outstanding clones of this `Drizzle` instance,
    /// since exclusive access to the underlying client is required for the migration transaction.
    pub async fn migrate(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::PostgreSQL,
            tracking,
        );

        if let Some(schema_sql) = set.create_schema_sql() {
            self.client.execute(&schema_sql, &[]).await?;
        }
        let lock_key = set.postgres_advisory_lock_key();
        self.client
            .query_one("SELECT pg_advisory_lock($1)", &[&lock_key])
            .await?;

        let result = async {
            ensure_postgres_migration_table(&self.client, &set).await?;
            let rows = self.client.query(&set.applied_names_sql(), &[]).await?;
            let applied_names = rows
                .iter()
                .map(|row| row.try_get::<_, String>(0))
                .collect::<Result<Vec<_>, tokio_postgres::Error>>()?;
            let pending: Vec<_> = set.pending(&applied_names).collect();

            if pending.is_empty() {
                return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
            }

            let mut applied = Vec::with_capacity(pending.len());
            if set.has_postgres_concurrent_index() {
                for migration in &pending {
                    for statement in migration.statements() {
                        if !statement.trim().is_empty() {
                            self.client.execute(statement, &[]).await?;
                        }
                    }
                    self.client
                        .execute(&set.record_migration_sql(migration), &[])
                        .await?;
                    applied.push(migration.tag().to_string());
                }
            } else {
                let client = Arc::get_mut(&mut self.client).ok_or_else(|| {
                    DrizzleError::Other(
                        "cannot run migrations: outstanding Drizzle clones exist".into(),
                    )
                })?;
                let tx = client.transaction().await?;
                for migration in &pending {
                    for statement in migration.statements() {
                        if !statement.trim().is_empty() {
                            tx.execute(statement, &[]).await?;
                        }
                    }
                    tx.execute(&set.record_migration_sql(migration), &[])
                        .await?;
                    applied.push(migration.tag().to_string());
                }
                tx.commit().await?;
            }

            Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied })
        }
        .await;

        let unlock_result = self
            .client
            .query_one("SELECT pg_advisory_unlock($1)", &[&lock_key])
            .await;

        match (result, unlock_result) {
            (Ok(outcome), Ok(_)) => Ok(outcome),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
        }
    }
}

async fn ensure_postgres_migration_table(
    client: &tokio_postgres::Client,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    client.execute(&set.create_table_sql(), &[]).await?;

    let schema = set.schema_name().unwrap_or("public");
    let rows = client
        .query(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2",
            &[&schema, &set.table_name()],
        )
        .await?;
    let column_names = rows
        .iter()
        .map(|row| row.try_get::<_, String>(0))
        .collect::<Result<Vec<_>, tokio_postgres::Error>>()?;
    if column_names.iter().any(|column| column == "name") {
        return Ok(());
    }

    let rows = client
        .query(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            &[],
        )
        .await?;
    let applied = rows
        .iter()
        .map(|row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row.try_get::<_, Option<i64>>(0).ok().flatten(),
                hash: row.try_get::<_, String>(1)?,
                created_at: row.try_get::<_, i64>(2)?,
            })
        })
        .collect::<Result<Vec<_>, tokio_postgres::Error>>()?;

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))?;

    client
        .execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"name\" TEXT",
                set.table_ident_sql()
            ),
            &[],
        )
        .await?;
    client
        .execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"applied_at\" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP",
                set.table_ident_sql()
            ),
            &[],
        )
        .await?;

    for row in matched {
        let where_clause = if let Some(id) = row.id {
            format!("\"id\" = {id}")
        } else {
            format!(
                "\"created_at\" = {} AND \"hash\" = '{}'",
                row.created_at,
                row.hash.replace('\'', "''")
            )
        };
        let update_sql = format!(
            "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
            set.table_ident_sql(),
            row.name.replace('\'', "''"),
            where_clause
        );
        client.execute(&update_sql, &[]).await?;
    }

    Ok(())
}

fn pg_async_err(msg: &str, e: &tokio_postgres::Error) -> DrizzleError {
    // `tokio_postgres::Error`'s Display is just "db error"; the server's
    // actual message lives in the DbError source.
    match e.as_db_error() {
        Some(db) => DrizzleError::Other(format!("{msg}: {db}").into()),
        None => DrizzleError::Other(format!("{msg}: {e}").into()),
    }
}

async fn pg_async_query_schemas(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::ddl::Schema>> {
    use drizzle_migrations::postgres::ddl::Schema as PgSchema;
    use drizzle_migrations::postgres::introspect::queries;

    Ok(client
        .query(queries::SCHEMAS_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query schemas", &e))?
        .into_iter()
        .map(|row| PgSchema::new(row.get::<_, String>(0)))
        .collect())
}

async fn pg_async_query_tables(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawTableInfo>> {
    use drizzle_migrations::postgres::introspect::{RawTableInfo, queries};

    Ok(client
        .query(queries::TABLES_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query tables", &e))?
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get(0),
            name: row.get(1),
            is_rls_enabled: row.get(2),
            is_unlogged: row.get(3),
            is_temporary: row.get(4),
            tablespace: row.get(5),
            comment: row.get(6),
        })
        .collect())
}

async fn pg_async_query_columns(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawColumnInfo>> {
    use drizzle_migrations::postgres::introspect::{RawColumnInfo, queries};

    Ok(client
        .query(queries::COLUMNS_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query columns", &e))?
        .into_iter()
        .map(|row| RawColumnInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            column_type: row.get(3),
            type_schema: row.get(4),
            not_null: row.get(5),
            default_value: row.get(6),
            is_identity: row.get(7),
            identity_type: row.get(8),
            is_generated: row.get(9),
            generated_expression: row.get(10),
            generated_stored: row.get(11),
            dimensions: row.get(12),
            comment: row.get(13),
            ordinal_position: row.get(14),
        })
        .collect())
}

async fn pg_async_query_enums(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawEnumInfo>> {
    use drizzle_migrations::postgres::introspect::{RawEnumInfo, queries};

    Ok(client
        .query(queries::ENUMS_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query enums", &e))?
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get(0),
            name: row.get(1),
            values: row.get(2),
        })
        .collect())
}

async fn pg_async_query_sequences(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawSequenceInfo>> {
    use drizzle_migrations::postgres::introspect::{RawSequenceInfo, queries};

    Ok(client
        .query(queries::SEQUENCES_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query sequences", &e))?
        .into_iter()
        .map(|row| RawSequenceInfo {
            schema: row.get(0),
            name: row.get(1),
            data_type: row.get(2),
            start_value: row.get(3),
            min_value: row.get(4),
            max_value: row.get(5),
            increment: row.get(6),
            cycle: row.get(7),
            cache_value: row.get(8),
        })
        .collect())
}

async fn pg_async_query_views(
    client: &tokio_postgres::Client,
    schema_filter: Option<&[String]>,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawViewInfo>> {
    use drizzle_migrations::postgres::introspect::{RawViewInfo, queries};

    Ok(client
        .query(queries::VIEWS_QUERY, &[&schema_filter])
        .await
        .map_err(|e| pg_async_err("Failed to query views", &e))?
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get(0),
            name: row.get(1),
            definition: row.get(2),
            is_materialized: row.get(3),
        })
        .collect())
}

async fn pg_async_query_indexes(
    client: &tokio_postgres::Client,
    schema_filter: Option<&[String]>,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawIndexInfo>> {
    use drizzle_migrations::postgres::introspect::{RawIndexInfo, parse_index_columns, queries};

    let rows = if let Some(schemas) = schema_filter {
        client
            .query(queries::INDEXES_QUERY_FILTERED, &[&schemas])
            .await
            .map_err(|e| pg_async_err("Failed to query indexes", &e))?
    } else {
        client
            .query(queries::INDEXES_QUERY, &[])
            .await
            .map_err(|e| pg_async_err("Failed to query indexes", &e))?
    };
    Ok(rows
        .into_iter()
        .map(|row| RawIndexInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            is_unique: row.get(3),
            is_primary: row.get(4),
            method: row.get(5),
            columns: parse_index_columns(row.get(6)),
            where_clause: row.get(7),
            concurrent: false,
        })
        .collect())
}

async fn pg_async_query_foreign_keys(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawForeignKeyInfo>> {
    use drizzle_migrations::postgres::introspect::{
        RawForeignKeyInfo, pg_action_code_to_string, queries,
    };

    Ok(client
        .query(queries::FOREIGN_KEYS_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query foreign keys", &e))?
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
            schema_to: row.get(4),
            table_to: row.get(5),
            columns_to: row.get(6),
            on_update: pg_action_code_to_string(&row.get::<_, String>(7)),
            on_delete: pg_action_code_to_string(&row.get::<_, String>(8)),
            deferrable: row.get(9),
            initially_deferred: row.get(10),
        })
        .collect())
}

async fn pg_async_query_primary_keys(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawPrimaryKeyInfo>> {
    use drizzle_migrations::postgres::introspect::{RawPrimaryKeyInfo, queries};

    Ok(client
        .query(queries::PRIMARY_KEYS_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query primary keys", &e))?
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
        })
        .collect())
}

async fn pg_async_query_uniques(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawUniqueInfo>> {
    use drizzle_migrations::postgres::introspect::{RawUniqueInfo, queries};

    Ok(client
        .query(queries::UNIQUES_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query unique constraints", &e))?
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
            nulls_not_distinct: row.get(4),
            deferrable: row.get(5),
            initially_deferred: row.get(6),
        })
        .collect())
}

async fn pg_async_query_checks(
    client: &tokio_postgres::Client,
    schema_filter: Option<&[String]>,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawCheckInfo>> {
    use drizzle_migrations::postgres::introspect::{RawCheckInfo, queries};

    let rows = if let Some(schemas) = schema_filter {
        client
            .query(queries::CHECKS_QUERY_FILTERED, &[&schemas])
            .await
            .map_err(|e| pg_async_err("Failed to query check constraints", &e))?
    } else {
        client
            .query(queries::CHECKS_QUERY, &[])
            .await
            .map_err(|e| pg_async_err("Failed to query check constraints", &e))?
    };
    Ok(rows
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            expression: row.get(3),
        })
        .collect())
}

async fn pg_async_query_roles(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawRoleInfo>> {
    use drizzle_migrations::postgres::introspect::{RawRoleInfo, queries};

    Ok(client
        .query(queries::ROLES_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query roles", &e))?
        .into_iter()
        .map(|row| RawRoleInfo {
            name: row.get(0),
            create_db: row.get(1),
            create_role: row.get(2),
            inherit: row.get(3),
        })
        .collect())
}

async fn pg_async_query_policies(
    client: &tokio_postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawPolicyInfo>> {
    use drizzle_migrations::postgres::introspect::{RawPolicyInfo, queries};

    Ok(client
        .query(queries::POLICIES_QUERY, &[])
        .await
        .map_err(|e| pg_async_err("Failed to query policies", &e))?
        .into_iter()
        .map(|row| RawPolicyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            as_clause: row.get(3),
            for_clause: row.get(4),
            to: row.get(5),
            using: row.get(6),
            with_check: row.get(7),
        })
        .collect())
}

impl<Schema> Drizzle<Schema> {
    /// Introspect the connected `PostgreSQL` database and return a [`Snapshot`](drizzle_migrations::schema::Snapshot).
    ///
    /// Queries the `pg_catalog` and `information_schema` to extract tables, columns,
    /// indexes, foreign keys, primary keys, unique/check constraints, enums, sequences,
    /// views, roles, and policies.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the underlying introspection queries fail.
    pub async fn introspect(
        &self,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        self.introspect_impl(None).await
    }

    /// Inner introspection with optional schema filter.
    ///
    /// When `schema_filter` is `Some`, queries that use `pg_get_indexdef()` or
    /// `pg_get_expr()` are scoped to those schemas.  These functions call
    /// `relation_open()` which is not MVCC-protected and can fail when
    /// concurrent DDL drops objects in other schemas.
    async fn introspect_impl(
        &self,
        schema_filter: Option<&[String]>,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        use drizzle_migrations::postgres::ddl::Schema as PgSchema;
        use drizzle_migrations::postgres::introspect::{RawIntrospection, assemble_ddl};

        let schemas: Vec<PgSchema> = pg_async_query_schemas(&self.client).await?;
        let accessible_schema_names = schemas
            .iter()
            .map(|schema| schema.name().to_string())
            .collect::<Vec<_>>();
        let effective_schema_filter = schema_filter.or(Some(accessible_schema_names.as_slice()));
        let raw_tables = pg_async_query_tables(&self.client).await?;
        let raw_columns = pg_async_query_columns(&self.client).await?;
        let raw_enums = pg_async_query_enums(&self.client).await?;
        let raw_sequences = pg_async_query_sequences(&self.client).await?;
        let raw_views = pg_async_query_views(&self.client, effective_schema_filter).await?;
        let raw_indexes = pg_async_query_indexes(&self.client, effective_schema_filter).await?;
        let raw_fks = pg_async_query_foreign_keys(&self.client).await?;
        let raw_primary_keys = pg_async_query_primary_keys(&self.client).await?;
        let raw_uniques = pg_async_query_uniques(&self.client).await?;
        let raw_checks = pg_async_query_checks(&self.client, effective_schema_filter).await?;
        let raw_roles = pg_async_query_roles(&self.client).await?;
        let raw_policies = pg_async_query_policies(&self.client).await?;

        let ddl = assemble_ddl(RawIntrospection {
            schemas,
            tables: raw_tables,
            columns: raw_columns,
            enums: raw_enums,
            sequences: raw_sequences,
            views: raw_views,
            indexes: raw_indexes,
            foreign_keys: raw_fks,
            primary_keys: raw_primary_keys,
            unique_constraints: raw_uniques,
            check_constraints: raw_checks,
            roles: raw_roles,
            policies: raw_policies,
        });

        // Build snapshot
        let mut snap = drizzle_migrations::postgres::PostgresSnapshot::new();
        for entity in ddl.to_entities() {
            snap.add_entity(entity);
        }

        Ok(drizzle_migrations::schema::Snapshot::Postgres(snap))
    }

    /// Introspect the live database, diff against the desired schema, and
    /// execute the SQL statements needed to bring the database in sync.
    ///
    /// This is a no-op if the database already matches.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if introspection, diff, or applying statements fails.
    pub async fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        let desired = schema.to_snapshot();
        let target_schemas: Vec<String> = match &desired {
            drizzle_migrations::schema::Snapshot::Postgres(pg) => pg.schema_names(),
            drizzle_migrations::schema::Snapshot::Sqlite(_) => Vec::new(),
        };
        let live = self
            .introspect_impl(if target_schemas.is_empty() {
                None
            } else {
                Some(&target_schemas)
            })
            .await?;
        let live = match (live, &desired) {
            (
                drizzle_migrations::schema::Snapshot::Postgres(live_pg),
                drizzle_migrations::schema::Snapshot::Postgres(desired_pg),
            ) => {
                drizzle_migrations::schema::Snapshot::Postgres(live_pg.prepare_for_push(desired_pg))
            }
            (other, _) => other,
        };
        let generated = drizzle_migrations::diff(&live, &desired)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for stmt in generated.statements {
            if !stmt.trim().is_empty() {
                self.client.execute(&*stmt, &[]).await?;
            }
        }
        Ok(())
    }
}

impl<S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'_, S, QueryBuilder<'_, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.execute");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self
            .runner
            .cached_statement(&sql_str, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        self.runner
            .client
            .execute(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::tokio_postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::tokio_postgres::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.all");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self
            .runner
            .cached_statement(&sql_str, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let rows = self
            .runner
            .client
            .query(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::tokio_postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a lazy row cursor using the builder's row type.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.rows");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self
            .runner
            .cached_statement(&sql_str, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let rows = self
            .runner
            .client
            .query(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::tokio_postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::tokio_postgres::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.get");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };

        let (param_types, param_refs) = tokio_postgres_materialize_params(&params);
        let statement = self
            .runner
            .cached_statement(&sql_str, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let row = self
            .runner
            .client
            .query_one(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        <Mk as drizzle_core::row::DecodeSelectedRef<&::tokio_postgres::Row, R>>::decode(&row)
    }
}

// =============================================================================
// Relational Query API
// =============================================================================

#[cfg(feature = "query")]
use drizzle_core::query::DeserializeStore;
#[cfg(feature = "query")]
use drizzle_core::query::FromJsonObject as _;

// AllColumns: read base from individual row columns via TryFrom<Row>
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        Cl,
    >
{
    /// Executes the query and returns all matching rows with their relations.
    pub async fn find_many(
        self,
    ) -> drizzle_core::error::Result<
        Vec<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let num_base_cols = T::COLUMN_NAMES.len();

        let builder = self.builder;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE_NAME,
            T::COLUMN_NAMES,
            T::BLOB_COLUMNS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            false,
        );
        let (sql, bind_params) = query_sql.build();

        drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

        let (param_types, param_refs) = tokio_postgres_materialize_params(&bind_params);
        let statement = self
            .runner
            .cached_statement(&sql, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql, &bind_params))?;

        let rows = self
            .runner
            .client
            .query(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql, &bind_params))?;
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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }
}

// AllColumns find_first: requires no LIMIT set yet (internally adds LIMIT 1)
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Executes the query and returns the first matching row, or `None`.
    pub async fn find_first(
        self,
    ) -> drizzle_core::error::Result<
        Option<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

// PartialColumns: read base from a single JSON "__base" column via FromJsonObject
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    >
{
    /// Executes the query and returns all matching rows with their relations.
    ///
    /// Base columns are deserialized from a JSON `"__base"` column.
    pub async fn find_many(
        self,
    ) -> drizzle_core::error::Result<
        Vec<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let builder = self.builder;
        let column_names = &builder.cols.columns;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = column_names.clone();
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE_NAME,
            &col_refs,
            T::BLOB_COLUMNS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            true,
        );
        let (sql, bind_params) = query_sql.build();

        drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

        let (param_types, param_refs) = tokio_postgres_materialize_params(&bind_params);
        let statement = self
            .runner
            .cached_statement(&sql, &param_types)
            .await
            .with_query(|| QueryContext::new(&sql, &bind_params))?;

        let rows = self
            .runner
            .client
            .query(&statement, &param_refs[..])
            .await
            .with_query(|| QueryContext::new(&sql, &bind_params))?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            // Column 0 is the JSON "__base" object
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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }
}

// PartialColumns find_first: requires no LIMIT set yet
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Executes the query and returns the first matching row, or `None`.
    pub async fn find_first(
        self,
    ) -> drizzle_core::error::Result<
        Option<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Client, T, Rels, drizzle_core::query::AllColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub async fn find_many<const N: usize>(
        &self,
        client: &Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Vec<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );

        let num_base_cols = T::COLUMN_NAMES.len();
        let (sql_str, bound_params) = self.inner.bind(params)?;
        let (lower, upper) = bound_params.size_hint();
        let mut params_vec: SmallVec<[PostgresValue<'a>; 8]> =
            SmallVec::with_capacity(upper.unwrap_or(lower));
        params_vec.extend(bound_params);
        let mut param_refs: SmallVec<[&(dyn ToSql + Sync); 8]> =
            SmallVec::with_capacity(params_vec.len());
        for param in &params_vec {
            param_refs.push(param as &(dyn ToSql + Sync));
        }
        let param_types =
            crate::builder::postgres::prepared_common::tokio_postgres_param_types(&params_vec);

        let statement = client.prepare_typed(sql_str, &param_types).await?;
        let rows = client.query(&statement, &param_refs).await?;
        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(&row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub async fn find_first<const N: usize>(
        &self,
        client: &Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Option<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(client, params).await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Client, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub async fn find_many<const N: usize>(
        &self,
        client: &Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Vec<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );

        let (sql_str, bound_params) = self.inner.bind(params)?;
        let (lower, upper) = bound_params.size_hint();
        let mut params_vec: SmallVec<[PostgresValue<'a>; 8]> =
            SmallVec::with_capacity(upper.unwrap_or(lower));
        params_vec.extend(bound_params);
        let mut param_refs: SmallVec<[&(dyn ToSql + Sync); 8]> =
            SmallVec::with_capacity(params_vec.len());
        for param in &params_vec {
            param_refs.push(param as &(dyn ToSql + Sync));
        }
        let param_types =
            crate::builder::postgres::prepared_common::tokio_postgres_param_types(&params_vec);

        let statement = client.prepare_typed(sql_str, &param_types).await?;
        let rows = client.query(&statement, &param_refs).await?;
        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub async fn find_first<const N: usize>(
        &self,
        client: &Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Option<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(client, params).await?.into_iter().next())
    }
}
