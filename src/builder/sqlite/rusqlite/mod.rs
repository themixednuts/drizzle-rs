//! Synchronous `SQLite` driver using [`rusqlite`].
//!
//! # Quick start
//!
//! ```
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
//!     let (db, AppSchema { user, .. }) = Drizzle::new(conn);
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
//! ```
//! # use drizzle::sqlite::rusqlite::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let conn = ::rusqlite::Connection::open_in_memory()?;
//! # let (mut db, S { user, .. }) = Drizzle::new(conn);
//! # db.create()?;
//! use drizzle::sqlite::TransactionConfig;
//!
//! let count = db.transaction(TransactionConfig::Deferred, |tx| {
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
//! ```
//! # use drizzle::sqlite::rusqlite::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::sqlite::TransactionConfig;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let conn = ::rusqlite::Connection::open_in_memory()?;
//! # let (mut db, S { user, .. }) = Drizzle::new(conn);
//! # db.create()?;
//! db.transaction(TransactionConfig::Deferred, |tx| {
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
//! Use `column.placeholder("name")` for type-safe bind parameters.
//!
//! ```
//! # use drizzle::sqlite::rusqlite::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::core::expr::eq;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let conn = ::rusqlite::Connection::open_in_memory()?;
//! # let (db, S { user, .. }) = Drizzle::new(conn);
//! # db.create()?;
//!
//! let find_name = user.name.placeholder("find_name");
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, find_name))
//!     .prepare();
//!
//! // Execute with different bound values each time
//! let alice: Vec<SelectUser> = find_user.all(db.conn(), [find_name.bind("Alice")])?;
//! let bob: Vec<SelectUser> = find_user.all(db.conn(), [find_name.bind("Bob")])?;
//! # Ok(()) }
//! ```
//!
//! # Statement caching
//!
//! The builder paths here deliberately call [`Connection::prepare`], not
//! `prepare_cached`, and that is not an oversight.
//!
//! Unlike the network drivers, where preparing costs a server round trip,
//! `sqlite3_prepare_v2` is an in-process C call against an already-open
//! database. Caching was measured on this driver and came out
//! **net-neutral to negative**: the hash lookup and the `RefCell` bookkeeping
//! that `prepare_cached` adds cost about as much as the prepare it avoids,
//! even on the warm path, while pinning statements alive for the connection's
//! lifetime. See the `sqlite-stmt-cache-neutral` note in the project's
//! benchmark records.
//!
//! Do not wire `prepare_cached` into these paths without a fresh measurement
//! showing a win. The explicit `.prepare()` API
//! does use `prepare_cached`, because there the caller has already declared
//! that one statement is going to be reused many times.

pub(crate) mod prepared;

use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_sqlite::values::SQLiteValue;
use rusqlite::{Connection, params_from_iter};

use drizzle_sqlite::builder::{self, QueryBuilder};

use crate::builder::sqlite::common;
use crate::builder::sqlite::rows::Rows;
use crate::transaction::sqlite::rusqlite::Transaction;

/// The rusqlite database handle: a [`rusqlite::Connection`] plus the schema's table handles.
///
/// Create it with `Drizzle::new(conn)`, then build queries with
/// `select`, `insert`, `update`, and `delete`.
pub type Drizzle<Schema = ()> = common::Drizzle<Connection, Schema>;
/// A query attached to a [`Drizzle`] handle, ready to run with `.execute()`,
/// `.all()`, `.get()`, or `.rows()`.
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, common::Drizzle<Connection, Schema>, Schema, Builder, State>;

crate::drizzle_prepare_impl!();

/// Runs a prepared statement and returns the number of rows it changed.
///
/// A statement with a `RETURNING` clause returns rows, which rusqlite's
/// `execute` rejects after SQLite has already applied the change. It is
/// stepped to completion instead; the rows it returned are the rows it
/// changed.
pub(crate) fn run_statement<P: rusqlite::Params>(
    statement: &mut rusqlite::Statement<'_>,
    params: P,
) -> rusqlite::Result<usize> {
    if statement.column_count() == 0 {
        return statement.execute(params);
    }
    let mut rows = statement.query(params)?;
    let mut changed = 0;
    while rows.next()?.is_some() {
        changed += 1;
    }
    Ok(changed)
}

