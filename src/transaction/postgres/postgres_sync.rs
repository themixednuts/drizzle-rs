use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use postgres::{Row, Transaction as PgTransaction};
use std::cell::RefCell;
use std::marker::PhantomData;

pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

use drizzle_postgres::{
    PostgresTransactionType, PostgresValue, ToPostgresSQL,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};

/// Postgres-specific transaction builder
#[derive(Debug)]
pub struct TransactionBuilder<'a, 'conn, Schema, Builder, State> {
    transaction: &'a Transaction<'conn, Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<PgTransaction<'conn>>>,
    tx_type: PostgresTransactionType,
    _schema: PhantomData<Schema>,
}

impl<'conn, Schema> std::fmt::Debug for Transaction<'conn, Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("tx_type", &self.tx_type)
            .field("is_active", &self.tx.borrow().is_some())
            .finish()
    }
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(tx: PgTransaction<'conn>, tx_type: PostgresTransactionType) -> Self {
        Self {
            tx: RefCell::new(Some(tx)),
            tx_type,
            _schema: PhantomData,
        }
    }

    /// Gets the transaction type
    #[inline]
    pub fn tx_type(&self) -> PostgresTransactionType {
        self.tx_type
    }

    /// Creates a SELECT query builder within the transaction
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'b, Schema, SelectInitial>,
        SelectInitial,
    >
    where
        T: ToSQL<'b, PostgresValue<'b>>,
    {
        use drizzle_postgres::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an INSERT query builder within the transaction
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertInitial, Table>,
        InsertInitial,
    >
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an UPDATE query builder within the transaction
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates a DELETE query builder within the transaction
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        DeleteBuilder<'a, Schema, DeleteInitial, T>,
        DeleteInitial,
    >
    where
        T: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression) within the transaction
    pub fn with<'a, C>(
        &'a self,
        cte: C,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn execute<'a, T>(&'a self, query: T) -> Result<u64, postgres::Error>
    where
        T: ToPostgresSQL<'a>,
    {
        let query_sql = query.to_sql();
        let sql = query_sql.sql();
        let params = query_sql.params();

        // Convert PostgresValue to &dyn ToSql
        let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = params
            .map(|p| p as &(dyn postgres::types::ToSql + Sync))
            .collect();

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");
        tx.execute(&sql, &param_refs[..])
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
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

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let results = rows
            .iter()
            .map(|row| R::try_from(row).map_err(Into::into))
            .collect::<Result<C, _>>()?;

        Ok(results)
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
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

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Commits the transaction
    pub(crate) fn commit(&self) -> drizzle_core::error::Result<()> {
        let tx = self
            .tx
            .borrow_mut()
            .take()
            .expect("Transaction already consumed");
        tx.commit()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Rolls back the transaction
    pub(crate) fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx = self
            .tx
            .borrow_mut()
            .take()
            .expect("Transaction already consumed");
        tx.rollback()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }
}

// CTE (WITH) Builder Implementation for Transaction
impl<'a, 'conn, Schema>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectInitial>,
        SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.select(query);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(
        self,
        cte: C,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

// Postgres-specific execution methods for all ExecutableState QueryBuilders in Transaction
impl<'a, 'conn, S, Schema, State, Table>
    TransactionBuilder<'a, 'conn, S, QueryBuilder<'a, Schema, State, Table>, State>
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

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        Ok(tx
            .execute(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?)
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

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

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

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }
}

impl<'a, 'conn, S, T, State> ToSQL<'a, PostgresValue<'a>>
    for TransactionBuilder<'a, 'conn, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}
