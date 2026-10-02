//! Transaction wrapper for the Durable Objects SQL driver.
//!
//! Obtained via
//! [`Drizzle::transaction`](crate::builder::sqlite::durable::Drizzle::transaction).
//! Supports the same query-builder surface as `Drizzle` plus nested
//! savepoints through [`Transaction::savepoint`]. Both run through the
//! storage's `transactionSync`; see
//! [`DurableStorage`](crate::builder::sqlite::durable::DurableStorage).

use ::worker::{SqlStorage, SqlStorageValue};
use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial},
    traits::SQLiteTable,
    values::SQLiteValue,
};

use crate::builder::sqlite::durable::{DurableStorage, sqlite_value_to_storage};

/// A query being built inside a [`Transaction`]. It has the same clause
/// methods as the connection's builder; run it with `.execute()`, `.all()`,
/// or `.get()`.
pub type TransactionBuilder<'tx, Schema, Builder, State> =
    crate::transaction::sqlite::typestate::TransactionBuilder<
        'tx,
        Transaction<Schema>,
        Schema,
        Builder,
        State,
    >;

use crate::builder::sqlite::durable::prepared;
use drizzle_core::prepared::prepare_render;

crate::drizzle_tx_prepare_impl!();

/// An open Durable Object transaction, passed to the closure given to
/// `transaction`.
///
/// Provides the same query-building surface as
/// [`Drizzle`](crate::builder::sqlite::durable::Drizzle) plus
/// [`Transaction::savepoint`] for nested savepoints.
pub struct Transaction<Schema = ()> {
    conn: DurableStorage,
    schema: Schema,
}

impl<Schema> std::fmt::Debug for Transaction<Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction").finish()
    }
}

impl<Schema> Transaction<Schema> {
    pub(crate) fn new(conn: DurableStorage, schema: Schema) -> Self {
        Self { conn, schema }
    }

    /// Returns the schema value the database handle was created with.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the storage this transaction runs on.
    #[inline]
    pub fn inner(&self) -> &DurableStorage {
        &self.conn
    }

    /// Runs `f` inside a savepoint nested in this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released. If
    /// it returns `Err` or panics, the savepoint is rolled back. The outer
    /// transaction is unaffected either way. Savepoints can be nested; the
    /// runtime's `transactionSync` gives each level its own savepoint.
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or [`DrizzleError::TransactionError`] when
    /// the runtime fails to release the savepoint.
    pub fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        self.conn.transaction(|| f(self))
    }

    sqlite_transaction_constructors!();

    /// Runs any SQL value inside the transaction and returns the number of rows
    /// it wrote.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] with the runtime's message when the
    /// statement fails.
    pub fn execute<'q, T>(&self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        let cursor = exec_in_tx(self.conn.sql(), &query)?;
        // Drain so `rows_written` is populated.
        let _ = cursor
            .to_array::<serde::de::IgnoredAny>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs any SQL value inside the transaction and deserializes its rows
    /// into `C` (for example `Vec<R>`). `R` must implement
    /// `serde::Deserialize` with field names matching the column names.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] when the query fails or a row cannot be
    /// deserialized into `R`.
    pub fn all<'q, T, R, C>(&self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'q, SQLiteValue<'q>>,
        C: Default + Extend<R>,
    {
        let cursor = exec_in_tx(self.conn.sql(), &query)?;
        let rows: Vec<R> = cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let mut out = C::default();
        out.extend(rows);
        Ok(out)
    }

    /// Runs any SQL value inside the transaction and deserializes its first
    /// row into `R`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and
    /// [`DrizzleError::Other`] when the query fails or a row cannot be
    /// deserialized into `R`.
    pub fn get<'q, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'q, SQLiteValue<'q>>,
    {
        let cursor = exec_in_tx(self.conn.sql(), &query)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }
}

fn exec_in_tx<'q, T>(
    conn: &SqlStorage,
    query: &T,
) -> drizzle_core::error::Result<::worker::SqlCursor>
where
    T: ToSQL<'q, SQLiteValue<'q>>,
{
    let sql = query.to_sql();
    let (sql_str, params) = sql.build();
    drizzle_core::drizzle_trace_query!(&sql_str, params.len());
    let values: Vec<SqlStorageValue> = params.into_iter().map(sqlite_value_to_storage).collect();
    conn.exec(&sql_str, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

// =============================================================================
// Terminal methods on TransactionBuilder (execute / all / get)
// =============================================================================

#[cfg(feature = "durable")]
impl<'tx, 'q, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'tx, Schema, QueryBuilder<'q, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the statement inside the transaction and returns the number of
    /// rows it wrote.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] with the runtime's message when the
    /// statement fails.
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        let cursor = exec_in_tx(self.runner.conn.sql(), &self.builder.sql)?;
        let _ = cursor
            .to_array::<serde::de::IgnoredAny>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs the query inside the transaction and deserializes every row into
    /// `R`, which must implement `serde::Deserialize` with field names matching
    /// the selected column names.
    ///
    /// Unlike the database handle's `.all()`, this does not check scope or
    /// grouping at compile time, and `R` is not checked against the selection.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] when the query fails or a row cannot be
    /// deserialized into `R`.
    pub fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_in_tx(self.runner.conn.sql(), &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query inside the transaction and deserializes its first row
    /// into `R`, with the same requirements as [`all`](Self::all).
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and
    /// [`DrizzleError::Other`] when the query fails or a row cannot be
    /// deserialized into `R`.
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_in_tx(self.runner.conn.sql(), &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
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
    /// handle's `query`. It sees the transaction's uncommitted writes.
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
    type PreparedDriver = DurableStorage;
}

// AllColumns: base decoded from the JSON "__base" column
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        crate::builder::sqlite::durable::relational_find_many(
            self.runner.inner().sql(),
            self.builder,
        )
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

// PartialColumns: base decoded from the JSON "__base" column of selected columns
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
        crate::builder::sqlite::durable::relational_find_many_partial(
            self.runner.inner().sql(),
            self.builder,
        )
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
