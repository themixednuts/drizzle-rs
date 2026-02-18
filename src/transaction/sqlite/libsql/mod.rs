mod delete;
mod insert;
mod select;
mod update;

use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use libsql::Row;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::builder::sqlite::rows::LibsqlRows as Rows;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

/// LibSQL-specific transaction builder
#[derive(Debug)]
pub struct TransactionBuilder<'a, Schema, Builder, State> {
    transaction: &'a Transaction<Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<Schema = ()> {
    tx: libsql::Transaction,
    tx_type: SQLiteTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<Schema> std::fmt::Debug for Transaction<Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("tx_type", &self.tx_type)
            .finish()
    }
}

impl<Schema> Transaction<Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(
        tx: libsql::Transaction,
        tx_type: SQLiteTransactionType,
        schema: Schema,
    ) -> Self {
        Self {
            tx,
            tx_type,
            savepoint_depth: AtomicU32::new(0),
            schema,
        }
    }

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Gets a reference to the underlying transaction
    #[inline]
    pub fn inner(&self) -> &libsql::Transaction {
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
    /// The outer transaction is unaffected either way.
    ///
    /// Savepoints can be nested — each level gets its own savepoint name.
    ///
    /// ```ignore
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::libsql::Drizzle;
    /// # use drizzle::sqlite::connection::SQLiteTransactionType;
    /// db.transaction(SQLiteTransactionType::Deferred, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///
    ///     let _ = tx.savepoint(async |stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     }).await;
    ///
    ///     // Alice is still there
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     assert_eq!(users.len(), 1);
    ///     Ok(())
    /// }).await?;
    /// ```
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
    ) -> TransactionBuilder<
        'a,
        Schema,
        SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
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
    ) -> TransactionBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteInitial, T>, DeleteInitial>
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
    pub async fn execute<'a, T>(&self, query: T) -> Result<u64, DrizzleError>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let (sql, params) = query.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        Ok(self.tx.execute(&sql, params).await?)
    }

    /// Runs a query and returns all matching rows within the transaction
    pub async fn all<'a, T, R>(&self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        self.rows(query).await?.collect().await
    }

    /// Runs a query and returns a row cursor within the transaction.
    pub async fn rows<'a, T, R>(&self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self.tx.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs a query and returns a single row within the transaction
    pub async fn get<'a, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let mut rows = self.tx.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Commits the transaction using libsql's SQL commands
    pub async fn commit(self) -> Result<(), DrizzleError> {
        Ok(self.tx.commit().await?)
    }

    /// Rolls back the transaction using libsql's SQL commands
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        Ok(self.tx.rollback().await?)
    }
}

#[cfg(feature = "libsql")]
impl<'a, S, Schema, State, Table, Mk, Rw>
    TransactionBuilder<'a, S, QueryBuilder<'a, Schema, State, Table, Mk, Rw>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        Ok(self.transaction.tx.execute(&sql, params).await?)
    }

    /// Runs the query and returns all matching rows, decoded as `R`.
    pub async fn all_as<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        self.rows_as::<R>().await?.collect().await
    }

    /// Runs the query and returns a row cursor, decoded as `R`.
    pub async fn rows_as<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self.transaction.tx.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row, decoded as `R`.
    pub async fn get_as<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let mut rows = self.transaction.tx.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();
        let mut rows = self.transaction.tx.query(&sql_str, params).await?;
        let mut decoded = Vec::new();
        while let Some(row) = rows.next().await? {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::libsql::Row,
                R,
            >>::decode(&row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a row cursor using the builder's row type.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        self.rows_as().await
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();
        let mut rows = self.transaction.tx.query(&sql_str, params).await?;
        if let Some(row) = rows.next().await? {
            <Mk as drizzle_core::row::DecodeSelectedRef<&::libsql::Row, R>>::decode(&row)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}
