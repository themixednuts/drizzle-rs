//! Synchronous SQLite driver using [`rusqlite`].
//!
//! # Example
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
        let statements = schema.create_statements();
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
        use rusqlite::OptionalExtension;

        self.conn.execute(&migrations.create_table_sql(), [])?;
        let last_created_at: Option<i64> = self
            .conn
            .query_row(
                &migrations.query_last_applied_sql(),
                [],
                |row| row.get::<_, i64>(2), // created_at is the 3rd column
            )
            .optional()?;
        let applied_hashes: Vec<String> = if last_created_at.is_some() {
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
