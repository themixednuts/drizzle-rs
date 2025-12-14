//! Synchronous PostgreSQL driver using [`postgres`].
//!
//! # Example
//!
//! ```no_run
//! use drizzle::postgres_sync::Drizzle;
//! use drizzle::postgres::prelude::*;
//! use postgres::{Client, NoTls};
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
//! fn main() -> drizzle::Result<()> {
//!     let client = Client::connect("host=localhost user=postgres", NoTls)?;
//!     let (mut db, AppSchema { user }) = Drizzle::new(client, AppSchema::new());
//!     db.create()?;
//!
//!     // Insert
//!     db.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!
//!     // Select
//!     let users: Vec<SelectUser> = db.select(()).from(user).all()?;
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
use postgres::{Client, Row};
use std::marker::PhantomData;

use drizzle_postgres::{
    PostgresTransactionType, PostgresValue, ToPostgresSQL,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};

/// Postgres-specific drizzle builder
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a mut Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}

use crate::transaction::postgres::postgres_sync::Transaction;

// Generic prepare method for postgres DrizzleBuilder
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

/// Synchronous PostgreSQL database wrapper using [`postgres::Client`].
///
/// Provides query building methods (`select`, `insert`, `update`, `delete`)
/// and execution methods (`execute`, `all`, `get`, `transaction`).
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

    /// Creates a new `Drizzle` instance from a `Config` with PostgresSyncConnection.
    ///
    /// This allows you to use the same configuration for both CLI operations
    /// and runtime database access. The connection is created from the credentials
    /// in the config.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use drizzle::postgres_sync::Drizzle;
    /// use drizzle::postgres::prelude::*;
    ///
    /// let config = drizzle_migrations::Config::builder()
    ///     .schema::<AppSchema>()
    ///     .postgres()
    ///     .postgres_sync("localhost", 5432, "postgres", "password", "mydb")
    ///     .out("./drizzle")
    ///     .build_with_credentials();
    ///
    /// let (db, schema) = Drizzle::with_config(config)?;
    /// ```
    pub fn with_config<S: drizzle_migrations::Schema>(
        config: drizzle_migrations::Config<
            S,
            drizzle_migrations::PostgresDialect,
            drizzle_migrations::PostgresSyncConnection,
            drizzle_migrations::PostgresCredentials,
        >,
    ) -> Result<(Drizzle<S>, S), postgres::Error> {
        let creds = &config.credentials;
        let conn_string = creds.connection_string();

        let client = Client::connect(&conn_string, postgres::NoTls)?;
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

    #[inline]
    pub fn mut_client(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Creates a SELECT query builder.
    pub fn select<'a, 'b, T>(
        &'a mut self,
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
        &'a mut self,
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
        &'a mut self,
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
        &'a mut self,
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
        &'a mut self,
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

    pub fn execute<'a, T>(&'a mut self, query: T) -> Result<u64, postgres::Error>
    where
        T: ToPostgresSQL<'a>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        self.client.execute(&sql, &param_refs[..])
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<'a, T, R, C>(&'a mut self, query: T) -> drizzle_core::error::Result<C>
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
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        let rows = self.client.query(&sql_str, &param_refs[..])?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a mut self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToPostgresSQL<'a>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params = sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        let row = self.client.query_one(&sql_str, &param_refs[..])?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Executes a transaction with the given callback
    pub fn transaction<F, R>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        // Begin transaction
        let mut tx = self.client.transaction()?;

        // Set isolation level
        tx.execute(&format!("SET TRANSACTION ISOLATION LEVEL {}", tx_type), &[])?;

        let transaction = Transaction::new(tx, tx_type);

        // Use catch_unwind to handle panics and ensure rollback
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&transaction)));

        match result {
            Ok(callback_result) => match callback_result {
                Ok(value) => {
                    transaction.commit()?;
                    Ok(value)
                }
                Err(e) => {
                    transaction.rollback()?;
                    Err(e)
                }
            },
            Err(panic_payload) => {
                // Rollback on panic and resume unwinding
                let _ = transaction.rollback();
                std::panic::resume_unwind(panic_payload);
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
    pub fn create(&mut self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements();

        for statement in statements {
            self.client.execute(&statement, &[])?;
        }

        Ok(())
    }
}

// CTE (WITH) Builder Implementation for Postgres
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

// Postgres-specific execution methods for all ExecutableState QueryBuilders
impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        Ok(self.drizzle.client.execute(&sql_str, &param_refs[..])?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..])?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        let row = self.drizzle.client.query_one(&sql_str, &param_refs[..])?;

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
