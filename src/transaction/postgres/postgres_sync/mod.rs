use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use postgres::fallible_iterator::FallibleIterator;
use postgres::{Row, Transaction as PgTransaction};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;

use crate::transaction::savepoint::sync_savepoint;

/// Returns an error indicating the transaction has already been consumed.
fn tx_consumed_error() -> DrizzleError {
    DrizzleError::TransactionError("Transaction already consumed".into())
}

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::transaction::{IsolationLevel, TransactionConfig};
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

#[cfg(feature = "query")]
use crate::builder::postgres::common;
use crate::builder::postgres::postgres_sync::{
    Rows, postgres_sync_materialize_params as materialize_params, prepared::StatementCache,
};
#[cfg(feature = "query")]
use drizzle_core::query::{DeserializeStore, FromJsonObject as _};

/// `postgres_sync`-specific transaction builder.
///
/// This is a thin type alias over the dialect-shared
/// `TransactionBuilder`; every
/// typestate-advancing method (`.value`/`.values`/`.r#where`/`.set`/
/// `.on_conflict`/`.returning`/`.from`/`.join`/etc.) lives on the generic
/// struct over there. Executor methods (`.execute`/`.all`/`.rows`/`.get`)
/// — the only parts that need `postgres::Transaction`-specific access —
/// stay below in this module.
pub type TransactionBuilder<'tx, 'conn, Schema, Builder, State> =
    crate::transaction::postgres::typestate::TransactionBuilder<
        'tx,
        &'tx Transaction<'conn, Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::postgres::postgres_sync::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!('conn);

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<PgTransaction<'conn>>>,
    config: TransactionConfig,
    savepoint_depth: AtomicU32,
    schema: Schema,
    client_id: u64,
    statement_cache: StatementCache,
}

