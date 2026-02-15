//! Synchronous SQLite driver using [`rusqlite`].
//!
//! # Quick start
//!
//! ```no_run
//! use drizzle::sqlite::rusqlite::Drizzle;
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
//!
//! # Transactions
//!
//! Return `Ok(value)` to commit, `Err(...)` to rollback. Panics also trigger
//! a rollback.
//!
//! ```no_run
//! # use drizzle::sqlite::rusqlite::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let conn = ::rusqlite::Connection::open_in_memory()?;
//! # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
//! # db.create()?;
//! use drizzle::sqlite::connection::SQLiteTransactionType;
//!
//! let count = db.transaction(SQLiteTransactionType::Deferred, |tx| {
//!     tx.insert(user)
//!         .values([InsertUser::new("Alice")])
//!         .execute()?;
//!
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
//!     Ok(users.len())
//! })?;
//! # Ok(()) }
//! ```
//!
//! # Savepoints
//!
//! Savepoints nest inside transactions — a failed savepoint rolls back
//! without aborting the outer transaction.
//!
//! ```no_run
//! # use drizzle::sqlite::rusqlite::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::sqlite::connection::SQLiteTransactionType;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let conn = ::rusqlite::Connection::open_in_memory()?;
//! # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
//! # db.create()?;
//! db.transaction(SQLiteTransactionType::Deferred, |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!
//!     // This savepoint fails and rolls back, but Alice is still inserted
//!     let _: Result<(), _> = tx.savepoint(|stx| {
//!         stx.insert(user).values([InsertUser::new("Bad")]).execute()?;
//!         Err(drizzle::error::DrizzleError::Other("rollback this".into()))
//!     });
//!
//!     tx.insert(user).values([InsertUser::new("Bob")]).execute()?;
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
//!     assert_eq!(users.len(), 2); // Alice + Bob, not Bad
//!     Ok(())
//! })?;
//! # Ok(()) }
//! ```
//!
//! # Prepared statements
//!
//! Build a query once and execute it many times with different parameters.
//! Use [`Placeholder::named`] for values that change between executions.
//!
//! ```no_run
//! # use drizzle::sqlite::rusqlite::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::core::expr::eq;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let conn = ::rusqlite::Connection::open_in_memory()?;
//! # let (db, S { user, .. }) = Drizzle::new(conn, S::new());
//! # db.create()?;
//! use drizzle::sqlite::params;
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, Placeholder::named("find_name")))
//!     .prepare();
//!
//! // Execute with different bound values each time
//! let alice: Vec<SelectUser> = find_user.all(db.conn(), params![{find_name: "Alice"}])?;
//! let bob: Vec<SelectUser> = find_user.all(db.conn(), params![{find_name: "Bob"}])?;
//! # Ok(()) }
//! ```

mod prepared;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_sqlite::values::SQLiteValue;
use rusqlite::{Connection, params_from_iter};

use drizzle_sqlite::{
    builder::{self, QueryBuilder},
    connection::SQLiteTransactionType,
};

use crate::builder::sqlite::common;
use crate::builder::sqlite::rows::Rows;
use crate::transaction::sqlite::rusqlite::Transaction;

pub type Drizzle<Schema = ()> = common::Drizzle<Connection, Schema>;
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, Connection, Schema, Builder, State>;

crate::drizzle_prepare_impl!();

impl<Schema> common::Drizzle<Connection, Schema> {
    pub fn execute<'a, T>(&'a self, query: T) -> rusqlite::Result<usize>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "drizzle.execute");
        let query = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "drizzle.execute.build");
        let (sql_str, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        self.conn.execute(&sql_str, params_from_iter(params))
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
        self.rows(query)?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a row cursor.
    pub fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "drizzle.all");
        let sql = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "drizzle.all.build");
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.conn.prepare(&sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            R::try_from(row).map_err(Into::into)
        })?;

        let (lower, _) = rows.size_hint();
        let mut decoded = Vec::with_capacity(lower);
        for row in rows {
            decoded.push(row?);
        }

        Ok(Rows::new(decoded))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "drizzle.get");
        let sql = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "drizzle.get.build");
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.conn.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }

    /// Executes a transaction with the given callback.
    ///
    /// Returns the value produced by the callback on success. The transaction
    /// is committed when the callback returns `Ok` and rolled back on `Err`
    /// or panic.
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
    /// let count = db.transaction(SQLiteTransactionType::Deferred, |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     Ok(users.len())
    /// })?;
    /// assert_eq!(count, 1);
    /// # Ok(()) }
    /// ```
    pub fn transaction<F, R>(
        &mut self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        drizzle_core::drizzle_trace_tx!("begin", "sqlite.rusqlite");
        let tx = self.conn.transaction_with_behavior(tx_type.into())?;

        let transaction = Transaction::new(tx, tx_type, self.schema);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&transaction)));

        match result {
            Ok(callback_result) => match callback_result {
                Ok(value) => {
                    drizzle_core::drizzle_trace_tx!("commit", "sqlite.rusqlite");
                    transaction.commit()?;
                    Ok(value)
                }
                Err(e) => {
                    drizzle_core::drizzle_trace_tx!("rollback", "sqlite.rusqlite");
                    transaction.rollback()?;
                    Err(e)
                }
            },
            Err(panic_payload) => {
                drizzle_core::drizzle_trace_tx!("rollback", "sqlite.rusqlite");
                let _ = transaction.rollback();
                std::panic::resume_unwind(panic_payload);
            }
        }
    }
}

impl<Schema> common::Drizzle<Connection, Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects from `SQLSchemaImpl`.
    pub fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements: Vec<_> = schema.create_statements()?.collect();
        if !statements.is_empty() {
            let batch_sql = statements.join(";");
            self.conn.execute_batch(&batch_sql)?;
        }
        Ok(())
    }
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Apply pending migrations from a MigrationSet.
    ///
    /// Creates the migrations table if needed and runs pending migrations in a transaction.
    pub fn migrate(
        &self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        self.conn.execute(&migrations.create_table_sql(), [])?;
        let mut stmt = self.conn.prepare(&migrations.query_all_created_at_sql())?;
        let rows = stmt.query_map([], |row| row.get::<_, Option<i64>>(0))?;
        let applied_created_at = rows.filter_map(Result::ok).flatten().collect::<Vec<_>>();

        let pending: Vec<_> = migrations
            .pending_by_created_at(&applied_created_at)
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        self.conn.execute("BEGIN", [])?;

        let result = (|| -> drizzle_core::error::Result<()> {
            for migration in &pending {
                for stmt in migration.statements() {
                    if !stmt.trim().is_empty() {
                        self.conn.execute(stmt, [])?;
                    }
                }
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

impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
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
        self.rows::<R>()?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a row cursor.
    pub fn rows<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            R::try_from(row).map_err(Into::into)
        })?;

        let (lower, _) = rows.size_hint();
        let mut decoded = Vec::with_capacity(lower);
        for row in rows {
            decoded.push(row?);
        }

        Ok(Rows::new(decoded))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }
}
