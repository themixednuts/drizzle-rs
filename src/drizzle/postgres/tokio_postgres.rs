//! Async PostgreSQL driver using [`tokio_postgres`].
//!
//! # Example
//!
//! ```no_run
//! use drizzle::tokio_postgres::Drizzle;
//! use drizzle::postgres::prelude::*;
//! use tokio_postgres::NoTls;
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
//!     let (client, connection) = tokio_postgres::connect("host=localhost user=postgres", NoTls).await?;
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

mod delete;
mod insert;
mod prepared;
mod select;
mod update;

use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use std::marker::PhantomData;
use tokio_postgres::{Client, Row};

use drizzle_postgres::{
    PostgresTransactionType, PostgresValue, ToPostgresSQL,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};

/// Tokio-postgres-specific drizzle builder
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}

use crate::transaction::postgres::tokio_postgres::Transaction;

// Generic prepare method for tokio-postgres DrizzleBuilder
impl<'a: 'b, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Creates a prepared statement that can be executed multiple times
    #[inline]
    pub fn prepare(self) -> prepared::PreparedStatement<'b> {
        let inner = prepare_render(self.to_sql().clone()).into();
        prepared::PreparedStatement { inner }
    }
}

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

    /// Creates a new `Drizzle` instance from a `Config` with TokioPostgresConnection.
    ///
    /// This allows you to use the same configuration for both CLI operations
    /// and runtime database access. The connection is created from the credentials
    /// in the config.
    ///
    /// **Note**: This spawns a tokio task to drive the connection. Make sure
    /// you have a tokio runtime available.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use drizzle::tokio_postgres::Drizzle;
    /// use drizzle::postgres::prelude::*;
    ///
    /// let config = drizzle_migrations::Config::builder()
    ///     .schema::<AppSchema>()
    ///     .postgres()
    ///     .tokio_postgres("localhost", 5432, "postgres", "password", "mydb")
    ///     .out("./drizzle")
    ///     .build_with_credentials();
    ///
    /// let (db, schema) = Drizzle::with_config(config).await?;
    /// ```
    pub async fn with_config<S: drizzle_migrations::Schema>(
        config: drizzle_migrations::Config<
            S,
            drizzle_migrations::PostgresDialect,
            drizzle_migrations::TokioPostgresConnection,
            drizzle_migrations::PostgresCredentials,
        >,
    ) -> Result<(Drizzle<S>, S), tokio_postgres::Error> {
        let creds = &config.credentials;
        let conn_string = creds.connection_string();

        let (client, connection) =
            tokio_postgres::connect(&conn_string, tokio_postgres::NoTls).await?;

        // Spawn connection driver task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("postgres connection error: {}", e);
            }
        });

        let schema = config.schema;

        let drizzle = Drizzle {
            client,
            _schema: PhantomData,
        };
        Ok((drizzle, schema))
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

    /// Creates a SELECT query builder.
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'b, PostgresValue<'b>>,
    {
        use drizzle_postgres::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    pub fn insert<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
    where
        Table: PostgresTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    pub fn update<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
    where
        Table: PostgresTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    pub fn delete<'a, 'b, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, T>, DeleteInitial>
    where
        T: PostgresTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression).
    pub fn with<'a, 'b, C>(
        &'a self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, tokio_postgres::Error>
    where
        T: ToPostgresSQL<'a>,
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
        T: ToPostgresSQL<'a>,
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
        T: ToPostgresSQL<'a>,
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

// CTE (WITH) Builder Implementation for Tokio-postgres
impl<'a, Schema>
    DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(
        self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
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

impl<'a, S, T, State> ToSQL<'a, PostgresValue<'a>> for DrizzleBuilder<'a, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}