/// Runs `sql` and returns the number of rows it changed (see
/// [`run_statement`]). Statements without a `RETURNING` clause keep
/// rusqlite's `execute`, which also rejects trailing statements.
pub(crate) fn execute_sql<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    returns_rows: bool,
) -> rusqlite::Result<usize> {
    if returns_rows {
        run_statement(&mut conn.prepare(sql)?, params)
    } else {
        conn.execute(sql, params)
    }
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, and
    /// returns the number of rows it changed.
    ///
    /// Prefer the builder's own `.execute()`. This method takes anything that
    /// renders to SQL, so it skips the builder's compile-time checks.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// use drizzle::sqlite::rusqlite::Drizzle;
    ///
    /// let (db, ()) = Drizzle::new(rusqlite::Connection::open_in_memory()?);
    /// db.execute(drizzle::sql!("CREATE TABLE notes (body TEXT)"))?;
    /// let inserted = db.execute(drizzle::sql!("INSERT INTO notes VALUES ('hi')"))?;
    /// assert_eq!(inserted, 1);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns rusqlite's error when SQLite cannot prepare or run the
    /// statement. Unlike the builder methods, the error is a plain
    /// [`rusqlite::Error`] without the SQL attached.
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

        execute_sql(
            &self.conn,
            &sql_str,
            params_from_iter(params),
            query.has_returning(),
        )
    }

    /// Runs any SQL value and collects its rows into `C` (for example
    /// `Vec<R>`).
    ///
    /// Each row is decoded with `R: TryFrom<&rusqlite::Row>`, which the
    /// generated `Select*` models and [`SQLiteFromRow`](crate::sqlite::SQLiteFromRow)
    /// types implement. Prefer the builder's own `.all()`: this method skips
    /// its compile-time scope and `NULL` checks.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # let (db, Schema { users, .. }) = app::database()?;
    /// let everyone: Vec<SelectUsers> = db.all(drizzle::sql!("SELECT * FROM {users}"))?;
    /// assert_eq!(everyone.len(), 3);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the query, or when a
    /// row cannot be decoded into `R`.
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

    /// Runs any SQL value and returns an iterator over its rows decoded into
    /// `R`.
    ///
    /// Every row is fetched and decoded before this returns. Like
    /// [`all`](Self::all), this skips the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the query, or when a
    /// row cannot be decoded into `R`.
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

        let mut stmt = self
            .conn
            .prepare(&sql_str)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let mut rows = stmt
            .query_and_then(params_from_iter(params.iter().copied()), |row| {
                R::try_from(row).map_err(Into::into)
            })
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let (lower, _) = rows.size_hint();
        let mut decoded = Vec::with_capacity(lower);
        for row in rows {
            decoded.push(row?);
        }

        Ok(Rows::new(decoded))
    }

    /// Runs any SQL value and decodes its first row into `R`.
    ///
    /// Like [`all`](Self::all), this skips the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when no row matches (rusqlite's
    /// `QueryReturnedNoRows`), when SQLite cannot prepare or run the query, or
    /// when the row cannot be decoded into `R`.
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

        let mut stmt = self
            .conn
            .prepare(&sql_str)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        stmt.query_row(params_from_iter(params.iter().copied()), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })
        .with_query(|| QueryContext::new(&sql_str, &params))?
    }

    fn start(
        &mut self,
        config: drizzle_sqlite::TransactionConfig,
    ) -> drizzle_core::error::Result<Transaction<'_, Schema>>
    where
        Schema: Copy,
    {
        drizzle_core::drizzle_trace_tx!("begin", "sqlite.rusqlite");
        let tx = self.conn.transaction_with_behavior(config.into())?;
        Ok(Transaction::new(tx, config, self.schema))
    }

    /// Runs `f` inside a transaction and returns its value.
    ///
    /// The transaction commits when `f` returns `Ok` and rolls back when it
    /// returns `Err` or panics (the panic then continues). `config` picks the
    /// SQLite mode: `BEGIN DEFERRED`, `IMMEDIATE`, or `EXCLUSIVE`.
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
    /// let count = db.transaction(TransactionConfig::Deferred, |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     Ok(users.len())
    /// })?;
    /// assert_eq!(count, 1);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or an error when `BEGIN`, `COMMIT`, or
    /// `ROLLBACK` fails. When the rollback after an `Err` also fails, both
    /// errors are reported together.
    pub fn transaction<F, R>(
        &mut self,
        config: drizzle_sqlite::TransactionConfig,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let transaction = self.start(config)?;

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
                    // Report the callback's error, with a failed rollback attached.
                    match transaction.rollback() {
                        Ok(()) => Err(e),
                        Err(rollback) => Err(crate::transaction::savepoint::cleanup_error(
                            "transaction",
                            e,
                            "rollback",
                            rollback.into(),
                        )),
                    }
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
    /// Creates every table, index, and view in the schema.
    ///
    /// Runs the schema's `CREATE` statements in one batch. Useful for tests
    /// and throwaway databases; use [`migrate`](Self::migrate) to evolve a
    /// real one.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite rejects one of the statements.
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
    /// Applies the migrations that have not run yet.
    ///
    /// Creates the tracking table if needed, then runs every pending migration
    /// and records it, all in one `BEGIN IMMEDIATE` transaction. An
    /// interrupted run rolls back completely and leaves nothing to clean up.
    /// Migrations that already ran are skipped.
    ///
    /// The connection's busy timeout is set to 30 seconds. When a pending
    /// migration wraps statements in `PRAGMA foreign_keys=OFF` / `ON` (as
    /// generated table rebuilds do), enforcement is switched off around the
    /// whole transaction instead, `PRAGMA foreign_key_check` must pass before
    /// `COMMIT`, and enforcement is restored afterwards.
    ///
    /// Load the migrations with [`include_migrations!`](crate::include_migrations)
    /// or [`MigrationDir`](drizzle_migrations::MigrationDir).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// use drizzle::migrations::{MigrateOutcome, Migration, Tracking};
    /// use drizzle::sqlite::rusqlite::Drizzle;
    ///
    /// let (db, ()) = Drizzle::new(rusqlite::Connection::open_in_memory()?);
    /// let migrations = [Migration::new("0000_init", "CREATE TABLE notes (body TEXT);")];
    ///
    /// let first = db.migrate(&migrations, Tracking::SQLITE)?;
    /// assert!(matches!(first, MigrateOutcome::Applied { .. }));
    ///
    /// let again = db.migrate(&migrations, Tracking::SQLITE)?;
    /// assert!(matches!(again, MigrateOutcome::UpToDate));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when a migration statement fails (the whole run is
    /// rolled back), or when the tracking table holds an unfinished ("dirty")
    /// row left by an interrupted runner. Use
    /// [`migrate_with_repair`](Self::migrate_with_repair) for that case.
    pub fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        self.migrate_inner(migrations, tracking, false)
    }

    /// Finishes any interrupted migration, then applies pending ones like
    /// [`migrate`](Self::migrate).
    ///
    /// A migration is "dirty" when its tracking row exists but `applied_at` is
    /// `NULL`. For each one, this reads `sqlite_master` and checks every
    /// statement. A `CREATE TABLE`, `CREATE [UNIQUE] INDEX`, or `CREATE VIEW`
    /// whose object already exists with the same definition is skipped; the
    /// rest run.
    ///
    /// # Errors
    ///
    /// Returns an error listing what needs manual attention when a statement
    /// cannot be proven applied or not applied (an `ALTER`, a data statement,
    /// or one that cannot be parsed), and for the same failures as
    /// [`migrate`](Self::migrate).
    pub fn migrate_with_repair(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        self.migrate_inner(migrations, tracking, true)
    }

    fn migrate_inner(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
        repair: bool,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::SQLite,
            tracking,
        );

        ensure_sqlite_migration_table(&self.conn, &set)?;
        self.conn.busy_timeout(std::time::Duration::from_secs(30))?;
        let applied_before_transaction = load_applied_migration_names(&self.conn, &set)?;
        let suspends_foreign_keys = set
            .pending(&applied_before_transaction)
            .map(|migration| migration.sqlite_execution())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?
            .iter()
            .any(|execution| execution.suspends_foreign_keys());
        let foreign_keys_were_enabled = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))?
            != 0;
        let suspend_foreign_keys = suspends_foreign_keys && foreign_keys_were_enabled;
        if suspend_foreign_keys && let Err(error) = set_sqlite_foreign_keys(&self.conn, false) {
            let restore = set_sqlite_foreign_keys(&self.conn, true);
            return super::finish_foreign_key_scope(Err(error), restore);
        }

        let result = (|| -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
            self.conn.execute("BEGIN IMMEDIATE", [])?;
            let mut applied = repair_dirty_migrations(&self.conn, &set, repair)?;

            let mut statement = self.conn.prepare(&set.applied_names_sql())?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            let applied_names = rows.collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            let pending: Vec<_> = set.pending(&applied_names).collect();
            if pending.is_empty() && applied.is_empty() {
                return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
            }

            for migration in &pending {
                let execution = migration
                    .sqlite_execution()
                    .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
                for stmt in execution.statements() {
                    if !stmt.trim().is_empty() {
                        self.conn.execute(stmt, [])?;
                    }
                }
                self.conn
                    .execute(&set.record_migration_sql(migration), [])?;
                applied.push(migration.tag().to_string());
            }
            Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied })
        })();

        let result = match result {
            Ok(outcome) => match suspends_foreign_keys
                .then(|| verify_sqlite_foreign_keys(&self.conn))
                .transpose()
            {
                Ok(_) => self
                    .conn
                    .execute("COMMIT", [])
                    .map(|_| outcome)
                    .map_err(DrizzleError::from),
                Err(error) => {
                    let _ = self.conn.execute("ROLLBACK", []);
                    Err(error)
                }
            },
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK", []);
                Err(e)
            }
        };
        let restore = if suspend_foreign_keys {
            set_sqlite_foreign_keys(&self.conn, true)
        } else {
            Ok(())
        };
        super::finish_foreign_key_scope(result, restore)
    }
}

