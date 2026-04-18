//! Transaction wrapper for the Durable Objects SQL driver.
//!
//! Obtained via
//! [`Drizzle::transaction`](crate::builder::sqlite::durable::Drizzle::transaction).
//! Supports the same query-builder surface as `Drizzle` plus nested
//! savepoints through [`Transaction::savepoint`].

mod delete;
mod insert;
mod select;
mod update;

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

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

use crate::builder::sqlite::durable::sqlite_value_to_storage;

/// Query builder scoped to a [`Transaction`].
#[derive(Debug)]
pub struct TransactionBuilder<'a, Schema, Builder, State> {
    transaction: &'a Transaction<Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction handle for a Durable Object's SQL storage.
///
/// Provides the same query-building surface as
/// [`Drizzle`](crate::builder::sqlite::durable::Drizzle) plus
/// [`Transaction::savepoint`] for nested savepoints.
pub struct Transaction<Schema = ()> {
    conn: SqlStorage,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<Schema> std::fmt::Debug for Transaction<Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction").finish()
    }
}

impl<Schema> Transaction<Schema> {
    pub(crate) fn new(conn: SqlStorage, schema: Schema) -> Self {
        Self {
            conn,
            savepoint_depth: AtomicU32::new(0),
            schema,
        }
    }

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Gets a reference to the underlying [`SqlStorage`] handle.
    #[inline]
    pub fn inner(&self) -> &SqlStorage {
        &self.conn
    }

    /// Executes a nested savepoint within this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released. If
    /// it returns `Err` or panics, the savepoint is rolled back. The outer
    /// transaction is unaffected either way. Savepoints can be nested — each
    /// level gets its own savepoint name.
    pub fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        let depth = self.savepoint_depth.load(Ordering::Relaxed);
        let sp_name = format!("drizzle_sp_{}", depth);
        self.savepoint_depth.store(depth + 1, Ordering::Relaxed);

        self.conn
            .exec(&format!("SAVEPOINT {}", sp_name), None)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self)));

        self.savepoint_depth.store(depth, Ordering::Relaxed);

        match result {
            Ok(Ok(value)) => {
                self.conn
                    .exec(&format!("RELEASE SAVEPOINT {}", sp_name), None)
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                Ok(value)
            }
            Ok(Err(e)) => {
                let _ = self
                    .conn
                    .exec(&format!("ROLLBACK TO SAVEPOINT {}", sp_name), None);
                let _ = self
                    .conn
                    .exec(&format!("RELEASE SAVEPOINT {}", sp_name), None);
                Err(e)
            }
            Err(panic_payload) => {
                let _ = self
                    .conn
                    .exec(&format!("ROLLBACK TO SAVEPOINT {}", sp_name), None);
                let _ = self
                    .conn
                    .exec(&format!("RELEASE SAVEPOINT {}", sp_name), None);
                std::panic::resume_unwind(panic_payload);
            }
        }
    }

    sqlite_transaction_constructors!();

    /// Executes a query within the transaction and returns the number of rows
    /// written.
    pub fn execute<'a, T>(&self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let cursor = exec_in_tx(&self.conn, &query)?;
        // Drain so `rows_written` is populated.
        let _ = cursor
            .to_array::<serde_json::Value>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs a query and returns all matching rows within the transaction.
    pub fn all<'a, T, R, C>(&self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: Default + Extend<R>,
    {
        let cursor = exec_in_tx(&self.conn, &query)?;
        let rows: Vec<R> = cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let mut out = C::default();
        out.extend(rows);
        Ok(out)
    }

    /// Runs a query and returns a single row within the transaction.
    pub fn get<'a, T, R>(&self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let cursor = exec_in_tx(&self.conn, &query)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }
}

fn exec_in_tx<'a, T>(
    conn: &SqlStorage,
    query: &T,
) -> drizzle_core::error::Result<::worker::SqlCursor>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    let sql = query.to_sql();
    let (sql_str, params) = sql.build();
    let values: Vec<SqlStorageValue> = params.into_iter().map(sqlite_value_to_storage).collect();
    conn.exec(&sql_str, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

// =============================================================================
// Terminal methods on TransactionBuilder (execute / all / get)
// =============================================================================

#[cfg(feature = "durable")]
impl<'a, 'b, Schema, State, Table, Mk, Rw, Grouped>
    TransactionBuilder<'a, Schema, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of rows written.
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        let cursor = exec_in_tx(&self.transaction.conn, &self.builder.sql)?;
        let _ = cursor
            .to_array::<serde_json::Value>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs the query and returns all matching rows deserialized into `R`.
    pub fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_in_tx(&self.transaction.conn, &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and returns the first matching row.
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_in_tx(&self.transaction.conn, &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }
}
