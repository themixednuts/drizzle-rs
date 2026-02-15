//! Async PostgreSQL driver using [`tokio_postgres`].
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
//! use drizzle::postgres::params;
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, Placeholder::named("find_name")))
//!     .prepare();
//!
//! let alice: Vec<SelectUser> = find_user
//!     .all(db.client(), params![{find_name: "Alice"}])
//!     .await?;
//! # Ok(()) }
//! ```

mod prepared;

use std::sync::Arc;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use smallvec::SmallVec;
use tokio_postgres::{Client, IsolationLevel, Row};

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

crate::drizzle_prepare_impl!();

/// Async PostgreSQL database wrapper using [`tokio_postgres::Client`].
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
}

impl<S: Clone> Clone for Drizzle<S> {
    #[inline]
    fn clone(&self) -> Self {
        Drizzle {
            client: self.client.clone(),
            schema: self.schema.clone(),
        }
    }
}

/// Lazy decoded row cursor for tokio-postgres queries.
pub type Rows<R> = DecodeRows<Row, R>;

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub fn new<S: Copy>(client: Client, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            client: Arc::new(client),
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
    /// Gets a reference to the underlying client
    #[inline]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Gets a mutable reference to the underlying client.
    ///
    /// Returns `None` if there are outstanding clones of this `Drizzle` instance.
    #[inline]
    pub fn mut_client(&mut self) -> Option<&mut Client> {
        Arc::get_mut(&mut self.client)
    }

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    postgres_builder_constructors!();

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, tokio_postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let query = query.to_sql();
        let (sql, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.execute");
            let (sql, params) = query.build();
            drizzle_core::drizzle_trace_query!(&sql, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql, param_refs)
        };

        self.client.execute(&sql, &param_refs[..]).await
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
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
    pub async fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.all");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let rows = self.client.query(&sql_str, &param_refs[..]).await?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.get");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let row = self.client.query_one(&sql_str, &param_refs[..]).await?;

        R::try_from(&row).map_err(Into::into)
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
        let builder = if tx_type != PostgresTransactionType::default() {
            let isolation = match tx_type {
                PostgresTransactionType::ReadUncommitted => IsolationLevel::ReadUncommitted,
                PostgresTransactionType::ReadCommitted => IsolationLevel::ReadCommitted,
                PostgresTransactionType::RepeatableRead => IsolationLevel::RepeatableRead,
                PostgresTransactionType::Serializable => IsolationLevel::Serializable,
            };
            builder.isolation_level(isolation)
        } else {
            builder
        };
        drizzle_core::drizzle_trace_tx!("begin", "postgres.tokio");
        let tx = builder.start().await?;

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
    /// Apply pending migrations from a MigrationSet.
    ///
    /// Creates the drizzle schema if needed and runs pending migrations in a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if there are outstanding clones of this `Drizzle` instance,
    /// since exclusive access to the underlying client is required for the migration transaction.
    pub async fn migrate(
        &mut self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        if let Some(schema_sql) = migrations.create_schema_sql() {
            self.client.execute(&schema_sql, &[]).await?;
        }
        self.client
            .execute(&migrations.create_table_sql(), &[])
            .await?;
        let rows = self
            .client
            .query(&migrations.query_all_created_at_sql(), &[])
            .await?;
        let applied_created_at: Vec<i64> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();
        let pending: Vec<_> = migrations
            .pending_by_created_at(&applied_created_at)
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        let client = Arc::get_mut(&mut self.client).ok_or_else(|| {
            DrizzleError::Other("cannot run migrations: outstanding Drizzle clones exist".into())
        })?;
        let tx = client.transaction().await?;

        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    tx.execute(stmt, &[]).await?;
                }
            }
            tx.execute(
                &migrations.record_migration_sql(migration.hash(), migration.created_at()),
                &[],
            )
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}

impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.execute");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        Ok(self
            .drizzle
            .client
            .execute(&sql_str, &param_refs[..])
            .await?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        self.rows::<R>()
            .await?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    pub async fn rows<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.all");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..]).await?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.get");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let row = self
            .drizzle
            .client
            .query_one(&sql_str, &param_refs[..])
            .await?;

        R::try_from(&row).map_err(Into::into)
    }
}
