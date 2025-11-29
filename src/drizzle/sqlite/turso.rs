mod delete;
mod insert;
mod prepared;
mod select;
mod update;

use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use std::marker::PhantomData;
use turso::{Connection, IntoValue, Row};

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    SQLiteTransactionType, SQLiteValue,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};

/// Turso-specific drizzle builder
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}

// Generic prepare method for turso DrizzleBuilder
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Creates a prepared statement that can be executed multiple times
    #[inline]
    pub fn prepare(self) -> prepared::PreparedStatement<'a> {
        let inner = prepare_render(self.to_sql());
        prepared::PreparedStatement { inner }
    }
}
use crate::transaction::sqlite::turso::Transaction;

/// Drizzle instance that provides access to the database and query builder.
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    conn: Connection,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
    #[inline]
    pub const fn new<S>(conn: Connection, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            conn,
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
    /// Gets a reference to the underlying connection
    #[inline]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[inline]
    pub fn mut_conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    #[cfg(feature = "sqlite")]
    pub fn select<'a, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        use drizzle_sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'a, Schema, InsertInitial, T>, InsertInitial>
    where
        T: SQLiteTable<'a> + 'a,
    {
        use drizzle_sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    #[cfg(feature = "sqlite")]
    pub fn update<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'a, Schema, UpdateInitial, T>, UpdateInitial>
    where
        T: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    #[cfg(feature = "sqlite")]
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteInitial, T>, DeleteInitial>
    where
        T: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression).
    #[cfg(feature = "sqlite")]
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

    pub async fn execute<'a, T>(
        &'a self,
        query: T,
    ) -> Result<u64, drizzle_core::error::DrizzleError>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params: Vec<turso::Value> = query
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.conn
            .execute(&sql, params)
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Executes a transaction with the given callback
    pub async fn transaction<F, R>(
        &mut self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: for<'t> FnOnce(
            &'t Transaction<Schema>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = drizzle_core::error::Result<R>> + Send + 't>,
        >,
    {
        let tx = self.conn.transaction_with_behavior(tx_type.into()).await?;
        let transaction = Transaction::new(tx, tx_type);

        match f(&transaction).await {
            Ok(result) => {
                transaction.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = transaction.rollback().await;
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
        for sql in statements {
            self.conn.execute(&sql, ()).await?;
        }
        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Run embedded migrations.
    ///
    /// This method applies compile-time embedded migrations to the database.
    /// Migrations are embedded using the `include_migrations!` macro.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use drizzle::prelude::*;
    /// use drizzle::turso::Drizzle;
    ///
    /// // Embed migrations at compile time
    /// const MIGRATIONS: EmbeddedMigrations = include_migrations!("./drizzle");
    ///
    /// async fn run() -> Result<()> {
    ///     let conn = turso::Connection::open(":memory:")?;
    ///     let (db, _) = Drizzle::new(conn, ());
    ///
    ///     // Apply embedded migrations
    ///     let applied = db.migrate(&MIGRATIONS).await?;
    ///     println!("Applied {} migrations", applied);
    ///     Ok(())
    /// }
    /// ```
    pub async fn migrate(
        &self,
        migrations: &drizzle_migrations::EmbeddedMigrations,
    ) -> drizzle_core::error::Result<usize> {
        if migrations.is_empty() {
            return Ok(0);
        }

        // Create migrations table
        self.conn.execute(migrations.create_table_sql(), ()).await?;

        // Get applied migrations
        let mut rows = self.conn.query(migrations.query_applied_sql(), ()).await?;
        let mut applied: Vec<String> = Vec::new();
        while let Some(row) = rows.next().await? {
            applied.push(row.get::<String>(0)?);
        }

        // Get pending migrations
        let pending = migrations.pending(&applied);
        let count = pending.len();

        for migration in pending {
            // Execute each statement
            for stmt in migration.statements() {
                self.conn.execute(stmt, ()).await?;
            }

            // Record as applied
            self.conn
                .execute(migrations.record_migration_sql(), [migration.tag])
                .await?;
        }

        Ok(count)
    }
}

// CTE (WITH) Builder Implementation for Turso
#[cfg(feature = "turso")]
impl<'a, Schema>
    DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
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

// Generic execution methods for all ExecutableState QueryBuilders (Turso)
#[cfg(feature = "turso")]
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let sql_str = self.builder.sql.sql();
        let params: Vec<turso::Value> = self
            .builder
            .sql
            .params()
            .into_iter()
            .map(|p| p.into())
            .collect();
        Ok(self.drizzle.conn.execute(&sql_str, params).await?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r turso::Row>,
        for<'r> <R as TryFrom<&'r turso::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: std::iter::FromIterator<R>,
    {
        let sql_str = self.builder.sql.sql();
        let params: Vec<turso::Value> = self
            .builder
            .sql
            .params()
            .into_iter()
            .map(|p| p.into())
            .collect();

        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }
        Ok(results.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r turso::Row>,
        for<'r> <R as TryFrom<&'r turso::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params: Vec<turso::Value> = self
            .builder
            .sql
            .params()
            .into_iter()
            .map(|p| p.into())
            .collect();

        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(drizzle_core::error::DrizzleError::NotFound)
        }
    }
}

impl<'a, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'a, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
        self.builder.to_sql()
    }
}
