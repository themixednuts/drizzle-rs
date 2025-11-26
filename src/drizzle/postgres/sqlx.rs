mod delete;
mod insert;
mod prepared;
mod select;
mod update;

use crate::transaction::postgres::sqlx::Transaction;
use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::SQLTable;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::{
    PostgresTransactionType, PostgresValue,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};
use sqlx::PgPool;
use std::marker::PhantomData;

/// Sqlx-specific drizzle builder
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}

// Generic prepare method for sqlx DrizzleBuilder
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Creates a prepared statement that can be executed multiple times
    #[inline]
    pub fn prepare(self) -> prepared::PreparedStatement<'a> {
        let inner = prepare_render(self.to_sql().clone());
        prepared::PreparedStatement { inner }
    }
}

/// Drizzle instance that provides access to the database and query builder.
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    pool: PgPool,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
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
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'a, Schema, InsertInitial, Table>, InsertInitial>
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'a, Schema, UpdateInitial, Table>, UpdateInitial>
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteInitial, T>, DeleteInitial>
    where
        T: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression).
    pub fn with<'a, C>(
        &'a self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, sqlx::Error>
    where
        T: ToPostgresSQL<'a>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        // Build sqlx query with proper parameter binding using the Encode trait
        let mut sqlx_query = sqlx::query(&sql);

        // Bind each parameter - PostgresValue implements sqlx::Encode
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
        T: ToPostgresSQL<'a>,
        C: std::iter::FromIterator<R>,
    {
        let query_sql = query.to_sql();
        let sql = query_sql.sql();
        let params = query_sql.params();

        // Build sqlx query with proper parameter binding using the Encode trait
        let mut sqlx_query = sqlx::query(&sql);

        // Bind each parameter - PostgresValue implements sqlx::Encode
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
        T: ToPostgresSQL<'a>,
    {
        let query_sql = query.to_sql();
        let sql = query_sql.sql();
        let params = query_sql.params();

        // Build sqlx query with proper parameter binding using the Encode trait
        let mut sqlx_query = sqlx::query(&sql);

        // Bind each parameter - PostgresValue implements sqlx::Encode
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
    pub async fn transaction<F, R, Fut>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Transaction<Schema>) -> Fut,
        Fut: std::future::Future<Output = drizzle_core::error::Result<R>>,
    {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        // Set isolation level
        sqlx::query(&format!("SET TRANSACTION ISOLATION LEVEL {}", tx_type))
            .execute(&mut *tx)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

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
            sqlx::query(&statement)
                .execute(&self.pool)
                .await
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        }

        Ok(())
    }
}

// CTE (WITH) Builder Implementation for Sqlx
impl<'a, Schema>
    DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
    where
        T: ToPostgresSQL<'a>,
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

// Sqlx-specific execution methods for all ExecutableState QueryBuilders
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params();

        // Build sqlx query with proper parameter binding using the Encode trait
        let mut sqlx_query = sqlx::query(&sql_str);

        // Bind each parameter - PostgresValue implements sqlx::Encode
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

        // Build sqlx query with proper parameter binding using the Encode trait
        let mut sqlx_query = sqlx::query(&sql_str);

        // Bind each parameter - PostgresValue implements sqlx::Encode
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

        // Build sqlx query with proper parameter binding using the Encode trait
        let mut sqlx_query = sqlx::query(&sql_str);

        // Bind each parameter - PostgresValue implements sqlx::Encode
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

impl<'a, S, T, State> ToPostgresSQL<'a> for DrizzleBuilder<'a, S, T, State>
where
    T: ToPostgresSQL<'a>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}
