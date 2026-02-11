//! Async PostgreSQL driver using [`tokio_postgres`].
//!
//! # Example
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
//!     let (client, connection) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
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

mod prepared;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use smallvec::SmallVec;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
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
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    client: Client,
    _schema: PhantomData<Schema>,
}

/// Lazy decoded row cursor for tokio-postgres queries.
pub type Rows<R> = DecodeRows<Row, R>;

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub const fn new<S>(client: Client, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            client,
            _schema: PhantomData,
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

    /// Gets a mutable reference to the underlying client
    #[inline]
    pub fn mut_client(&mut self) -> &mut Client {
        &mut self.client
    }

    postgres_builder_constructors!();

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, tokio_postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.execute");
        let query = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.execute.build");
        let (sql, params) = query.build();

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.execute.param_refs");
        let mut param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync)),
        );

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
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.all");
        let sql = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.all.build");
        let (sql_str, params) = sql.build();

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync)),
        );

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
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.get");
        let sql = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.get.build");
        let (sql_str, params) = sql.build();

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync)),
        );

        let row = self.client.query_one(&sql_str, &param_refs[..]).await?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Executes a transaction with the given callback
    pub async fn transaction<F, R>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: for<'t> FnOnce(
            &'t Transaction<Schema>,
        ) -> Pin<
            Box<dyn Future<Output = drizzle_core::error::Result<R>> + Send + 't>,
        >,
    {
        let builder = self.client.build_transaction();
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
        let tx = builder.start().await?;

        let transaction = Transaction::new(tx, tx_type);

        match f(&transaction).await {
            Ok(value) => {
                transaction.commit().await?;
                Ok(value)
            }
            Err(e) => {
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
            .query(&migrations.query_all_hashes_sql(), &[])
            .await?;
        let applied_hashes: Vec<String> = rows.iter().map(|r| r.get(0)).collect();
        let pending: Vec<_> = migrations.pending(&applied_hashes).collect();

        if pending.is_empty() {
            return Ok(());
        }

        let tx = self.client.transaction().await?;

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
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.execute");
        let (sql_str, params) = self.builder.sql.build();

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.execute.param_refs");
        let mut param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync)),
        );

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
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.all");
        let (sql_str, params) = self.builder.sql.build();

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync)),
        );

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..]).await?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.get");
        let (sql_str, params) = self.builder.sql.build();

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync)),
        );

        let row = self
            .drizzle
            .client
            .query_one(&sql_str, &param_refs[..])
            .await?;

        R::try_from(&row).map_err(Into::into)
    }
}
