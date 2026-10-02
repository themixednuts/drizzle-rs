use crate::builder::sqlite::rows::LibsqlRows as Rows;
use crate::transaction::savepoint::{AsyncSavepointState, async_savepoint};
use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use libsql::Row;
use std::marker::PhantomData;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

/// A query being built inside a [`Transaction`]. It has the same clause
/// methods as the connection's builder; run it with `.execute()`, `.all()`,
/// `.get()`, or `.rows()`.
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

/// An open libsql transaction, passed to the closure given to `transaction`.
///
/// It has the same query methods as the database handle (`select`,
/// `insert`, `update`, `delete`, `with`), plus `savepoint` for nested
/// rollback points. It commits or rolls back when the closure returns.
///
/// If a savepoint future is cancelled or its cleanup fails, every later
/// query on the transaction returns `DrizzleError::TransactionError`.
pub struct Transaction<Schema = ()> {
    tx: libsql::Transaction,
    tx_type: SQLiteTransactionType,
    savepoints: AsyncSavepointState,
    schema: Schema,
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
    /// Wraps a driver transaction that has already begun.
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

    /// Returns the schema value the database handle was created with.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the driver's transaction, for calls drizzle does not cover,
    /// such as running a prepared statement inside this transaction.
    #[inline]
    pub const fn inner(&self) -> &libsql::Transaction {
        &self.tx
    }

    /// Returns the mode the transaction was started with (`DEFERRED`,
    /// `IMMEDIATE`, or `EXCLUSIVE`).
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

    /// Runs `f` inside a savepoint nested in this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released.
    /// If it returns `Err`, the savepoint is rolled back.
    /// The outer transaction is unaffected either way.
    ///
    /// Savepoints can be nested. Each level gets its own savepoint name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::libsql::Drizzle;
    /// # use drizzle::sqlite::TransactionConfig;
    /// # #[SQLiteTable]
    /// # struct User {
    /// #     #[column(PRIMARY, AUTOINCREMENT)]
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// # #[derive(SQLiteSchema)]
    /// # struct AppSchema { user: User }
    /// # async fn example(db: &Drizzle<AppSchema>, user: User) -> drizzle::Result<()> {
    /// db.transaction(TransactionConfig::Deferred, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///
    ///     let _ = tx.savepoint(async |stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
    ///         Err::<(), _>(drizzle::error::DrizzleError::Other("oops".into()))
    ///     }).await;
    ///
    ///     // Alice is still there
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     assert_eq!(users.len(), 1);
    ///     Ok(())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or an error when `SAVEPOINT`, `RELEASE`, or
    /// `ROLLBACK TO` fails. When cleanup after an `Err` also fails, both
    /// errors are reported together.
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

    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, inside
    /// the transaction and returns the number of rows it changed.
    ///
    /// Prefer the builder's own `.execute()`. This method skips the builder's
    /// compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when libsql cannot prepare or run the statement.
    pub async fn execute<'q, T>(&self, query: T) -> Result<u64, DrizzleError>
    where
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.savepoints.ensure_usable()?;
        let query = query.to_sql();
        let (sql, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        Ok(crate::builder::sqlite::libsql::execute_sql(
            &self.tx,
            &sql,
            params,
            query.has_returning(),
        )
        .await?)
    }

    /// Runs any SQL value inside the transaction and decodes every row into
    /// `R` with `TryFrom<&Row>`, skipping the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when libsql cannot prepare or run the query, or when a
    /// row cannot be decoded into `R`.
    pub async fn all<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.rows(query).await?.collect().await
    }

    /// Runs any SQL value inside the transaction and returns its rows, fetched
    /// and decoded into `R` as you call `next().await`.
    ///
    /// # Errors
    ///
    /// Returns an error when libsql cannot prepare or run the query. Decoding
    /// errors surface per row.
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

    /// Runs any SQL value inside the transaction and decodes its first row
    /// into `R`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and an error
    /// when libsql cannot prepare or run the query or the row cannot be decoded
    /// into `R`.
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

    /// Commits the transaction.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the commit call to the database fails.
    pub(crate) async fn commit(self) -> Result<(), DrizzleError> {
        if let Err(error) = self.savepoints.ensure_usable() {
            self.tx.rollback().await?;
            return Err(error);
        }
        Ok(self.tx.commit().await?)
    }

    /// Rolls back the transaction.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the rollback call to the database fails.
    pub(crate) async fn rollback(self) -> Result<(), DrizzleError> {
        Ok(self.tx.rollback().await?)
    }
}

#[cfg(feature = "libsql")]
impl<'tx, 'q, S, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, S, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the statement inside the transaction and returns the number of rows
    /// it changed.
    ///
    /// # Errors
    ///
    /// Returns an error when libsql cannot prepare or run the statement, for
    /// example on a constraint violation.
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.runner.savepoints.ensure_usable()?;
        let (sql, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());
        let params: Vec<libsql::Value> = params.into_iter().map(std::convert::Into::into).collect();

        Ok(crate::builder::sqlite::libsql::execute_sql(
            &self.runner.tx,
            &sql,
            params,
            self.builder.sql.has_returning(),
        )
        .await?)
    }

    /// Runs the query inside the transaction and decodes every row into `R`.
    ///
    /// Reads see the transaction's own uncommitted writes.
    ///
    /// # Errors
    ///
    /// Returns an error when libsql cannot prepare or run the query, or when a
    /// row cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The call does not compile unless every column the query reads belongs to
    /// a table in its `FROM`/`JOIN` list, `R` matches the selection (with
    /// `Option<T>` wherever a value can be `NULL`), and, with `GROUP BY`, each
    /// column in a selected tuple is grouped or aggregated.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::libsql::Row, Rw, R, Proof>,
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

    /// Runs the query inside the transaction and returns its rows, decoded into
    /// the row type the query infers from its selection.
    ///
    /// Rows are fetched lazily as you call `next().await`. Unlike
    /// [`all`](Self::all), this method does not check scope or grouping at
    /// compile time.
    ///
    /// # Errors
    ///
    /// Returns an error when libsql cannot prepare or run the query. Decoding
    /// errors surface per row.
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

    /// Runs the query inside the transaction and decodes its first row into
    /// `R`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`](drizzle_core::error::DrizzleError::NotFound)
    /// when no row matches, and an error when libsql cannot prepare or run
    /// the query or the row cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The same checks as [`all`](Self::all).
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::libsql::Row, Rw, R, Proof>,
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
    /// Starts a relational query inside this transaction, like the database
    /// handle's `query`. It sees the transaction's uncommitted writes. Unlike the
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
    /// Runs the relational query and returns every root row with its loaded
    /// relations.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded.
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
    /// Runs the relational query with `LIMIT 1` and returns the first root row,
    /// or `None` when nothing matches.
    ///
    /// Available only while no `.limit(..)` is set.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or the row cannot be decoded.
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
    /// Runs the relational query and returns every root row with its loaded
    /// relations, in the table's `PartialSelect*` shape.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded.
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
    /// Runs the relational query with `LIMIT 1` and returns the first root row,
    /// or `None` when nothing matches.
    ///
    /// Available only while no `.limit(..)` is set.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or the row cannot be decoded.
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
