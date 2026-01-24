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
use std::marker::PhantomData;
use tokio_postgres::{Client, Row};

use drizzle_postgres::{
    PostgresTransactionType, PostgresValue,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};

use crate::builder::postgres::common;

/// Tokio-postgres-specific drizzle builder
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

use crate::transaction::postgres::tokio_postgres::Transaction;

// Generic prepare method for DrizzleBuilder
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
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

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
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params = sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = self.client.query(&sql_str, &param_refs[..]).await?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params = sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let row = self.client.query_one(&sql_str, &param_refs[..]).await?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Executes a transaction with the given callback
    pub async fn transaction<F, R, Fut>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Transaction<Schema>) -> Fut,
        Fut: std::future::Future<Output = drizzle_core::error::Result<R>>,
    {
        // Begin transaction
        let mut tx = self.client.transaction().await?;

        // Set isolation level
        tx.execute(&format!("SET TRANSACTION ISOLATION LEVEL {}", tx_type), &[])
            .await?;

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

// Implementation for schemas that implement SQLSchemaImpl
impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects using SQLSchemaImpl trait
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements();

        for statement in statements {
            self.client.execute(&statement, &[]).await?;
        }

        Ok(())
    }
}

// Migration support
impl<Schema> Drizzle<Schema> {
    /// Run pending migrations from a MigrationSet.
    ///
    /// This method follows the drizzle-orm migration spec:
    /// - Creates the `drizzle` schema if it doesn't exist
    /// - Creates the migrations table if it doesn't exist (idempotent)
    /// - Queries the last applied migration by `created_at`
    /// - Runs all pending migrations in a single transaction
    /// - Records each migration after execution
    ///
    /// # Example
    ///
    /// ```ignore
    /// use drizzle::postgres::tokio::Drizzle;
    /// use drizzle_migrations::{migrations, MigrationSet};
    /// use drizzle_types::Dialect;
    ///
    /// let migrations = migrations![
    ///     ("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
    ///     ("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
    /// ];
    /// let set = MigrationSet::new(migrations, Dialect::PostgreSQL);
    ///
    /// let (client, connection) = tokio_postgres::connect("host=localhost dbname=mydb", tokio_postgres::NoTls).await?;
    /// tokio::spawn(async move { connection.await });
    /// let (mut db, _) = Drizzle::new(client, ());
    ///
    /// db.migrate(&set).await?;
    /// ```
    pub async fn migrate(
        &mut self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        // 1. Create schema and migrations table (idempotent)
        if let Some(schema_sql) = migrations.create_schema_sql() {
            self.client.execute(&schema_sql, &[]).await?;
        }
        self.client
            .execute(&migrations.create_table_sql(), &[])
            .await?;

        // 2. Query all applied hashes
        let rows = self
            .client
            .query(&migrations.query_all_hashes_sql(), &[])
            .await?;
        let applied_hashes: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

        // 3. Get pending migrations
        let pending: Vec<_> = migrations.pending(&applied_hashes).collect();

        if pending.is_empty() {
            return Ok(());
        }

        // 4. Execute all pending in a single transaction
        let tx = self.client.transaction().await?;

        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    tx.execute(stmt, &[]).await?;
                }
            }
            // Record migration
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

// Tokio-postgres-specific execution methods for all ExecutableState QueryBuilders
impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

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
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..]).await?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let row = self
            .drizzle
            .client
            .query_one(&sql_str, &param_refs[..])
            .await?;

        R::try_from(&row).map_err(Into::into)
    }
}