fn load_applied_migration_names(
    conn: &rusqlite::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<Vec<String>> {
    let mut statement = conn.prepare(&set.applied_names_sql())?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn set_sqlite_foreign_keys(
    conn: &rusqlite::Connection,
    enabled: bool,
) -> drizzle_core::error::Result<()> {
    conn.execute_batch(if enabled {
        "PRAGMA foreign_keys=ON"
    } else {
        "PRAGMA foreign_keys=OFF"
    })?;
    let actual = conn.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))? != 0;
    if actual != enabled {
        return Err(DrizzleError::Other(
            format!(
                "SQLite refused to set foreign_keys={} outside the migration transaction",
                if enabled { "ON" } else { "OFF" }
            )
            .into(),
        ));
    }
    Ok(())
}

fn verify_sqlite_foreign_keys(conn: &rusqlite::Connection) -> drizzle_core::error::Result<()> {
    let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = statement.query([])?;
    if rows.next()?.is_some() {
        return Err(DrizzleError::Other(
            "SQLite foreign_key_check failed after migration rebuild".into(),
        ));
    }
    Ok(())
}

/// Read `sqlite_master` into a repair [`Catalog`](drizzle_migrations::repair::Catalog).
fn introspect_catalog(
    conn: &rusqlite::Connection,
) -> drizzle_core::error::Result<drizzle_migrations::repair::Catalog> {
    let mut statement = conn.prepare(drizzle_migrations::repair::sqlite::OBJECTS_QUERY)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(drizzle_migrations::repair::sqlite::catalog(&rows))
}

