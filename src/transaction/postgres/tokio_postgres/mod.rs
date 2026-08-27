use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use std::cell::RefCell;
use std::marker::PhantomData;
use tokio_postgres::{Row, Transaction as TokioPgTransaction};

use crate::builder::postgres::tokio_postgres::prepared::ClientStatementCache;
use crate::transaction::savepoint::{AsyncSavepointState, async_savepoint};

#[cfg(feature = "query")]
use crate::builder::postgres::common;

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

use crate::builder::postgres::tokio_postgres::tokio_postgres_materialize_params as materialize_params;

/// `tokio_postgres`-specific transaction builder. See
/// `TransactionBuilder` for the
/// typestate-advancing methods; executor methods live below.
pub type TransactionBuilder<'tx, 'conn, Schema, Builder, State> =
    crate::transaction::postgres::typestate::TransactionBuilder<
        'tx,
        &'tx Transaction<'conn, Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::postgres::tokio_postgres::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!('conn);

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<TokioPgTransaction<'conn>>>,
    tx_type: PostgresTransactionType,
    savepoints: AsyncSavepointState,
    schema: Schema,
    statement_cache: ClientStatementCache,
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
    pub(crate) fn new(
        tx: TokioPgTransaction<'conn>,
        tx_type: PostgresTransactionType,
        schema: Schema,
        statement_cache: ClientStatementCache,
    ) -> Self {
        Self {
            tx: RefCell::new(Some(tx)),
            tx_type,
            savepoints: AsyncSavepointState::new(),
            schema,
            statement_cache,
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
        self.savepoints.ensure_usable()?;
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
            &self.savepoints,
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
        self.savepoints.ensure_usable()?;
        let query_sql = query.to_sql();
        let (sql, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx.execute");
            let (sql, params) = query_sql.build();
            drizzle_core::drizzle_trace_query!(&sql, params.len());
            (sql, params)
        };
        let (param_types, param_refs) = materialize_params(&params);

        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .statement_cache
            .transaction_statement(tx, &sql, &param_types)
            .await
            .map_err(DrizzleError::from)?;
        Ok(tx
            .execute(&statement, &param_refs[..])
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
        self.savepoints.ensure_usable()?;
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx.all");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };
        let (param_types, param_refs) = materialize_params(&params);

        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .statement_cache
            .transaction_statement(tx, &sql_str, &param_types)
            .await
            .map_err(DrizzleError::from)?;

        let rows = tx
            .query(&statement, &param_refs[..])
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
        self.savepoints.ensure_usable()?;
        let sql = query.to_sql();
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx.get");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };
        let (param_types, param_refs) = materialize_params(&params);

        let tx_ref = self.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .statement_cache
            .transaction_statement(tx, &sql_str, &param_types)
            .await
            .map_err(DrizzleError::from)?;

        let row = tx
            .query_one(&statement, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Creates a relational query builder scoped to this transaction.
    #[cfg(feature = "query")]
    pub fn query<'a, T>(&self, _table: T) -> common::DrizzleQueryBuilder<'_, 'a, &Self, Schema, T>
    where
        T: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            _schema: PhantomData,
        }
    }

    /// Commits the transaction
    pub(crate) async fn commit(&self) -> drizzle_core::error::Result<()> {
        if let Err(error) = self.savepoints.ensure_usable() {
            let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
            tx.rollback().await.map_err(DrizzleError::from)?;
            return Err(error);
        }
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.commit().await.map_err(DrizzleError::from)
    }

    /// Rolls back the transaction
    pub(crate) async fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.rollback().await.map_err(DrizzleError::from)
    }
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = tokio_postgres::Client;
}

// =============================================================================
// Relational Query API
// =============================================================================

#[cfg(feature = "query")]
use drizzle_core::query::{DeserializeStore, FromJsonObject as _};

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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: DeserializeStore,
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

        let (param_types, param_refs) = materialize_params(&bind_params);
        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .statement_cache
            .transaction_statement(tx, &sql, &param_types)
            .await
            .map_err(DrizzleError::from)?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: DeserializeStore,
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
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: DeserializeStore,
    {
        self.runner.savepoints.ensure_usable()?;

        let builder = self.builder;
        let column_names = &builder.cols.columns;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = column_names.clone();
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

        let (param_types, param_refs) = materialize_params(&bind_params);
        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .statement_cache
            .transaction_statement(tx, &sql, &param_types)
            .await
            .map_err(DrizzleError::from)?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            let base_json: String = row.get(0);
            let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
                &base_json, "base",
            )?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
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
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
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
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx_builder.execute");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };
        let (param_types, param_refs) = materialize_params(&params);

        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .statement_cache
            .transaction_statement(tx, &sql_str, &param_types)
            .await
            .map_err(DrizzleError::from)?;

        Ok(tx
            .execute(&statement, &param_refs[..])
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
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx_builder.all");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };
        let (param_types, param_refs) = materialize_params(&params);

        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .statement_cache
            .transaction_statement(tx, &sql_str, &param_types)
            .await
            .map_err(DrizzleError::from)?;
        let rows = tx
            .query(&statement, &param_refs[..])
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
        self.runner.savepoints.ensure_usable()?;
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "tx_builder.get");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());
            (sql_str, params)
        };
        let (param_types, param_refs) = materialize_params(&params);

        let tx_ref = self.runner.tx.borrow();
        let tx = tx_ref.as_ref().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .statement_cache
            .transaction_statement(tx, &sql_str, &param_types)
            .await
            .map_err(DrizzleError::from)?;
        let row = tx
            .query_one(&statement, &param_refs[..])
            .await
            .map_err(DrizzleError::from)?;

        <Mk as drizzle_core::row::DecodeSelectedRef<&::tokio_postgres::Row, R>>::decode(&row)
    }
}

// `ToSQL for TransactionBuilder` is now provided by the shared `DrizzleBuilder`
// impl in `crate::builder::postgres::common`.
