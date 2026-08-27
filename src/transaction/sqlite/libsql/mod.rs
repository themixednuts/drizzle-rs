use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use libsql::Row;
use std::marker::PhantomData;
use std::ops::Deref;

use crate::builder::sqlite::rows::LibsqlRows as Rows;
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

/// LibSQL-specific transaction builder. See
/// `TransactionBuilder` for the
/// typestate-advancing methods; executor methods live below in this module.
pub type TransactionBuilder<'tx, Schema, Builder, State> =
    crate::transaction::sqlite::typestate::TransactionBuilder<
        'tx,
        Transaction<Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::sqlite::libsql::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!();

/// Transaction wrapper that provides the same query building capabilities as Drizzle
#[must_use = "transactions must be committed or rolled back"]
pub struct Transaction<Schema = ()> {
    tx: libsql::Transaction,
    tx_type: SQLiteTransactionType,
    savepoints: AsyncSavepointState,
    schema: Schema,
}

/// Explicit transaction that keeps its originating `Drizzle` handle borrowed.
#[must_use = "transactions must be committed or rolled back"]
pub struct TransactionGuard<'db, Schema = ()> {
    transaction: Transaction<Schema>,
    borrow: PhantomData<&'db mut ()>,
}

impl<Schema> TransactionGuard<'_, Schema> {
    pub(crate) const fn new(transaction: Transaction<Schema>) -> Self {
        Self {
            transaction,
            borrow: PhantomData,
        }
    }

    /// Commits the transaction.
    pub async fn commit(self) -> Result<(), DrizzleError> {
        self.transaction.commit().await
    }

    /// Rolls back the transaction.
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        self.transaction.rollback().await
    }
}

impl<Schema> Deref for TransactionGuard<'_, Schema> {
    type Target = Transaction<Schema>;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl<Schema> std::fmt::Debug for TransactionGuard<'_, Schema> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.transaction.fmt(formatter)
    }
}

impl<Schema> std::fmt::Debug for Transaction<Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("tx_type", &self.tx_type)
            .field("savepoints", &self.savepoints)
            .finish_non_exhaustive()
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
    pub const fn inner(&self) -> &libsql::Transaction {
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
    /// ```rust
    /// # let _ = r####"
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::libsql::Drizzle;
    /// # use drizzle::sqlite::TransactionConfig;
    /// db.transaction(TransactionConfig::Deferred, async |tx| {
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
    /// # "####;
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

    sqlite_transaction_constructors!();

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
        let (sql, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        Ok(self.tx.execute(&sql, params).await?)
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
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        let rows = self.tx.query(&sql_str, params).await?;
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
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        let mut rows = self.tx.query(&sql_str, params).await?;

        rows.next().await?.map_or_else(
            || Err(DrizzleError::NotFound),
            |row| R::try_from(&row).map_err(Into::into),
        )
    }

    /// Commits the transaction using libsql's SQL commands
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

    /// Rolls back the transaction using libsql's SQL commands
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the rollback call to the database fails.
    pub async fn rollback(self) -> Result<(), DrizzleError> {
        Ok(self.tx.rollback().await?)
    }
}

#[cfg(feature = "libsql")]
impl<'tx, 'q, S, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, S, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.runner.savepoints.ensure_usable()?;
        let (sql, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        Ok(self.runner.tx.execute(&sql, params).await?)
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::libsql::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();
        let mut rows = self.runner.tx.query(&sql_str, params).await?;
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
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        let rows = self.runner.tx.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::libsql::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();
        let mut rows = self.runner.tx.query(&sql_str, params).await?;
        rows.next().await?.map_or_else(
            || Err(DrizzleError::NotFound),
            |row| <Mk as drizzle_core::row::DecodeSelectedRef<&::libsql::Row, R>>::decode(&row),
        )
    }
}

// =============================================================================
// Query API: transaction-scoped find_many / find_first
// =============================================================================

#[cfg(feature = "query")]
use crate::builder::sqlite::common;

#[cfg(feature = "query")]
impl<Schema> Transaction<Schema> {
    /// Creates a relational query builder scoped to this transaction.
    ///
    /// Rows read here observe the transaction's uncommitted state. Unlike the
    /// database handle's relational queries, statements are not cached — a
    /// transaction is short-lived by design.
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
impl<Schema> common::RelationalPreparedDriver for &Transaction<Schema> {
    type PreparedDriver = ::libsql::Connection;
}

// AllColumns: read base from individual row columns via TryFrom<Row>
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<Schema>,
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::libsql::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::libsql::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        self.runner.savepoints.ensure_usable()?;

        let num_base_cols = T::COLUMN_NAMES.len();
        let builder = self.builder;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE,
            T::COLUMN_NAMES,
            T::BLOB_COLUMNS,
            T::JSON_PROJECTIONS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            false,
        );
        let (sql, bind_params) = query_sql.build();
        drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

        let params: Vec<libsql::Value> = bind_params
            .iter()
            .copied()
            .map(std::convert::Into::into)
            .collect();
        let mut raw_rows = self.runner.inner().query(&sql, params).await?;
        let mut results = Vec::new();

        while let Some(row) = raw_rows.next().await? {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(&row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let idx = i32::try_from(rel_col).map_err(|_| {
                    drizzle_core::error::DrizzleError::Other("column index overflow".into())
                })?;
                let json = row
                    .get::<Option<String>>(idx)
                    .map_err(drizzle_core::error::DrizzleError::from)?;
                rel_col += 1;
                Ok(json)
            };
            let store = <<Rels as drizzle_core::query::BuildStore>::Store as
                drizzle_core::query::DeserializeStore>::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
    }
}

// AllColumns find_first: requires no LIMIT set yet (internally adds LIMIT 1)
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<Schema>,
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::libsql::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::libsql::Row>>::Error:
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
impl<'db, 'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<Schema>,
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

        let builder = self.builder;
        let column_names = &builder.cols.columns;
        let col_refs: Vec<&str> = column_names.clone();
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE,
            &col_refs,
            T::BLOB_COLUMNS,
            T::JSON_PROJECTIONS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            true,
        );
        let (sql, bind_params) = query_sql.build();
        drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

        let params: Vec<libsql::Value> = bind_params
            .iter()
            .copied()
            .map(std::convert::Into::into)
            .collect();
        let mut raw_rows = self.runner.inner().query(&sql, params).await?;
        let mut results = Vec::new();

        while let Some(row) = raw_rows.next().await? {
            // Column 0 is the JSON "__base" object
            let base_json: String = row
                .get::<String>(0)
                .map_err(drizzle_core::error::DrizzleError::from)?;
            let base = <<T as drizzle_core::query::QueryTable>::PartialSelect as
                drizzle_core::query::FromJsonObject>::from_json_str(&base_json, "base")?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let idx = i32::try_from(rel_col).map_err(|_| {
                    drizzle_core::error::DrizzleError::Other("column index overflow".into())
                })?;
                let json = row
                    .get::<Option<String>>(idx)
                    .map_err(drizzle_core::error::DrizzleError::from)?;
                rel_col += 1;
                Ok(json)
            };
            let store = <<Rels as drizzle_core::query::BuildStore>::Store as
                drizzle_core::query::DeserializeStore>::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
    }
}

// PartialColumns find_first: requires no LIMIT set yet
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Transaction<Schema>,
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
