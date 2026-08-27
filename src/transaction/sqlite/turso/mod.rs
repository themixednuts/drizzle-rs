use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use std::marker::PhantomData;
use turso::Row;

use crate::builder::sqlite::rows::TursoRows as Rows;
use crate::transaction::savepoint::{AsyncSavepointState, async_savepoint};

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

// `Transaction` derefs to `Connection`, so the compiled-program cache these
// helpers hit is the connection's and outlives the transaction. Turso caches
// the compiled program (not the live statement) and builds a fresh statement
// per call, so concurrent uses of the same SQL don't alias cursors.
async fn turso_transaction_execute_cached(
    tx: &turso::transaction::Transaction<'_>,
    sql: &str,
    params: Vec<turso::Value>,
) -> turso::Result<u64> {
    let mut statement = tx.prepare_cached(sql).await?;
    statement.execute(params).await
}

async fn turso_transaction_query_cached(
    tx: &turso::transaction::Transaction<'_>,
    sql: &str,
    params: Vec<turso::Value>,
) -> turso::Result<turso::Rows> {
    let mut statement = tx.prepare_cached(sql).await?;
    statement.query(params).await
}

/// Turso-specific transaction builder. See
/// `TransactionBuilder` for the
/// typestate-advancing methods; executor methods live below in this module.
pub type TransactionBuilder<'tx, 'conn, Schema, Builder, State> =
    crate::transaction::sqlite::typestate::TransactionBuilder<
        'tx,
        Transaction<'conn, Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::sqlite::turso::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!('conn);

/// Transaction wrapper that provides the same query building capabilities as Drizzle
#[derive(Debug)]
#[must_use = "transactions must be committed or rolled back"]
pub struct Transaction<'conn, Schema = ()> {
    tx: turso::transaction::Transaction<'conn>,
    tx_type: SQLiteTransactionType,
    savepoints: AsyncSavepointState,
    schema: Schema,
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(
        tx: turso::transaction::Transaction<'conn>,
        tx_type: SQLiteTransactionType,
        schema: Schema,
    ) -> Self {
        Self {
            tx,
            tx_type,
            savepoints: AsyncSavepointState::new(),
            schema,
        }
    }

    /// Gets a reference to the schema.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Gets a reference to the underlying transaction
    #[inline]
    pub const fn inner(&self) -> &turso::transaction::Transaction<'conn> {
        &self.tx
    }

    /// Gets the transaction type
    #[inline]
    pub const fn tx_type(&self) -> SQLiteTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    async fn execute_raw(&self, sql: &str) -> Result<(), DrizzleError> {
        self.savepoints.ensure_usable()?;
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
    /// ```no_run
    /// # use drizzle::sqlite::turso::Drizzle;
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::TransactionConfig;
    /// # use turso::Builder;
    /// # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct S { user: User }
    /// # #[tokio::main] async fn main() -> drizzle::Result<()> {
    /// # let db_builder = Builder::new_local(":memory:").build().await?;
    /// # let conn = db_builder.connect()?;
    /// # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
    /// db.transaction(TransactionConfig::Deferred, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///
    ///     let _: Result<(), _> = tx.savepoint(async |stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     }).await;
    ///
    ///     // Alice is still there
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     assert_eq!(users.len(), 1);
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
            &self.savepoints,
            |sql| async move { self.execute_raw(&sql).await },
            f(self),
        )
        .await
    }

    sqlite_transaction_constructors!('conn);

    /// Executes a raw query within the transaction
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the database call fails or the SQL is invalid.
    pub async fn execute<'q, T>(&self, query: T) -> Result<u64, DrizzleError>
    where
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.savepoints.ensure_usable()?;
        let query = query.to_sql();
        let (sql_str, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();

        Ok(turso_transaction_execute_cached(&self.tx, &sql_str, params).await?)
    }

    /// Runs a query and returns all matching rows within the transaction
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails or row decoding fails.
    pub async fn all<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.rows(query).await?.collect().await
    }

    /// Runs a query and returns a row cursor within the transaction.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails.
    pub async fn rows<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.savepoints.ensure_usable()?;
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();

        let rows = turso_transaction_query_cached(&self.tx, &sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs a query and returns a single row within the transaction
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails, no rows match (returns `DrizzleError::NotFound`), or decoding fails.
    pub async fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.savepoints.ensure_usable()?;
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();

        let mut rows = turso_transaction_query_cached(&self.tx, &sql_str, params).await?;

        crate::builder::sqlite::turso::turso_decode_first_and_finish(&mut rows, |row| {
            R::try_from(row).map_err(Into::into)
        })
        .await
    }

    /// Commits the transaction (turso transactions are auto-committed)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the commit call to the database fails.
    pub async fn commit(self) -> Result<(), DrizzleError> {
        if let Err(error) = self.savepoints.ensure_usable() {
            self.tx.rollback().await?;
            return Err(error);
        }
        Ok(self.tx.commit().await?)
    }

    /// Rolls back the transaction
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the rollback call to the database fails.
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        Ok(self.tx.rollback().await?)
    }
}

