use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
use drizzle_core::traits::{IsInSchema, SQLTable};
use std::marker::PhantomData;
use turso::{Connection, IntoValue, Row};

#[cfg(feature = "sqlite")]
use sqlite::{
    SQLiteTransactionType, SQLiteValue,
    builder::{
        self, QueryBuilder,
        delete::{self, DeleteBuilder},
        insert::{self, InsertBuilder},
        select::{self, SelectBuilder},
        update::{self, UpdateBuilder},
    },
};

use crate::transaction::sqlite::TransactionBuilder;

/// Transaction wrapper that provides the same query building capabilities as Drizzle
#[derive(Debug)]
pub struct Transaction<'conn, Schema = ()> {
    conn: &'conn Connection,
    tx_type: SQLiteTransactionType,
    _schema: PhantomData<Schema>,
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(conn: &'conn Connection, tx_type: SQLiteTransactionType) -> Self {
        Self {
            conn,
            tx_type,
            _schema: PhantomData,
        }
    }

    /// Gets a reference to the underlying connection
    #[inline]
    pub fn conn(&self) -> &Connection {
        self.conn
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
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'b, Schema, select::SelectInitial>,
        select::SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>>,
    {
        use sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        TransactionBuilder {
            transaction: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertInitial, Table>,
        insert::InsertInitial,
    >
    where
        Table: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        TransactionBuilder {
            transaction: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateInitial, Table>,
        update::UpdateInitial,
    >
    where
        Table: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        TransactionBuilder {
            transaction: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        DeleteBuilder<'a, Schema, delete::DeleteInitial, T>,
        delete::DeleteInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        TransactionBuilder {
            transaction: self,
            builder,
            state: PhantomData,
        }
    }

    /// Executes a raw query within the transaction
    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, DrizzleError>
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
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.conn
            .execute(&sql, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))
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
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        {
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
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Commits the transaction (turso transactions are auto-committed)
    pub async fn commit(self) -> Result<(), DrizzleError> {
        // For turso, we execute COMMIT manually
        self.conn
            .execute("COMMIT", vec![])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;
        Ok(())
    }

    /// Rolls back the transaction
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        self.conn
            .execute("ROLLBACK", vec![])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;
        Ok(())
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
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.transaction
            .conn
            .execute(&sql, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.builder.sql;
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut rows = self
            .transaction
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        {
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
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut rows = self
            .transaction
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}