/// Reject or reconcile interrupted migrations. Returns the repaired tags.
fn repair_dirty_migrations(
    conn: &rusqlite::Connection,
    set: &drizzle_migrations::Migrations,
    repair: bool,
) -> drizzle_core::error::Result<Vec<String>> {
    let dirty = {
        let mut statement = conn.prepare(&set.dirty_names_sql())?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<String>, _>>()?
    };

    if dirty.is_empty() {
        return Ok(Vec::new());
    }

    let migrator_error =
        |error: drizzle_migrations::MigratorError| DrizzleError::Other(error.to_string().into());

    if !repair {
        return Err(migrator_error(
            set.interrupted_migration_error(&dirty)
                .expect("dirty list is non-empty"),
        ));
    }
    super::reject_unsafe_dirty_rebuild_repair(set, &dirty)?;

    let table_ident = set.table_ident_sql();
    let mut repaired = Vec::new();
    for migration in set
        .resolve_dirty_migrations(&dirty)
        .map_err(migrator_error)?
    {
        let catalog = introspect_catalog(conn)?;
        let plan =
            drizzle_migrations::repair::plan(drizzle_types::Dialect::SQLite, migration, &catalog);
        for statement in plan.into_executable(&table_ident).map_err(migrator_error)? {
            conn.execute(&statement, [])?;
        }
        conn.execute(&set.record_migration_finished_sql(migration), [])?;
        repaired.push(migration.tag().to_string());
    }

    Ok(repaired)
}

fn ensure_sqlite_migration_table(
    conn: &rusqlite::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    conn.execute(&set.create_table_sql(), [])?;

    let table_name = set.table_name().replace('\'', "''");
    let pragma_sql = format!("SELECT name FROM pragma_table_info('{table_name}')");
    let mut stmt = conn.prepare(&pragma_sql)?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if columns.iter().any(|column| column == "name") {
        return Ok(());
    }

    let mut stmt = conn.prepare(&format!(
        "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
        set.table_ident_sql()
    ))?;
    let applied = stmt
        .query_map([], |row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row.get::<_, Option<i64>>(0)?,
                hash: row.get::<_, String>(1)?,
                created_at: row.get::<_, i64>(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))?;

    conn.execute("BEGIN", [])?;
    let result = (|| -> drizzle_core::error::Result<()> {
        conn.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"name\" text",
                set.table_ident_sql()
            ),
            [],
        )?;
        conn.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"applied_at\" TEXT",
                set.table_ident_sql()
            ),
            [],
        )?;

        for row in matched {
            conn.execute(&set.backfill_migration_metadata_sql(&row), [])?;
        }

        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute("COMMIT", [])?;
            Ok(())
        }
        Err(err) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(err)
        }
    }
}

fn introspect_query_tables(
    conn: &rusqlite::Connection,
) -> drizzle_core::error::Result<Vec<(String, Option<String>)>> {
    use drizzle_migrations::sqlite::introspect::queries;
    let mut tables_stmt = conn.prepare(queries::TABLES_QUERY)?;
    let tables: Vec<(String, Option<String>)> = tables_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tables)
}

