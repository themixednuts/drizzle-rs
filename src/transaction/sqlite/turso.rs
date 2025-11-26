use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use std::marker::PhantomData;
use turso::Row;

pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    SQLiteTransactionType, SQLiteValue,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
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
        let sql = query.sql();
        let params: Vec<turso::Value> = query.params().into_iter().map(|p| p.into()).collect();

        Ok(self.tx.execute(&sql, params).await?)
    }

    /// Runs a query and returns all matching rows within the transaction
    pub async fn all<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql.params().into_iter().map(|p| p.into()).collect();

        let mut rows = self.tx.query(&sql_str, params).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs a query and returns a single row within the transaction
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql.params().into_iter().map(|p| p.into()).collect();

        let mut rows = self.tx.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Commits the transaction (turso transactions are auto-committed)
    pub async fn commit(self) -> Result<(), DrizzleError> {
        // For turso, we execute COMMIT manually
        Ok(self.tx.commit().await?)
    }

    /// Rolls back the transaction
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        Ok(self.tx.rollback().await?)
    }
}

// turso-specific execution methods for all ExecutableState QueryBuilders in transactions
#[cfg(feature = "turso")]
impl<'a, 'conn, S, Schema, State, Table>
    TransactionBuilder<'a, 'conn, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let sql = self.builder.sql.sql();
        let params: Vec<turso::Value> = self
            .builder
            .sql
            .params()
            .into_iter()
            .map(|p| p.into())
            .collect();

        Ok(self.transaction.tx.execute(&sql, params).await?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.builder.sql;
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql.params().into_iter().map(|p| p.into()).collect();

        let mut rows = self.transaction.tx.query(&sql_str, params).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.builder.sql;
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql.params().into_iter().map(|p| p.into()).collect();

        let mut rows = self.transaction.tx.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}
