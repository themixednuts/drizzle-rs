use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio_postgres::{Row, Transaction as TokioPgTransaction};

pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

/// Tokio-postgres-specific transaction builder
#[derive(Debug)]
pub struct TransactionBuilder<'a, 'conn, Schema, Builder, State> {
    transaction: &'a Transaction<'conn, Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<TokioPgTransaction<'conn>>>,
    tx_type: PostgresTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
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
    pub(crate) fn new(
        tx: TokioPgTransaction<'conn>,
        tx_type: PostgresTransactionType,
        schema: Schema,
    ) -> Self {
        Self {
            tx: RefCell::new(Some(tx)),
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

    /// Gets the transaction type
    #[inline]
    pub fn tx_type(&self) -> PostgresTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    async fn execute_raw(&self, sql: &str) -> drizzle_core::error::Result<()> {
        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");
        tx.execute(sql, &[])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
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
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::tokio::Drizzle;
    /// # use drizzle::postgres::common::PostgresTransactionType;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # #[tokio::main] async fn main() -> drizzle::Result<()> {
    /// # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
    /// # tokio::spawn(async move { conn.await.unwrap() });
    /// # let (mut db, S { user }) = Drizzle::new(client, S::new());
    /// db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///
    ///     // This savepoint fails — only its changes roll back
    ///     let _: Result<(), _> = tx.savepoint(async |stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     }).await;
    ///
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     assert_eq!(users.len(), 1); // only Alice
    ///     Ok(())
    /// }).await?;
    /// # Ok(()) }
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

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, tokio_postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let query_sql = query.to_sql();
        let (sql, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx.execute");
            let (sql, params) = query_sql.build();
            drizzle_core::drizzle_trace_query!(&sql, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql, param_refs)
        };

        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");
        tx.execute(&sql, &param_refs[..]).await
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        let sql = query.to_sql();
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx.all");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in rows {
            decoded.push(R::try_from(&row).map_err(Into::into)?);
        }

        Ok(decoded.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx.get");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Commits the transaction
    pub(crate) async fn commit(&self) -> drizzle_core::error::Result<()> {
        let tx = self
            .tx
            .borrow_mut()
            .take()
            .expect("Transaction already consumed");
        tx.commit()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Rolls back the transaction
    pub(crate) async fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx = self
            .tx
            .borrow_mut()
            .take()
            .expect("Transaction already consumed");
        tx.rollback()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }
}

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

impl<'a, 'conn, S, Schema, State, Table>
    TransactionBuilder<'a, 'conn, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx_builder.execute");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let tx_ref = self.transaction.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");

        Ok(tx
            .execute(&sql_str, &param_refs[..])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx_builder.all");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let tx_ref = self.transaction.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in rows {
            decoded.push(R::try_from(&row).map_err(Into::into)?);
        }

        Ok(decoded.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx_builder.get");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let tx_ref = self.transaction.tx.borrow();
        let tx = tx_ref.as_ref().expect("Transaction already consumed");

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }
}

impl<'a, 'conn, S, T, State> ToSQL<'a, PostgresValue<'a>>
    for TransactionBuilder<'a, 'conn, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}