fn introspect_query_columns(
    conn: &rusqlite::Connection,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::sqlite::introspect::RawColumnInfo>> {
    use drizzle_migrations::sqlite::introspect::{RawColumnInfo, queries};
    let mut columns_stmt = conn.prepare(queries::COLUMNS_QUERY)?;
    let raw_columns: Vec<RawColumnInfo> = columns_stmt
        .query_map([], |row| {
            Ok(RawColumnInfo {
                table: row.get(0)?,
                cid: row.get(1)?,
                name: row.get(2)?,
                column_type: row.get(3)?,
                not_null: row.get(4)?,
                default_value: row.get(5)?,
                pk: row.get(6)?,
                hidden: row.get(7)?,
                sql: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(raw_columns)
}

fn introspect_query_indexes_and_fks(
    conn: &rusqlite::Connection,
) -> drizzle_core::error::Result<(
    Vec<drizzle_migrations::sqlite::introspect::RawIndexInfo>,
    Vec<drizzle_migrations::sqlite::introspect::RawIndexColumn>,
    Vec<drizzle_migrations::sqlite::introspect::RawForeignKey>,
)> {
    use drizzle_migrations::sqlite::introspect::{
        RawForeignKey, RawIndexColumn, RawIndexInfo, queries,
    };

    let mut index_stmt = conn.prepare(queries::INDEXES_QUERY)?;
    let all_indexes = index_stmt
        .query_map([], |row| {
            Ok(RawIndexInfo {
                table: row.get(0)?,
                name: row.get(1)?,
                unique: row.get::<_, i32>(2)? != 0,
                origin: row.get(3)?,
                partial: row.get::<_, i32>(4)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut index_columns_stmt = conn.prepare(queries::INDEX_COLUMNS_QUERY)?;
    let all_index_columns = index_columns_stmt
        .query_map([], |row| {
            Ok(RawIndexColumn {
                index_name: row.get(0)?,
                seqno: row.get(1)?,
                cid: row.get(2)?,
                name: row.get(3)?,
                desc: row.get::<_, i32>(4)? != 0,
                coll: row.get(5)?,
                key: row.get::<_, i32>(6)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut foreign_keys_stmt = conn.prepare(queries::FOREIGN_KEYS_QUERY)?;
    let all_fks = foreign_keys_stmt
        .query_map([], |row| {
            Ok(RawForeignKey {
                table: row.get(0)?,
                id: row.get(1)?,
                seq: row.get(2)?,
                to_table: row.get(3)?,
                from_column: row.get(4)?,
                to_column: row.get(5)?,
                on_update: row.get(6)?,
                on_delete: row.get(7)?,
                r#match: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok((all_indexes, all_index_columns, all_fks))
}

fn introspect_query_views(
    conn: &rusqlite::Connection,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::sqlite::introspect::RawViewInfo>> {
    use drizzle_migrations::sqlite::introspect::{RawViewInfo, queries};

    let mut all_views: Vec<RawViewInfo> = Vec::new();
    let mut views_stmt = conn.prepare(queries::VIEWS_QUERY)?;
    let view_iter = views_stmt.query_map([], |row| {
        Ok(RawViewInfo {
            name: row.get(0)?,
            sql: row.get(1)?,
        })
    })?;
    all_views.extend(view_iter.collect::<Result<Vec<_>, _>>()?);
    Ok(all_views)
}

fn introspect_query_index_sql(
    conn: &rusqlite::Connection,
) -> drizzle_core::error::Result<Vec<(String, String)>> {
    use drizzle_migrations::sqlite::introspect::queries;

    let mut index_sql_stmt = conn.prepare(queries::INDEX_SQL_QUERY)?;
    let rows = index_sql_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Reads the live database's schema into a
    /// [`Snapshot`](drizzle_migrations::schema::Snapshot).
    ///
    /// Queries `sqlite_master` and SQLite's `PRAGMA` tables to rebuild every
    /// table, column, index, foreign key, and view, and returns them as
    /// `Snapshot::Sqlite(..)`.
    ///
    /// # Errors
    ///
    /// Returns an error when one of the catalog queries fails.
    pub fn introspect(&self) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        let tables = introspect_query_tables(&self.conn)?;
        let raw_columns = introspect_query_columns(&self.conn)?;
        let (all_indexes, all_index_columns, all_fks) =
            introspect_query_indexes_and_fks(&self.conn)?;
        let all_views = introspect_query_views(&self.conn)?;
        let index_sql = introspect_query_index_sql(&self.conn)?;

        let ddl = drizzle_migrations::sqlite::introspect::assemble_ddl(
            drizzle_migrations::sqlite::introspect::RawIntrospection {
                tables,
                columns: raw_columns,
                indexes: all_indexes,
                index_columns: all_index_columns,
                foreign_keys: all_fks,
                views: all_views,
                index_sql,
            },
        );

        let mut snapshot = drizzle_migrations::sqlite::SQLiteSnapshot::new();
        for entity in ddl.to_entities() {
            snapshot.add_entity(entity);
        }

        Ok(drizzle_migrations::schema::Snapshot::Sqlite(snapshot))
    }

    /// Changes the live database to match `schema`, without migration files.
    ///
    /// Introspects the database, diffs it against `schema`, and runs the
    /// resulting statements in one transaction. Does nothing when they already
    /// match. Meant for local development: nothing is recorded in the
    /// migration tracking table.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::Schema;
    /// use drizzle::sqlite::rusqlite::Drizzle;
    ///
    /// let (db, schema) = Drizzle::<Schema>::new(rusqlite::Connection::open_in_memory()?);
    /// db.push(&schema)?; // creates the tables
    /// db.push(&schema)?; // already in sync: no-op
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// When a table or column may have been renamed, `push` fails rather
    /// than guess, and the error gives the hint for each answer;
    /// pass the answers to [`push_with`](Self::push_with).
    ///
    /// # Errors
    ///
    /// Returns an error when introspection or diffing fails, or when a
    /// statement fails (the transaction is rolled back).
    pub fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        self.push_with(schema, &drizzle_migrations::RenameHints::new())
    }

    /// [`push`](Self::push), with answers to its rename-or-create questions.
    ///
    /// # Errors
    ///
    /// Same as [`push`](Self::push).
    pub fn push_with<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
        renames: &drizzle_migrations::RenameHints,
    ) -> drizzle_core::error::Result<()> {
        let live = self.introspect()?;
        let desired = schema.to_snapshot();
        let generated = drizzle_migrations::diff_with(
            &live,
            &desired,
            &drizzle_migrations::DiffOptions::new().with_renames(renames.clone()),
        )
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let operation =
            drizzle_migrations::Migration::with_hash("push", "", 0, generated.statements);
        let execution = operation
            .sqlite_execution()
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
        let foreign_keys_were_enabled = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))?
            != 0;
        let suspend_foreign_keys = execution.suspends_foreign_keys() && foreign_keys_were_enabled;
        if suspend_foreign_keys && let Err(error) = set_sqlite_foreign_keys(&self.conn, false) {
            let restore = set_sqlite_foreign_keys(&self.conn, true);
            return super::finish_foreign_key_scope(Err(error), restore);
        }

        let result = (|| -> drizzle_core::error::Result<()> {
            self.conn.execute("BEGIN IMMEDIATE", [])?;
            for statement in execution
                .statements()
                .filter(|statement| !statement.trim().is_empty())
            {
                self.conn.execute(statement, [])?;
            }
            if execution.suspends_foreign_keys() {
                verify_sqlite_foreign_keys(&self.conn)?;
            }
            self.conn.execute("COMMIT", [])?;
            Ok(())
        })();
        let result = match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = self.conn.execute("ROLLBACK", []);
                Err(error)
            }
        };
        let restore = if suspend_foreign_keys {
            set_sqlite_foreign_keys(&self.conn, true)
        } else {
            Ok(())
        };
        super::finish_foreign_key_scope(result, restore)
    }
}

// =============================================================================
// Query API: find_many / find_first
// =============================================================================

#[cfg(feature = "query")]
use drizzle_core::query::DeserializeStore as _;
#[cfg(feature = "query")]
use drizzle_core::query::FromJsonObject as _;

#[cfg(feature = "query")]
impl common::private::Sealed for Connection {}

// Positional rows: base columns decode via `TryFrom<&Row>` by index.
#[cfg(feature = "query")]
impl common::QueryRowFormat for Connection {
    const WRAP_BASE_JSON: bool = false;
}

/// Runs an `AllColumns` relational query on `conn` and decodes every row.
///
/// Shared by the `&Drizzle` and `&Transaction` runner impls (a
/// `rusqlite::Transaction` derefs to `Connection`).
#[cfg(feature = "query")]
pub(crate) fn relational_find_many<'a, T, Rels, Cl>(
    conn: &Connection,
    builder: drizzle_core::query::QueryBuilder<
        'a,
        SQLiteValue<'a>,
        T,
        Rels,
        drizzle_core::query::AllColumns,
        Cl,
    >,
) -> drizzle_core::error::Result<
    Vec<<Rels as drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>>::Row>,
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
    let num_base_cols = T::COLUMN_NAMES.len();

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

    let mut stmt = conn
        .prepare(&sql)
        .with_query(|| QueryContext::new(&sql, &bind_params))?;
    let mut raw_rows = stmt
        .query(params_from_iter(bind_params.iter().copied()))
        .with_query(|| QueryContext::new(&sql, &bind_params))?;
    let mut results = Vec::new();

    while let Some(row) = raw_rows
        .next()
        .with_query(|| QueryContext::new(&sql, &bind_params))?
    {
        let base =
            <T as drizzle_core::query::QueryTable>::Select::try_from(row).map_err(Into::into)?;

        let mut rel_col = num_base_cols;
        let mut next_rel = || {
            let json: Option<String> = row
                .get(rel_col)
                .map_err(drizzle_core::error::DrizzleError::from)?;
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

// AllColumns: read base from individual row columns via TryFrom<Row>
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        relational_find_many(&self.runner.conn, self.builder)
    }
}

// AllColumns find_first: requires no LIMIT set yet (internally adds LIMIT 1)
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
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

/// Runs a `PartialColumns` relational query on `conn` and decodes every row.
///
/// Shared by the `&Drizzle` and `&Transaction` runner impls. Base columns are
/// deserialized from a JSON `"__base"` column.
#[cfg(feature = "query")]
pub(crate) fn relational_find_many_partial<'a, T, Rels, Cl>(
    conn: &Connection,
    builder: drizzle_core::query::QueryBuilder<
        'a,
        SQLiteValue<'a>,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    >,
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

    let mut stmt = conn
        .prepare(&sql)
        .with_query(|| QueryContext::new(&sql, &bind_params))?;
    let mut raw_rows = stmt
        .query(params_from_iter(bind_params.iter().copied()))
        .with_query(|| QueryContext::new(&sql, &bind_params))?;
    let mut results = Vec::new();

    while let Some(row) = raw_rows
        .next()
        .with_query(|| QueryContext::new(&sql, &bind_params))?
    {
        // Column 0 is the JSON "__base" object
        let base_json: String = row.get(0)?;
        let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
            &base_json, "base",
        )?;

        let mut rel_col = 1usize;
        let mut next_rel = || {
            let json: Option<String> = row
                .get(rel_col)
                .map_err(drizzle_core::error::DrizzleError::from)?;
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

// PartialColumns: read base from a single JSON "__base" column via FromJsonObject
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
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
        relational_find_many_partial(&self.runner.conn, self.builder)
    }
}

// PartialColumns find_first: requires no LIMIT set yet
#[cfg(feature = "query")]
impl<'db, 'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        'db,
        'a,
        &'db Drizzle<Schema>,
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

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Connection, T, Rels, drizzle_core::query::AllColumns>
{
    /// Runs the prepared relational query with `params` bound and returns
    /// every root row with its loaded relations.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub fn find_many<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
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
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let num_base_cols = T::COLUMN_NAMES.len();
        let (sql_str, params) = self.inner.bind(params)?;
        let mut stmt = conn.prepare_cached(sql_str)?;
        let mut raw_rows = stmt.query(params_from_iter(params))?;
        let mut results = Vec::new();

        while let Some(row) = raw_rows.next()? {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col).map_err(DrizzleError::from)?;
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

    /// Runs the prepared relational query and returns its first root row, or
    /// `None` when nothing matches.
    ///
    /// Every matching row is still fetched; call `.limit(1)` before
    /// `.prepare()` to limit the query itself.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub fn find_first<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
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
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(conn, params)?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Connection, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Runs the prepared relational query with `params` bound and returns
    /// every root row with its loaded relations.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub fn find_many<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
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
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::PartialSelect>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let (sql_str, params) = self.inner.bind(params)?;
        let mut stmt = conn.prepare_cached(sql_str)?;
        let mut raw_rows = stmt.query(params_from_iter(params))?;
        let mut results = Vec::new();

        while let Some(row) = raw_rows.next()? {
            let base_json: String = row.get(0)?;
            let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
                &base_json, "base",
            )?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col).map_err(DrizzleError::from)?;
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

    /// Runs the prepared relational query and returns its first root row, or
    /// `None` when nothing matches.
    ///
    /// Every matching row is still fetched; call `.limit(1)` before
    /// `.prepare()` to limit the query itself.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub fn find_first<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
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
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::PartialSelect>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(conn, params)?.into_iter().next())
    }
}

impl<S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'_, S, QueryBuilder<'_, Schema, State, Table, Mk, Rw, Grouped>, State>
{
    /// Runs the statement and returns the number of rows it changed.
    ///
    /// Use it for `INSERT`, `UPDATE`, and `DELETE`. A statement with a
    /// `RETURNING` clause still runs to completion; its returned rows are
    /// counted, not decoded (use [`all`](Self::all) to read them).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// use drizzle::core::expr::eq;
    /// # let (db, Schema { users, .. }) = app::database()?;
    ///
    /// let inserted = db.insert(users).value(InsertUsers::new("Dana", 41)).execute()?;
    /// assert_eq!(inserted, 1);
    ///
    /// let changed = db
    ///     .update(users)
    ///     .set(UpdateUsers::default().with_age(42))
    ///     .r#where(eq(users.name, "Dana"))
    ///     .execute()?;
    /// assert_eq!(changed, 1);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the statement, for
    /// example on a constraint violation. The error carries the SQL and its
    /// parameters ([`DrizzleError::QueryFailed`]).
    pub fn execute(self) -> drizzle_core::error::Result<usize>
    where
        State: builder::ExecutableState,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        execute_sql(
            &self.runner.conn,
            &sql_str,
            params_from_iter(params.iter().copied()),
            self.builder.sql.has_returning(),
        )
        .with_query(|| QueryContext::new(&sql_str, &params))
    }

    /// Runs the query and decodes every row into `R`.
    ///
    /// `R` is usually the generated `Select*` model (for `select(())`), a
    /// tuple matching the selected columns, or a type deriving
    /// [`SQLiteFromRow`](crate::sqlite::SQLiteFromRow).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// use drizzle::core::expr::gt;
    /// # let (db, Schema { users, posts, .. }) = app::database()?;
    ///
    /// let adults: Vec<SelectUsers> = db.select(()).from(users).r#where(gt(users.age, 18)).all()?;
    /// assert_eq!(adults.len(), 2);
    ///
    /// // After a LEFT JOIN, the joined table's columns decode as `Option`.
    /// let rows: Vec<(String, Option<String>)> = db
    ///     .select((users.name, posts.title))
    ///     .from(users)
    ///     .left_join(posts)
    ///     .all()?;
    /// # let _ = rows;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the query, or when a
    /// row cannot be decoded into `R`. Query errors carry the SQL and its
    /// parameters ([`DrizzleError::QueryFailed`]).
    ///
    /// # Compile-time checks
    ///
    /// The call does not compile unless:
    ///
    /// - every column the query reads (in `SELECT`, `WHERE`, `ORDER BY`, ...)
    ///   belongs to a table in its `FROM`/`JOIN` list;
    /// - `R` matches the selection: one field per selected column, with a
    ///   compatible type, and `Option<T>` wherever the value can be `NULL`
    ///   (a nullable column, or any column of an outer-joined table);
    /// - with `GROUP BY`, each column in a selected tuple is grouped or
    ///   aggregated;
    /// - a raw `sql!` selection carries an explicit result type.
    ///
    /// ```compile_fail,E0277
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # fn main() -> drizzle::Result<()> {
    /// # let (db, Schema { users, posts, .. }) = app::database()?;
    /// // `posts` is never joined, so `posts.title` is out of scope.
    /// let titles: Vec<String> = db.select(posts.title).from(users).all()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        State: builder::ExecutableState,
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self
            .runner
            .conn
            .prepare(&sql_str)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let mut raw_rows = stmt
            .query(params_from_iter(params.iter().copied()))
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let mut decoded = Vec::new();
        while let Some(row) = raw_rows
            .next()
            .with_query(|| QueryContext::new(&sql_str, &params))?
        {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::rusqlite::Row<'_>,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns an iterator over its rows, decoded into the
    /// row type the query infers from its selection.
    ///
    /// Unlike [`all`](Self::all), you do not pick the row type: `select(())`
    /// yields the table's `Select*` model and a column tuple yields a tuple.
    /// Every row is fetched and decoded before this returns.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # let (db, Schema { users, .. }) = app::database()?;
    /// for row in db.select((users.id, users.name)).from(users).rows()? {
    ///     let (id, name) = row?;
    ///     println!("{id}: {name}");
    /// }
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite cannot prepare or run the query, or when a
    /// row cannot be decoded.
    ///
    /// # Compile-time checks
    ///
    /// The same scope and grouping checks as [`all`](Self::all).
    pub fn rows<Proof, AggProof>(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        State: builder::ExecutableState,
        for<'r> Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, Rw, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
        Rw: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <Rw as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.rows");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self
            .runner
            .conn
            .prepare(&sql_str)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let mut rows = stmt
            .query_and_then(params_from_iter(params.iter().copied()), |row| {
                Rw::try_from(row).map_err(Into::into)
            })
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let (lower, _) = rows.size_hint();
        let mut decoded = Vec::with_capacity(lower);
        for row in rows {
            decoded.push(row?);
        }

        Ok(Rows::new(decoded))
    }

    /// Runs the query and decodes its first row into `R`.
    ///
    /// Rows after the first are ignored; add `.limit(1)` if the query could
    /// match many.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// use drizzle::core::expr::{count, eq};
    /// # let (db, Schema { users, .. }) = app::database()?;
    ///
    /// let alice: SelectUsers = db.select(()).from(users).r#where(eq(users.name, "Alice")).get()?;
    /// assert_eq!(alice.age, 30);
    ///
    /// let (total,): (i64,) = db.select((count(users.id),)).from(users).get()?;
    /// assert_eq!(total, 3);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when no row matches (rusqlite's
    /// `QueryReturnedNoRows`), when SQLite cannot prepare or run the query, or
    /// when the row cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The same scope, `NULL`, and grouping checks as [`all`](Self::all).
    pub fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        State: builder::ExecutableState,
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self
            .runner
            .conn
            .prepare(&sql_str)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        stmt.query_row(params_from_iter(params.iter().copied()), |row| {
            Ok(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::rusqlite::Row<'_>,
                R,
            >>::decode(row))
        })
        .with_query(|| QueryContext::new(&sql_str, &params))?
    }
}
