use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use rusqlite::params_from_iter;
use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;

use crate::builder::sqlite::rows::Rows;
use crate::transaction::savepoint::sync_savepoint;

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
pub type TransactionBuilder<'tx, 'conn, Schema, Builder, State> =
    crate::transaction::sqlite::typestate::TransactionBuilder<
        'tx,
        Transaction<'conn, Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::sqlite::rusqlite::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!('conn);

/// An open rusqlite transaction, passed to the closure given to `transaction`.
///
/// It has the same query methods as the database handle (`select`,
/// `insert`, `update`, `delete`, `with`), plus `savepoint` for nested
/// rollback points. It commits or rolls back when the closure returns.
#[derive(Debug)]
pub struct Transaction<'conn, Schema = ()> {
    tx: rusqlite::Transaction<'conn>,
    tx_type: SQLiteTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Wraps a driver transaction that has already begun.
    pub(crate) const fn new(
        tx: rusqlite::Transaction<'conn>,
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

    /// Returns the schema value the database handle was created with.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the driver's transaction, for calls drizzle does not cover,
    /// such as running a prepared statement inside this transaction.
    #[inline]
    pub const fn inner(&self) -> &rusqlite::Transaction<'conn> {
        &self.tx
    }

    /// Returns the mode the transaction was started with (`DEFERRED`,
    /// `IMMEDIATE`, or `EXCLUSIVE`).
    #[inline]
    pub const fn tx_type(&self) -> SQLiteTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    fn execute_raw(&self, sql: &str) -> rusqlite::Result<()> {
        self.tx.execute(sql, [])?;
        Ok(())
    }

    /// Runs `f` inside a savepoint nested in this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released.
    /// If it returns `Err` or panics, the savepoint is rolled back.
    /// The outer transaction is unaffected either way.
    ///
    /// Savepoints can be nested — each level gets its own savepoint name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use drizzle::sqlite::rusqlite::Drizzle;
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::TransactionConfig;
    /// # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let conn = ::rusqlite::Connection::open_in_memory()?;
    /// # let (mut db, S { user, .. }) = Drizzle::new(conn);
    /// # db.create()?;
    /// db.transaction(TransactionConfig::Deferred, |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///
    ///     // This savepoint fails — only its changes are rolled back
    ///     let _: Result<(), _> = tx.savepoint(|stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute()?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     });
    ///
    ///     // Alice is still inserted
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     assert_eq!(users.len(), 1);
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
            |sql| self.execute_raw(sql).map_err(DrizzleError::from),
            || f(self),
        )
    }

    sqlite_transaction_constructors!('conn);

    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, inside
    /// the transaction and returns the number of rows it changed.
    ///
    /// # Errors
    ///
    /// Returns a [`rusqlite::Error`] if the database call fails or the SQL is invalid.
    pub fn execute<'q, T>(&self, query: T) -> rusqlite::Result<usize>
    where
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx.execute");
        let query = query.to_sql();
        let (sql_str, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        crate::builder::sqlite::rusqlite::execute_sql(
            &self.tx,
            &sql_str,
            params_from_iter(params),
            query.has_returning(),
        )
    }

    /// Runs any SQL value inside the transaction and decodes every row.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails or row decoding fails.
    pub fn all<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<Vec<R>>>()
    }

    /// Runs any SQL value inside the transaction and returns its decoded rows.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails or row decoding fails.
    pub fn rows<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx.all");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.tx.prepare(&sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            R::try_from(row).map_err(Into::into)
        })?;

        let (lower, _) = rows.size_hint();
        let mut results = Vec::with_capacity(lower);
        for row in rows {
            results.push(row?);
        }

        Ok(Rows::new(results))
    }

    /// Runs any SQL value inside the transaction and decodes its first row.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError`] if the query fails, no rows match, or decoding fails.
    pub fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx.get");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.tx.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }

    /// Commits the transaction
    ///
    /// # Errors
    ///
    /// Returns [`rusqlite::Error`] if the commit call to the database fails.
    pub(crate) fn commit(self) -> rusqlite::Result<()> {
        self.tx.commit()
    }

    /// Rolls back the transaction
    ///
    /// # Errors
    ///
    /// Returns [`rusqlite::Error`] if the rollback call to the database fails.
    pub(crate) fn rollback(self) -> rusqlite::Result<()> {
        self.tx.rollback()
    }
}

#[cfg(feature = "rusqlite")]
impl<'tx, 'q, S, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, '_, S, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the statement inside the transaction and returns the number of rows it changed.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the statement, for
    /// example on a constraint violation.
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        Ok(crate::builder::sqlite::rusqlite::execute_sql(
            &self.runner.tx,
            &sql_str,
            params_from_iter(params),
            self.builder.sql.has_returning(),
        )?)
    }

    /// Runs the query inside the transaction and decodes every row into `R`.
    ///
    /// Reads see the transaction's own uncommitted writes.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the query, or when a
    /// row cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The call does not compile unless every column the query reads belongs to
    /// a table in its `FROM`/`JOIN` list, `R` matches the selection (with
    /// `Option<T>` wherever a value can be `NULL`), and, with `GROUP BY`, each
    /// column in a selected tuple is grouped or aggregated.
    pub fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.runner.tx.prepare(&sql_str)?;
        let mut raw_rows = stmt.query(params_from_iter(params))?;
        let mut decoded = Vec::new();
        while let Some(row) = raw_rows.next()? {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::rusqlite::Row<'_>,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query inside the transaction and returns its rows, decoded into
    /// the row type the query infers from its selection.
    ///
    /// Every row is fetched and decoded before this returns. Unlike [`all`](Self::all), this method does not check scope or
    /// grouping at compile time.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the query, or when a
    /// row cannot be decoded.
    pub fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <Rw as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.rows");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.runner.tx.prepare(&sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            Rw::try_from(row).map_err(Into::into)
        })?;

        let (lower, _) = rows.size_hint();
        let mut results = Vec::with_capacity(lower);
        for row in rows {
            results.push(row?);
        }

        Ok(Rows::new(results))
    }

    /// Runs the query inside the transaction and decodes its first row into
    /// `R`.
    ///
    /// # Errors
    ///
    /// Returns rusqlite's `QueryReturnedNoRows` when no row matches, and an error when SQLite
    /// cannot prepare or run the query or the row cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The same checks as [`all`](Self::all).
    pub fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.runner.tx.prepare(&sql_str)?;
        stmt.query_row(params_from_iter(params), |row| {
            Ok(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::rusqlite::Row<'_>,
                R,
            >>::decode(row))
        })?
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
            _schema: PhantomData,
        }
    }
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = rusqlite::Connection;
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        crate::builder::sqlite::rusqlite::relational_find_many(self.runner.inner(), self.builder)
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
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
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        crate::builder::sqlite::rusqlite::relational_find_many_partial(
            self.runner.inner(),
            self.builder,
        )
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
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}