impl<Schema> std::fmt::Debug for Transaction<'_, Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("config", &self.config)
            .field("is_active", &self.tx.borrow().is_some())
            .finish()
    }
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) const fn new(
        tx: PgTransaction<'conn>,
        config: TransactionConfig,
        schema: Schema,
        client_id: u64,
        statement_cache: StatementCache,
    ) -> Self {
        Self {
            tx: RefCell::new(Some(tx)),
            config,
            savepoint_depth: AtomicU32::new(0),
            schema,
            client_id,
            statement_cache,
        }
    }

    /// Resolves a cached `Statement` for a query running inside this transaction.
    fn cached_statement(
        &self,
        tx: &mut PgTransaction<'_>,
        sql: &str,
        param_types: &[postgres::types::Type],
    ) -> Result<postgres::Statement, postgres::Error> {
        self.statement_cache
            .transaction_statement(self.client_id, tx, sql, param_types)
    }

    /// Gets a reference to the schema.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Legacy isolation view.
    ///
    /// This cannot distinguish server-default isolation from explicit
    /// `READ COMMITTED`. Use [`Self::config`] when that distinction matters.
    #[deprecated(since = "0.1.17", note = "use config()")]
    #[inline]
    pub const fn tx_type(&self) -> PostgresTransactionType {
        match self.config.isolation() {
            None | Some(IsolationLevel::ReadCommitted) => PostgresTransactionType::ReadCommitted,
            Some(IsolationLevel::ReadUncommitted) => PostgresTransactionType::ReadUncommitted,
            Some(IsolationLevel::RepeatableRead) => PostgresTransactionType::RepeatableRead,
            Some(IsolationLevel::Serializable) => PostgresTransactionType::Serializable,
        }
    }

    /// Gets the configuration used to begin this transaction.
    #[inline]
    pub const fn config(&self) -> TransactionConfig {
        self.config
    }

    /// Executes a raw SQL string with no parameters.
    fn execute_raw(&self, sql: &str) -> drizzle_core::error::Result<()> {
        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        tx.execute(sql, &[]).map_err(DrizzleError::from)?;
        Ok(())
    }

    /// Executes a nested savepoint within this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released.
    /// If it returns `Err` or panics, the savepoint is rolled back.
    /// The outer transaction is unaffected either way.
    ///
    /// Savepoints can be nested — each level gets its own savepoint name.
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # use drizzle::postgres::TransactionConfig;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
    /// # let (mut db, S { user }) = Drizzle::new(client, S::new());
    /// db.transaction(TransactionConfig::default(), |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///
    ///     // This savepoint fails — only its changes roll back
    ///     let _: Result<(), _> = tx.savepoint(|stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute()?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     });
    ///
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     assert_eq!(users.len(), 1); // only Alice
    ///     Ok(())
    /// })?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the savepoint cannot be created/released, or the inner closure returns an error.
    pub fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        sync_savepoint(
            &self.savepoint_depth,
            |sql| self.execute_raw(sql),
            || f(self),
        )
    }

    postgres_transaction_constructors!('conn);

    /// Execute a statement within the transaction and return the number of affected rows.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the database call fails or the SQL is invalid.
    pub fn execute<'q, T>(&self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute");
        let query_sql = query.to_sql();
        let (sql, params) = query_sql.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());

        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute.db");
        let statement = self
            .cached_statement(tx, &sql, &param_types)
            .map_err(DrizzleError::from)?;
        Ok(tx
            .execute(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails or row decoding fails.
    pub fn all<'q, T, R, C>(&self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails.
    pub fn rows<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.all");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.all.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        let statement = self
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(DrizzleError::from)?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails, no rows match, or decoding fails.
    pub fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, PostgresValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.get");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.get.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        let statement = self
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(DrizzleError::from)?;
        let row = tx
            .query_one(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Creates a relational query builder scoped to this transaction.
    ///
    /// Rows read here observe the transaction's uncommitted state.
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

    #[cfg(feature = "query")]
    fn relational_rows<'q>(
        &self,
        query: &drizzle_core::sql::SQL<'q, PostgresValue<'q>>,
    ) -> drizzle_core::error::Result<Vec<Row>> {
        let (sql, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());

        let (param_types, param_refs) = materialize_params(&params);
        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        let statement = self
            .cached_statement(tx, &sql, &param_types)
            .map_err(DrizzleError::from)?;

        tx.query(&statement, &param_refs[..])
            .map_err(DrizzleError::from)
    }

    /// Commits the transaction
    pub(crate) fn commit(self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.commit().map_err(DrizzleError::from)
    }

    /// Rolls back the transaction
    pub(crate) fn rollback(self) -> drizzle_core::error::Result<()> {
        let tx = self.tx.borrow_mut().take().ok_or_else(tx_consumed_error)?;
        tx.rollback().map_err(DrizzleError::from)
    }
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = postgres::Client;
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
    pub fn find_many(
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
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
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
        let rows = self.runner.relational_rows(&query_sql)?;
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
    pub fn find_first(
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
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
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
    pub fn find_many(
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
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
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
        let rows = self.runner.relational_rows(&query_sql)?;
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
    pub fn find_first(
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
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
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
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute.db");
        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(DrizzleError::from)?;
        Ok(tx
            .execute(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?)
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(DrizzleError::from)?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a lazy row cursor using the builder's row type.
    pub fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.rows");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.rows.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;

        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(DrizzleError::from)?;
        let rows = tx
            .query(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get.param_refs");
        let (param_types, param_refs) = materialize_params(&params);

        let mut tx_ref = self.runner.tx.borrow_mut();
        let tx = tx_ref.as_mut().ok_or_else(tx_consumed_error)?;
        let statement = self
            .runner
            .cached_statement(tx, &sql_str, &param_types)
            .map_err(DrizzleError::from)?;
        let row = tx
            .query_one(&statement, &param_refs[..])
            .map_err(DrizzleError::from)?;

        <Mk as drizzle_core::row::DecodeSelectedRef<&::postgres::Row, R>>::decode(&row)
    }
}

// `ToSQL for TransactionBuilder` is now provided by the shared `DrizzleBuilder`
// impl in `crate::builder::postgres::common`.
