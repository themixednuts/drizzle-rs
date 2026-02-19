use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
use rusqlite::params_from_iter;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::builder::sqlite::rows::Rows;

pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

/// Rusqlite-specific transaction builder
#[derive(Debug)]
pub struct TransactionBuilder<'a, 'conn, Schema, Builder, State> {
    transaction: &'a Transaction<'conn, Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction wrapper that provides the same query building capabilities as Drizzle
#[derive(Debug)]
pub struct Transaction<'conn, Schema = ()> {
    tx: rusqlite::Transaction<'conn>,
    tx_type: SQLiteTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(
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

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Gets a reference to the underlying transaction
    #[inline]
    pub fn inner(&self) -> &rusqlite::Transaction<'conn> {
        &self.tx
    }

    /// Gets the transaction type
    #[inline]
    pub fn tx_type(&self) -> SQLiteTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    fn execute_raw(&self, sql: &str) -> rusqlite::Result<()> {
        self.tx.execute(sql, [])?;
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
    /// # use drizzle::sqlite::rusqlite::Drizzle;
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::connection::SQLiteTransactionType;
    /// # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let conn = ::rusqlite::Connection::open_in_memory()?;
    /// # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
    /// # db.create()?;
    /// db.transaction(SQLiteTransactionType::Deferred, |tx| {
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
    pub fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        let depth = self.savepoint_depth.load(Ordering::Relaxed);
        let sp_name = format!("drizzle_sp_{}", depth);
        self.savepoint_depth.store(depth + 1, Ordering::Relaxed);

        self.execute_raw(&format!("SAVEPOINT {}", sp_name))?;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self)));

        self.savepoint_depth.store(depth, Ordering::Relaxed);

        match result {
            Ok(Ok(value)) => {
                self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name))?;
                Ok(value)
            }
            Ok(Err(e)) => {
                let _ = self.execute_raw(&format!("ROLLBACK TO SAVEPOINT {}", sp_name));
                let _ = self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name));
                Err(e)
            }
            Err(panic_payload) => {
                let _ = self.execute_raw(&format!("ROLLBACK TO SAVEPOINT {}", sp_name));
                let _ = self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name));
                std::panic::resume_unwind(panic_payload);
            }
        }
    }

    /// Creates a SELECT query builder within the transaction
    #[cfg(feature = "sqlite")]
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
    {
        use drizzle_sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an INSERT query builder within the transaction
    #[cfg(feature = "sqlite")]
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
        Table: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an UPDATE query builder within the transaction
    #[cfg(feature = "sqlite")]
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
        Table: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates a DELETE query builder within the transaction
    #[cfg(feature = "sqlite")]
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
        T: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Executes a raw query within the transaction
    pub fn execute<'a, T>(&'a self, query: T) -> rusqlite::Result<usize>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx.execute");
        let query = query.to_sql();
        let (sql_str, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        self.tx.execute(&sql_str, params_from_iter(params))
    }

    /// Runs a query and returns all matching rows within the transaction
    pub fn all<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<Vec<R>>>()
    }

    /// Runs a query and returns a row cursor within the transaction.
    pub fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
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

    /// Runs a query and returns a single row within the transaction
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
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
    pub fn commit(self) -> rusqlite::Result<()> {
        self.tx.commit()
    }

    /// Rolls back the transaction
    pub fn rollback(self) -> rusqlite::Result<()> {
        self.tx.rollback()
    }
}

#[cfg(feature = "rusqlite")]
impl<'a, 'conn, S, Schema, State, Table, Mk, Rw>
    TransactionBuilder<'a, 'conn, S, QueryBuilder<'a, Schema, State, Table, Mk, Rw>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        Ok(self
            .transaction
            .tx
            .execute(&sql_str, params_from_iter(params))?)
    }

    /// Runs the query and returns all matching rows, decoded as `R`.
    pub fn all_as<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        self.rows_as::<R>()?
            .collect::<drizzle_core::error::Result<Vec<R>>>()
    }

    /// Runs the query and returns a row cursor, decoded as `R`.
    pub fn rows_as<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.transaction.tx.prepare(&sql_str)?;

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

    /// Runs the query and returns a single row, decoded as `R`.
    pub fn get_as<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.transaction.tx.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub fn all<R, Proof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.transaction.tx.prepare(&sql_str)?;
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

    /// Runs the query and returns a row cursor using the builder's row type.
    pub fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <Rw as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        self.rows_as()
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub fn get<R, Proof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.transaction.tx.prepare(&sql_str)?;
        stmt.query_row(params_from_iter(params), |row| {
            Ok(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::rusqlite::Row<'_>,
                R,
            >>::decode(row))
        })?
    }
}