#[cfg(feature = "turso")]
impl<'tx, 'q, S, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, '_, S, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();

        Ok(turso_transaction_execute_cached(&self.runner.tx, &sql_str, params).await?)
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::turso::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::turso::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();
        let mut rows = turso_transaction_query_cached(&self.runner.tx, &sql_str, params).await?;
        let mut decoded = Vec::new();
        while let Some(row) = rows.next().await? {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::turso::Row,
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
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();

        let rows = turso_transaction_query_cached(&self.runner.tx, &sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::turso::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::turso::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<turso::Value> = params.into_iter().map(std::convert::Into::into).collect();
        let mut rows = turso_transaction_query_cached(&self.runner.tx, &sql_str, params).await?;
        crate::builder::sqlite::turso::turso_decode_first_and_finish(&mut rows, |row| {
            <Mk as drizzle_core::row::DecodeSelectedRef<&::turso::Row, R>>::decode(row)
        })
        .await
    }
}

// =============================================================================
// Query API: transaction-scoped find_many / find_first
// =============================================================================

#[cfg(feature = "query")]
use crate::builder::sqlite::common;

#[cfg(feature = "query")]
impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a relational query builder scoped to this transaction.
    ///
    /// Rows read here observe the transaction's uncommitted state.
    pub fn query<'a, T>(&self, _table: T) -> common::DrizzleQueryBuilder<'_, 'a, &Self, Schema, T>
    where
        T: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            _schema: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = ::turso::Connection;
}

// AllColumns: read base from individual row columns via TryFrom<Row>
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        Cl,
    >
{
    /// Executes the query and returns all matching rows with their relations.
    pub async fn find_many(
        self,
    ) -> drizzle_core::error::Result<
        Vec<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::Select,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        self.runner.savepoints.ensure_usable()?;
        crate::builder::sqlite::turso::relational_find_many(self.runner.inner(), self.builder).await
    }
}

// AllColumns find_first: requires no LIMIT set yet (internally adds LIMIT 1)
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Executes the query and returns the first matching row, or `None`.
    pub async fn find_first(
        self,
    ) -> drizzle_core::error::Result<
        Option<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::Select,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

// PartialColumns: read base from a single JSON "__base" column via FromJsonObject
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    >
{
    /// Executes the query and returns all matching rows with their relations.
    ///
    /// Base columns are deserialized from a JSON `"__base"` column.
    pub async fn find_many(
        self,
    ) -> drizzle_core::error::Result<
        Vec<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::PartialSelect>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        self.runner.savepoints.ensure_usable()?;
        crate::builder::sqlite::turso::relational_find_many_partial(
            self.runner.inner(),
            self.builder,
        )
        .await
    }
}

// PartialColumns find_first: requires no LIMIT set yet
#[cfg(feature = "query")]
impl<'db, 'a, 'conn, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<'conn, Schema>,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Executes the query and returns the first matching row, or `None`.
    pub async fn find_first(
        self,
    ) -> drizzle_core::error::Result<
        Option<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::PartialSelect>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}
