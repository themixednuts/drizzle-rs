use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;
use tokio_postgres::{Row, Transaction as TokioPgTransaction};

use crate::transaction::savepoint::async_savepoint;

/// Returns an error indicating the transaction has already been consumed.
fn tx_consumed_error() -> DrizzleError {
    DrizzleError::TransactionError("Transaction already consumed".into())
}

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

/// `tokio_postgres`-specific transaction builder. See
/// [`crate::transaction::postgres::typestate::TransactionBuilder`] for the
/// typestate-advancing methods; executor methods live below.
pub type TransactionBuilder<'tx, 'conn, Schema, Builder, State> =
    crate::transaction::postgres::typestate::TransactionBuilder<
        'tx,
        &'tx Transaction<'conn, Schema>,
        Schema,
        Builder,
        State,
    >;

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<TokioPgTransaction<'conn>>>,
    tx_type: PostgresTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<Schema> std::fmt::Debug for Transaction<'_, Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("tx_type", &self.tx_type)
            .field("is_active", &self.tx.borrow().is_some())
            .finish()
    }
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) const fn new(
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
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Gets the transaction type
    #[inline]
    pub const fn tx_type(&self) -> PostgresTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    async fn execute_raw(&self, sql: &str) -> drizzle_core::error::Result<()> {
        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        tx.execute(sql, &[]).await.map_err(DrizzleError::from)?;
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
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the savepoint cannot be created/released, or the inner closure returns an error.
    pub async fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: AsyncFnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        async_savepoint(
            &self.savepoint_depth,
            |sql| async move { self.execute_raw(&sql).await },
            f(self),
        )
        .await
    }

    postgres_transaction_constructors!('conn);

    /// Execute a statement within the transaction and return the number of affected rows.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the database call fails or the SQL is invalid.
    pub async fn execute<'q, T>(&self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'q, PostgresValue<'q>>,
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
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        Ok(tx
            .execute(&sql, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails or row decoding fails.
    pub async fn all<'q, T, R, C>(&self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
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
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in rows {
            decoded.push(R::try_from(&row).map_err(Into::into)?);
        }

        Ok(decoded.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails, no rows match, or decoding fails.
    pub async fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
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
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Commits the transaction
    pub(crate) async fn commit(&self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.commit().await.map_err(DrizzleError::from)
    }

    /// Rolls back the transaction
    pub(crate) async fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.rollback().await.map_err(DrizzleError::from)
    }
}

// `TransactionBuilder<CTEInit>::select` and `.with` are now provided by
// the shared `DrizzleBuilder` typestate impls (see
// `crate::builder::postgres::common`).

impl<'tx, 'q, S, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, '_, S, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
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

        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;

        Ok(tx
            .execute(&sql_str, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?)
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::tokio_postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::tokio_postgres::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
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

        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let rows = tx
            .query(&sql_str, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::tokio_postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::tokio_postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::tokio_postgres::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
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

        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;

        <Mk as drizzle_core::row::DecodeSelectedRef<&::tokio_postgres::Row, R>>::decode(&row)
    }
}

// `ToSQL for TransactionBuilder` is now provided by the shared `DrizzleBuilder`
// impl in `crate::builder::postgres::common`.
