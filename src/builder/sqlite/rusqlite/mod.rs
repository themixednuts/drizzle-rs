//! Synchronous SQLite driver using [`rusqlite`].
//!
//! # Example
//!
//! ```no_run
//! use drizzle::rusqlite::Drizzle;
//! use drizzle::sqlite::prelude::*;
//!
//! #[SQLiteTable]
//! struct User {
//!     #[column(primary)]
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct AppSchema {
//!     user: User,
//! }
//!
//! fn main() -> drizzle::Result<()> {
//!     let conn = ::rusqlite::Connection::open_in_memory()?;
//!     let (db, AppSchema { user, .. }) = Drizzle::new(conn, AppSchema::new());
//!     db.create()?;
//!
//!     // Insert
//!     db.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!
//!     // Select
//!     let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//!
//!     Ok(())
//! }
//! ```

mod delete;
mod insert;
mod prepared;
mod select;
mod update;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;
use rusqlite::{Connection, params_from_iter};
use std::marker::PhantomData;

use drizzle_sqlite::{
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
    connection::SQLiteTransactionType,
};

/// Rusqlite-specific drizzle builder
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}
use crate::transaction::sqlite::rusqlite::Transaction;

// Generic prepare method for rusqlite DrizzleBuilder
impl<'a: 'b, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Creates a prepared statement that can be executed multiple times
    #[inline]
    pub fn prepare(self) -> prepared::PreparedStatement<'b> {
        let inner = prepare_render(self.to_sql().clone());
        prepared::PreparedStatement { inner }
    }
}

/// Synchronous SQLite database wrapper using [`rusqlite::Connection`].
///
/// Provides query building methods (`select`, `insert`, `update`, `delete`)
/// and execution methods (`execute`, `all`, `get`, `transaction`).
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    conn: Connection,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub const fn new<S>(conn: Connection, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            conn,
            _schema: PhantomData,
        };
        (drizzle, schema)
    }
}

impl<S> AsRef<Drizzle<S>> for Drizzle<S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    /// Gets a reference to the underlying connection
    #[inline]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[inline]
    pub fn mut_conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'b, SQLiteValue<'b>>,
    {
        use drizzle_sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    pub fn insert<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    pub fn update<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    pub fn delete<'a, 'b, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, T>, DeleteInitial>
    where
        T: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression).
    pub fn with<'a, 'b, C>(
        &'a self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    pub fn execute<'a, T>(&'a self, query: T) -> rusqlite::Result<usize>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        self.conn.execute(&sql, params_from_iter(params))
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();

        let params = sql.params();

        let mut stmt = self
            .conn
            .prepare(&sql_str)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?;

        rows.collect::<Result<Result<C, _>, _>>()?
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();

        // Get parameters and handle potential errors from IntoParams
        let params = sql.params();

        let mut stmt = self.conn.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }

    /// Executes a transaction with the given callback
    pub fn transaction<F, R>(
        &mut self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let tx = self.conn.transaction_with_behavior(tx_type.into())?;

        let transaction = Transaction::new(tx, tx_type);

        // Use catch_unwind to handle panics and ensure rollback
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&transaction)));

        match result {
            Ok(callback_result) => match callback_result {
                Ok(value) => {
                    transaction.commit()?;
                    Ok(value)
                }
                Err(e) => {
                    transaction.rollback()?;
                    Err(e)
                }
            },
            Err(panic_payload) => {
                // Rollback on panic and resume unwinding
                let _ = transaction.rollback();
                std::panic::resume_unwind(panic_payload);
            }
        }
    }
}

// Implementation for schemas that implement SQLSchemaImpl
impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects using SQLSchemaImpl trait
    pub fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements();
        if !statements.is_empty() {
            let batch_sql = statements.join(";");
            self.conn.execute_batch(&batch_sql)?;
        }
        Ok(())
    }
}

// Migration support
impl<Schema> Drizzle<Schema> {
    /// Run pending migrations from a MigrationSet.
    ///
    /// This method follows the drizzle-orm migration spec:
    /// - Creates the migrations table if it doesn't exist (idempotent)
    /// - Queries the last applied migration by `created_at`
    /// - Runs all pending migrations in a single transaction
    /// - Records each migration after execution
    ///
    /// # Example
    ///
    /// ```ignore
    /// use drizzle::rusqlite::Drizzle;
    /// use drizzle_migrations::{migrations, MigrationSet};
    /// use drizzle_types::Dialect;
    ///
    /// let migrations = migrations![
    ///     ("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
    ///     ("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
    /// ];
    /// let set = MigrationSet::new(migrations, Dialect::SQLite);
    ///
    /// let conn = rusqlite::Connection::open("./dev.db")?;
    /// let (db, _) = Drizzle::new(conn, ());
    ///
    /// db.migrate(&set)?;
    /// ```
    pub fn migrate(
        &self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        use rusqlite::OptionalExtension;

        // 1. Create migrations table (idempotent)
        self.conn.execute(&migrations.create_table_sql(), [])?;

        // 2. Query last applied migration
        let last_created_at: Option<i64> = self
            .conn
            .query_row(
                &migrations.query_last_applied_sql(),
                [],
                |row| row.get::<_, i64>(2), // created_at is the 3rd column
            )
            .optional()?;

        // 3. Get pending migrations
        let applied_hashes: Vec<String> = if last_created_at.is_some() {
            // Get all applied hashes
            let mut stmt = self.conn.prepare(&migrations.query_all_hashes_sql())?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

        let pending: Vec<_> = migrations.pending(&applied_hashes).collect();

        if pending.is_empty() {
            return Ok(());
        }

        // 4. Execute all pending in a single transaction
        self.conn.execute("BEGIN", [])?;

        let result = (|| -> drizzle_core::error::Result<()> {
            for migration in &pending {
                for stmt in migration.statements() {
                    if !stmt.trim().is_empty() {
                        self.conn.execute(stmt, [])?;
                    }
                }
                // Record migration
                self.conn.execute(
                    &migrations.record_migration_sql(migration.hash(), migration.created_at()),
                    [],
                )?;
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute("COMMIT", [])?;
                Ok(())
            }
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }
}

// CTE (WITH) Builder Implementation for RusQLite
impl<'a, Schema>
    DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(
        self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

// Rusqlite-specific execution methods for all ExecutableState QueryBuilders
impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params().cloned();
        Ok(self
            .drizzle
            .conn
            .execute(&sql_str, params_from_iter(params))?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params().cloned();

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?;

        rows.collect::<Result<Result<C, _>, _>>()?
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params().cloned();

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }
}

impl<'a, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'a, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.builder.to_sql()
    }
}
