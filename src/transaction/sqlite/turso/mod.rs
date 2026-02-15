use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use turso::Row;

use crate::builder::sqlite::rows::TursoRows as Rows;

pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

/// Turso-specific transaction builder
#[derive(Debug)]
pub struct TransactionBuilder<'a, 'conn, Schema, Builder, State> {
    transaction: &'a Transaction<'conn, Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction wrapper that provides the same query building capabilities as Drizzle
#[derive(Debug)]
pub struct Transaction<'conn, Schema = ()> {
    tx: turso::transaction::Transaction<'conn>,
    tx_type: SQLiteTransactionType,
    savepoint_depth: AtomicU32,
    _schema: PhantomData<Schema>,
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(
        tx: turso::transaction::Transaction<'conn>,
        tx_type: SQLiteTransactionType,
    ) -> Self {
        Self {
            tx,
            tx_type,
            savepoint_depth: AtomicU32::new(0),
            _schema: PhantomData,
        }
    }

    /// Gets a reference to the underlying connection
    #[inline]
    pub fn tx(&self) -> &turso::transaction::Transaction<'conn> {
        &self.tx
    }

    /// Gets the transaction type
    #[inline]
    pub fn tx_type(&self) -> SQLiteTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    async fn execute_raw(&self, sql: &str) -> Result<(), DrizzleError> {
        self.tx.execute(sql, ()).await?;
        Ok(())
    }

    /// Executes a nested savepoint within this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released.
    /// If it returns `Err`, the savepoint is rolled back.
    pub async fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: AsyncFnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        let depth = self.savepoint_depth.load(Ordering::Relaxed);
        let sp_name = format!("drizzle_sp_{}", depth);
        self.savepoint_depth.store(depth + 1, Ordering::Relaxed);

        self.execute_raw(&format!("SAVEPOINT {}", sp_name)).await?;

        let result = f(self).await;

        self.savepoint_depth.store(depth, Ordering::Relaxed);

        match result {
            Ok(value) => {
                self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name))
                    .await?;
                Ok(value)
            }
            Err(e) => {
                let _ = self
                    .execute_raw(&format!("ROLLBACK TO SAVEPOINT {}", sp_name))
                    .await;
                let _ = self
                    .execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name))
                    .await;
                Err(e)
            }
        }
    }

    /// Creates a SELECT query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> TransactionBuilder<'a, 'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'b, SQLiteValue<'b>>,
    {
        use drizzle_sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an INSERT query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertInitial, Table>,
        InsertInitial,
    >
    where
        Table: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an UPDATE query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'a,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
    where
        Table: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates a DELETE query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> TransactionBuilder<
        'a,
        'a,
        Schema,
        DeleteBuilder<'a, Schema, DeleteInitial, T>,
        DeleteInitial,
    >
    where
        T: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Executes a raw query within the transaction
    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, DrizzleError>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let (sql_str, params) = query.build();
        let params: Vec<turso::Value> = params.into_iter().map(|p| p.into()).collect();

        Ok(self.tx.execute(&sql_str, params).await?)
    }

    /// Runs a query and returns all matching rows within the transaction
    pub async fn all<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        self.rows(query).await?.collect().await
    }

    /// Runs a query and returns a row cursor within the transaction.
    pub async fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        let params: Vec<turso::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self.tx.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs a query and returns a single row within the transaction
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        let params: Vec<turso::Value> = params.into_iter().map(|p| p.into()).collect();

        let mut rows = self.tx.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Commits the transaction (turso transactions are auto-committed)
    pub async fn commit(self) -> Result<(), DrizzleError> {
        Ok(self.tx.commit().await?)
    }

    /// Rolls back the transaction
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        Ok(self.tx.rollback().await?)
    }
}

#[cfg(feature = "turso")]
impl<'a, 'conn, S, Schema, State, Table>
    TransactionBuilder<'a, 'conn, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<turso::Value> = params.into_iter().map(|p| p.into()).collect();

        Ok(self.transaction.tx.execute(&sql_str, params).await?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        self.rows::<R>().await?.collect().await
    }

    /// Runs the query and returns a row cursor.
    pub async fn rows<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<turso::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self.transaction.tx.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<turso::Value> = params.into_iter().map(|p| p.into()).collect();

        let mut rows = self.transaction.tx.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}
