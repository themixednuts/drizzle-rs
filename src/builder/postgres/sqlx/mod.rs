//! Async PostgreSQL driver using [`sqlx`] with connection pooling.
//!
//! # Example
//!
//! ```no_run
//! use drizzle::postgres::sqlx::Drizzle;
//! use drizzle::prelude::*;
//! use sqlx::PgPool;
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
//!     let pool = PgPool::connect("postgres://localhost/mydb").await?;
//!     let (db, AppSchema { user }) = Drizzle::new(pool, AppSchema::new());
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

use crate::transaction::postgres::sqlx::Transaction;
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::SQLTable;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;
use sqlx::PgPool;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use crate::builder::postgres::common;

/// Sqlx-specific drizzle builder
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

crate::drizzle_prepare_impl!();

/// Async PostgreSQL database wrapper using [`sqlx::PgPool`].
///
/// Provides query building methods (`select`, `insert`, `update`, `delete`)
/// and execution methods (`execute`, `all`, `get`, `transaction`).
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    pool: PgPool,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub const fn new<S>(pool: PgPool, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            pool,
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
    /// Gets a reference to the underlying connection pool
    #[inline]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    postgres_builder_constructors!();

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, sqlx::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        let mut sqlx_query = sqlx::query(&sql);

        for param_value in params {
            sqlx_query = sqlx_query.bind(param_value);
        }

        let result = sqlx_query.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r sqlx::postgres::PgRow>,
        for<'r> <R as TryFrom<&'r sqlx::postgres::PgRow>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        let query_sql = query.to_sql();
        let sql = query_sql.sql();
        let params = query_sql.params();

        let mut sqlx_query = sqlx::query(&sql);

        for param_value in params {
            sqlx_query = sqlx_query.bind(param_value);
        }

        let rows = sqlx_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r sqlx::postgres::PgRow>,
        for<'r> <R as TryFrom<&'r sqlx::postgres::PgRow>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let query_sql = query.to_sql();
        let sql = query_sql.sql();
        let params = query_sql.params();

        let mut sqlx_query = sqlx::query(&sql);

        for param_value in params {
            sqlx_query = sqlx_query.bind(param_value);
        }

        let row = sqlx_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

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
        )
            -> Pin<Box<dyn Future<Output = drizzle_core::error::Result<R>> + Send + 't>>,
    {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        if tx_type != PostgresTransactionType::default() {
            sqlx::query(&format!("SET TRANSACTION ISOLATION LEVEL {}", tx_type))
                .execute(&mut *tx)
                .await
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        }

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
    /// Create schema objects using SQLSchemaImpl trait
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements()?;

        for statement in statements {
            sqlx::query(&statement)
                .execute(&self.pool)
                .await
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        }

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
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        let mut sqlx_query = sqlx::query(&sql_str);

        for param_value in params {
            sqlx_query = sqlx_query.bind(param_value);
        }

        let result = sqlx_query
            .execute(self.drizzle.pool())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        Ok(result.rows_affected())
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r sqlx::postgres::PgRow>,
        for<'r> <R as TryFrom<&'r sqlx::postgres::PgRow>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        let mut sqlx_query = sqlx::query(&sql_str);

        for param_value in params {
            sqlx_query = sqlx_query.bind(param_value);
        }

        let rows = sqlx_query
            .fetch_all(self.drizzle.pool())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r sqlx::postgres::PgRow>,
        for<'r> <R as TryFrom<&'r sqlx::postgres::PgRow>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        let mut sqlx_query = sqlx::query(&sql_str);

        for param_value in params {
            sqlx_query = sqlx_query.bind(param_value);
        }

        let row = sqlx_query
            .fetch_one(self.drizzle.pool())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }
}

impl<'a, S, T, State> ToSQL<'a, PostgresValue<'a>> for DrizzleBuilder<'a, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}

impl<'a, S, T, State> drizzle_core::expr::Expr<'a, PostgresValue<'a>> for DrizzleBuilder<'a, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    type SQLType = drizzle_core::types::Any;
    type Nullable = drizzle_core::expr::NonNull;
    type Aggregate = drizzle_core::expr::Scalar;
}

