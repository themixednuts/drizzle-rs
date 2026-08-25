//! Async `SQLite` driver using [`turso`].
//!
//! # Quick start
//!
//! ```no_run
//! use drizzle::sqlite::turso::Drizzle;
//! use drizzle::sqlite::prelude::*;
//! use turso::Builder;
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
//! #[tokio::main]
//! async fn main() -> drizzle::Result<()> {
//!     let db_builder = Builder::new_local(":memory:").build().await?;
//!     let conn = db_builder.connect()?;
//!     let (db, AppSchema { user }) = Drizzle::new(conn, AppSchema::new());
//!     db.create().await?;
//!
//!     // Insert
//!     db.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!
//!     // Select
//!     let users: Vec<SelectUser> = db.select(()).from(user).all().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Transactions
//!
//! Return `Ok(value)` to commit, `Err(...)` to rollback.
//!
//! ```no_run
//! # use drizzle::sqlite::turso::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use turso::Builder;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let db_builder = Builder::new_local(":memory:").build().await?;
//! # let conn = db_builder.connect()?;
//! # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
//! use drizzle::sqlite::connection::SQLiteTransactionType;
//!
//! let count = db.transaction(SQLiteTransactionType::Deferred, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     Ok(users.len())
//! }).await?;
//! # Ok(()) }
//! ```
//!
//! # Savepoints
//!
//! Savepoints nest inside transactions — a failed savepoint rolls back
//! without aborting the outer transaction.
//!
//! ```no_run
//! # use drizzle::sqlite::turso::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::sqlite::connection::SQLiteTransactionType;
//! # use turso::Builder;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let db_builder = Builder::new_local(":memory:").build().await?;
//! # let conn = db_builder.connect()?;
//! # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
//! db.transaction(SQLiteTransactionType::Deferred, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!
//!     // This savepoint fails — only its changes roll back
//!     let _: Result<(), _> = tx.savepoint(async |stx| {
//!         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
//!         Err(drizzle::error::DrizzleError::Other("oops".into()))
//!     }).await;
//!
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     assert_eq!(users.len(), 1); // only Alice
//!     Ok(())
//! }).await?;
//! # Ok(()) }
//! ```
//!
//! # Cloning for `tokio::spawn`
//!
//! `Drizzle` is cheaply cloneable — the underlying connection is shared.
//! Move a clone into spawned tasks for concurrent queries.
//!
//! ```no_run
//! # use drizzle::sqlite::turso::Drizzle;
//! # use drizzle::sqlite::prelude::*;
//! # use turso::Builder;
//! # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
//! # #[derive(SQLiteSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let db_builder = Builder::new_local(":memory:").build().await?;
//! # let conn = db_builder.connect()?;
//! # let (db, S { user, .. }) = Drizzle::new(conn, S::new());
//! let db_clone = db.clone();
//! tokio::spawn(async move {
//!     db_clone
//!         .insert(user)
//!         .values([InsertUser::new("Bob")])
//!         .execute()
//!         .await
//!         .expect("insert from task");
//! }).await.unwrap();
//! # Ok(()) }
//! ```

pub(crate) mod prepared;

use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use futures_util::FutureExt;
use turso::{Connection, IntoValue, Row};

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{self, QueryBuilder},
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

crate::drizzle_prepare_impl!();
use crate::builder::sqlite::common;
use crate::builder::sqlite::rows::TursoRows as Rows;
use crate::transaction::sqlite::turso::Transaction;

pub type Drizzle<Schema = ()> = common::Drizzle<Connection, Schema>;
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, common::Drizzle<Connection, Schema>, Schema, Builder, State>;

async fn turso_execute_cached(
    conn: &Connection,
    sql: &str,
    params: Vec<turso::Value>,
) -> turso::Result<u64> {
    let mut stmt = conn.prepare_cached(sql).await?;
    stmt.execute(params).await
}

async fn turso_query_cached(
    conn: &Connection,
    sql: &str,
    params: Vec<turso::Value>,
) -> turso::Result<turso::Rows> {
    let mut stmt = conn.prepare_cached(sql).await?;
    stmt.query(params).await
}

pub(crate) async fn turso_decode_first_and_finish<R>(
    rows: &mut turso::Rows,
    decode: impl FnOnce(&Row) -> drizzle_core::error::Result<R>,
) -> drizzle_core::error::Result<R> {
    let decoded = match rows.next().await? {
        Some(row) => decode(&row),
        None => Err(DrizzleError::NotFound),
    };

    // Turso owns the live statement through `Rows`. Step it to `Done` before
    // returning so a one-row read or `RETURNING` query cannot retain a read or
    // write transaction until the cursor is eventually dropped.
    while rows.next().await?.is_some() {}
    decoded
}

impl<Schema> common::Drizzle<Connection, Schema> {
    pub async fn execute<'a, T>(
        &'a self,
        query: T,
    ) -> Result<u64, drizzle_core::error::DrizzleError>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let (sql_str, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(|p| {
                p.into_value()
                    .map_err(drizzle_core::error::DrizzleError::from)
            })
            .collect::<Result<Vec<_>, _>>()
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        turso_execute_cached(&self.conn, &sql_str, driver_params)
            .await
            .map_err(drizzle_core::error::DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: Default + Extend<R>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(|p| p.into_value().map_err(DrizzleError::from))
            .collect::<Result<Vec<_>, _>>()
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let mut rows = turso_query_cached(&self.conn, &sql_str, driver_params)
            .await
            .map_err(DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let mut out = C::default();
        while let Some(row) = rows
            .next()
            .await
            .map_err(DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?
        {
            out.extend(core::iter::once(R::try_from(&row).map_err(Into::into)?));
        }
        Ok(out)
    }

    /// Runs the query and returns a row cursor.
    pub async fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(|p| p.into_value().map_err(DrizzleError::from))
            .collect::<Result<Vec<_>, _>>()
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let rows = turso_query_cached(&self.conn, &sql_str, driver_params)
            .await
            .map_err(DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(|p| p.into_value().map_err(DrizzleError::from))
            .collect::<Result<Vec<_>, _>>()
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        let mut rows = turso_query_cached(&self.conn, &sql_str, driver_params)
            .await
            .map_err(DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        match turso_decode_first_and_finish(&mut rows, |row| R::try_from(row).map_err(Into::into))
            .await
        {
            Err(DrizzleError::NotFound) => Err(DrizzleError::NotFound),
            result => result.with_query(|| QueryContext::new(&sql_str, &params)),
        }
    }

    /// Executes a transaction with the given callback.
    ///
    /// The transaction is committed when the callback returns `Ok` and
    /// rolled back on `Err`.
    ///
    /// ```no_run
    /// # use drizzle::sqlite::turso::Drizzle;
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::connection::SQLiteTransactionType;
    /// # use turso::Builder;
    /// # #[SQLiteTable] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct S { user: User }
    /// # #[tokio::main] async fn main() -> drizzle::Result<()> {
    /// # let db_builder = Builder::new_local(":memory:").build().await?;
    /// # let conn = db_builder.connect()?;
    /// # let (mut db, S { user, .. }) = Drizzle::new(conn, S::new());
    /// let count = db.transaction(SQLiteTransactionType::Deferred, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     Ok(users.len())
    /// }).await?;
    /// # Ok(()) }
    /// ```
    pub async fn transaction<F, R>(
        &mut self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: AsyncFnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        drizzle_core::drizzle_trace_tx!("begin", "sqlite.turso");
        let tx = self.conn.transaction_with_behavior(tx_type.into()).await?;
        let transaction = Transaction::new(tx, tx_type, self.schema);

        let outcome = std::panic::AssertUnwindSafe(f(&transaction))
            .catch_unwind()
            .await;

        match outcome {
            Ok(Ok(result)) => {
                drizzle_core::drizzle_trace_tx!("commit", "sqlite.turso");
                transaction.commit().await?;
                Ok(result)
            }
            Ok(Err(e)) => {
                drizzle_core::drizzle_trace_tx!("rollback", "sqlite.turso");
                let _ = transaction.rollback().await;
                Err(e)
            }
            Err(panic_payload) => {
                drizzle_core::drizzle_trace_tx!("rollback", "sqlite.turso");
                let _ = transaction.rollback().await;
                std::panic::resume_unwind(panic_payload);
            }
        }
    }
}

impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects from `SQLSchemaImpl`.
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements()?;
        for sql in statements {
            self.conn.execute(&sql, ()).await?;
        }
        Ok(())
    }
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// # Two-phase tracking
    ///
    /// turso is beta and its crash-recovery guarantees around in-transaction
    /// DDL are not something to bet a production schema on (0.7 also stopped
    /// auto-rolling-back on `Drop`, which is why [`Self::transaction`]
    /// compensates with an explicit rollback). So this path does **not** assume
    /// that wrapping statements and the tracking insert in one transaction
    /// makes them atomic. Each pending migration runs as:
    ///
    /// 1. insert the tracking row with `applied_at` NULL (autocommit — the
    ///    migration is now marked **dirty**);
    /// 2. run the migration's statements inside an immediate transaction
    ///    (still worth having: when turso's recovery does work, a crash here
    ///    rolls the statements back);
    /// 3. update the row to set `applied_at` (autocommit — the migration is
    ///    now **applied**).
    ///
    /// A process killed between 1 and 3 therefore leaves a dirty row rather
    /// than an untracked partial schema, and the next `migrate()` reports
    /// exactly which migration was interrupted instead of blindly re-running
    /// its DDL. Use [`Self::migrate_with_repair`] to reconcile it.
    ///
    /// If step 2 fails on its *first* statement nothing can have been applied,
    /// so the dirty marker is dropped and the original error is returned
    /// unchanged.
    pub async fn migrate(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        self.migrate_inner(migrations, tracking, false).await
    }

    /// Apply pending migrations, first reconciling any interrupted migration.
    ///
    /// For each migration marked dirty by the two-phase flow in
    /// [`Self::migrate`], this introspects `sqlite_master` and classifies every
    /// statement: `CREATE TABLE` / `CREATE [UNIQUE] INDEX` / `CREATE VIEW`
    /// statements whose object already exists with a matching definition are
    /// skipped, and the rest are executed. Anything that cannot be proven
    /// either way aborts with a list of what needs manual resolution.
    pub async fn migrate_with_repair(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        self.migrate_inner(migrations, tracking, true).await
    }

    async fn migrate_inner(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
        repair: bool,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::SQLite,
            tracking,
        );
        ensure_sqlite_migration_table(&mut self.conn, &set).await?;
        let mut applied = repair_dirty_migrations(&self.conn, &set, repair).await?;

        let applied_names = load_applied_migration_names(&self.conn, &set).await?;
        let pending: Vec<_> = set
            .pending(&applied_names)
            .map(drizzle_migrations::Migration::clone)
            .collect();
        if pending.is_empty() {
            if applied.is_empty() {
                return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
            }
            return Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied });
        }
        pending
            .iter()
            .map(drizzle_migrations::Migration::sqlite_execution)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?;

        for migration in &pending {
            let execution = migration
                .sqlite_execution()
                .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
            let foreign_keys_were_enabled = sqlite_foreign_keys_enabled(&self.conn).await?;
            let suspend_foreign_keys =
                execution.suspends_foreign_keys() && foreign_keys_were_enabled;
            if suspend_foreign_keys
                && let Err(error) = set_sqlite_foreign_keys(&self.conn, false).await
            {
                let restore = set_sqlite_foreign_keys(&self.conn, true).await;
                return super::finish_foreign_key_scope(Err(error), restore);
            }

            let migration_result = async {
                // Phase 1: mark dirty before touching the schema.
                self.conn
                    .execute(&set.record_migration_started_sql(migration), ())
                    .await
                    .map_err(DrizzleError::from)?;

                // Phase 2: run the statements. The inner transaction is a best
                // effort — the dirty marker is what makes recovery possible.
                if let Err(error) = run_migration_statements(&mut self.conn, execution).await {
                    // Failing on the first statement means nothing was applied, so
                    // the marker would only demand a pointless repair.
                    if error.failed_first_statement {
                        let _ = self
                            .conn
                            .execute(&set.clear_migration_started_sql(migration), ())
                            .await;
                    }
                    return Err(error.error);
                }
                Ok(())
            }
            .await;
            let restore_result = if suspend_foreign_keys {
                set_sqlite_foreign_keys(&self.conn, true).await
            } else {
                Ok(())
            };
            super::finish_foreign_key_scope(migration_result, restore_result)?;

            // Phase 3: the migration is complete.
            self.conn
                .execute(&set.record_migration_finished_sql(migration), ())
                .await
                .map_err(DrizzleError::from)?;
            applied.push(migration.tag().to_string());
        }

        Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied })
    }
}

/// A failed statement run, plus whether it failed before anything could have
/// been applied.
struct StatementRunError {
    error: DrizzleError,
    failed_first_statement: bool,
}

/// Run one migration's statements inside an immediate transaction.
async fn run_migration_statements(
    conn: &mut turso::Connection,
    execution: drizzle_migrations::SqliteMigrationExecution<'_>,
) -> Result<(), StatementRunError> {
    let verify_foreign_keys = execution.suspends_foreign_keys();
    let tx = conn
        .transaction_with_behavior(turso::transaction::TransactionBehavior::Immediate)
        .await
        .map_err(|error| StatementRunError {
            error: DrizzleError::from(error),
            failed_first_statement: true,
        })?;

    let mut executed = 0usize;
    for stmt in execution.statements() {
        if stmt.trim().is_empty() {
            continue;
        }
        if let Err(error) = tx.execute(stmt, ()).await {
            let _ = tx.rollback().await;
            return Err(StatementRunError {
                error: DrizzleError::from(error),
                failed_first_statement: executed == 0,
            });
        }
        executed += 1;
    }

    if verify_foreign_keys {
        let check = async {
            let mut rows = tx
                .query("PRAGMA foreign_key_check", ())
                .await
                .map_err(DrizzleError::from)?;
            if rows.next().await.map_err(DrizzleError::from)?.is_some() {
                return Err(DrizzleError::Other(
                    "SQLite foreign_key_check failed after migration rebuild".into(),
                ));
            }
            Ok::<_, DrizzleError>(())
        }
        .await;
        if let Err(error) = check {
            let _ = tx.rollback().await;
            return Err(StatementRunError {
                error,
                failed_first_statement: false,
            });
        }
    }

    tx.commit().await.map_err(|error| StatementRunError {
        error: DrizzleError::from(error),
        // The statements ran; only the commit failed. Whether they landed is
        // exactly the thing turso's recovery does not promise, so keep the
        // dirty marker.
        failed_first_statement: false,
    })
}

async fn sqlite_foreign_keys_enabled(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<bool> {
    let mut rows = conn
        .query("PRAGMA foreign_keys", ())
        .await
        .map_err(DrizzleError::from)?;
    let row = rows
        .next()
        .await
        .map_err(DrizzleError::from)?
        .ok_or_else(|| DrizzleError::Other("PRAGMA foreign_keys returned no row".into()))?;
    Ok(row.get::<i64>(0).map_err(DrizzleError::from)? != 0)
}

async fn set_sqlite_foreign_keys(
    conn: &turso::Connection,
    enabled: bool,
) -> drizzle_core::error::Result<()> {
    conn.execute(
        if enabled {
            "PRAGMA foreign_keys=ON"
        } else {
            "PRAGMA foreign_keys=OFF"
        },
        (),
    )
    .await
    .map_err(DrizzleError::from)?;
    if sqlite_foreign_keys_enabled(conn).await? != enabled {
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

/// Read `sqlite_master` into a repair [`Catalog`](drizzle_migrations::repair::Catalog).
async fn introspect_catalog(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<drizzle_migrations::repair::Catalog> {
    let mut rows = conn
        .query(drizzle_migrations::repair::sqlite::OBJECTS_QUERY, ())
        .await
        .map_err(DrizzleError::from)?;

    let mut master_rows = Vec::new();
    while let Some(row) = rows.next().await.map_err(DrizzleError::from)? {
        master_rows.push((
            row.get::<String>(0).map_err(DrizzleError::from)?,
            row.get::<String>(1).map_err(DrizzleError::from)?,
            row.get::<Option<String>>(2).ok().flatten(),
        ));
    }
    Ok(drizzle_migrations::repair::sqlite::catalog(&master_rows))
}

/// Reject or reconcile interrupted migrations. Returns the repaired tags.
async fn repair_dirty_migrations(
    conn: &turso::Connection,
    set: &drizzle_migrations::Migrations,
    repair: bool,
) -> drizzle_core::error::Result<Vec<String>> {
    let mut rows = conn
        .query(&set.dirty_names_sql(), ())
        .await
        .map_err(DrizzleError::from)?;
    let mut dirty: Vec<String> = Vec::new();
    while let Some(row) = rows.next().await.map_err(DrizzleError::from)? {
        dirty.push(row.get::<String>(0).map_err(DrizzleError::from)?);
    }

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
        let catalog = introspect_catalog(conn).await?;
        let plan =
            drizzle_migrations::repair::plan(drizzle_types::Dialect::SQLite, migration, &catalog);
        for statement in plan.into_executable(&table_ident).map_err(migrator_error)? {
            conn.execute(&statement, ())
                .await
                .map_err(DrizzleError::from)?;
        }
        conn.execute(&set.record_migration_finished_sql(migration), ())
            .await
            .map_err(DrizzleError::from)?;
        repaired.push(migration.tag().to_string());
    }

    Ok(repaired)
}

async fn load_applied_migration_names(
    conn: &turso::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<Vec<String>> {
    let mut rows = conn
        .query(&set.applied_names_sql(), ())
        .await
        .map_err(DrizzleError::from)?;
    let mut applied_names = Vec::new();
    while let Some(row) = rows.next().await.map_err(DrizzleError::from)? {
        applied_names.push(row.get::<String>(0).map_err(DrizzleError::from)?);
    }
    Ok(applied_names)
}

async fn migration_table_exists(
    conn: &turso::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<bool> {
    let table_name = set.table_name().replace('\'', "''");
    let sql = format!(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table_name}' LIMIT 1"
    );
    let mut rows = conn.query(&sql, ()).await.map_err(DrizzleError::from)?;
    let exists = rows.next().await.map_err(DrizzleError::from)?.is_some();
    while rows.next().await.map_err(DrizzleError::from)?.is_some() {}
    Ok(exists)
}

async fn migration_table_has_name_column(
    conn: &turso::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<bool> {
    let table_name = set.table_name().replace('\'', "''");
    let pragma_sql = format!("SELECT name FROM pragma_table_info('{table_name}')");
    let mut rows = conn
        .query(&pragma_sql, ())
        .await
        .map_err(DrizzleError::from)?;
    while let Some(row) = rows.next().await.map_err(DrizzleError::from)? {
        if let Ok(name) = row.get::<String>(0)
            && name == "name"
        {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn load_legacy_applied_migrations(
    conn: &turso::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::AppliedMigrationMetadata>> {
    let mut rows = conn
        .query(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            (),
        )
        .await
        .map_err(DrizzleError::from)?;
    let mut applied = Vec::new();
    while let Some(row) = rows.next().await.map_err(DrizzleError::from)? {
        applied.push(drizzle_migrations::AppliedMigrationMetadata {
            id: row.get::<Option<i64>>(0).ok().flatten(),
            hash: row.get::<String>(1).map_err(DrizzleError::from)?,
            created_at: row.get::<i64>(2).map_err(DrizzleError::from)?,
        });
    }
    Ok(applied)
}

async fn backfill_migration_name_column(
    conn: &mut turso::Connection,
    set: &drizzle_migrations::Migrations,
    matched: Vec<drizzle_migrations::MatchedMigrationMetadata>,
) -> drizzle_core::error::Result<()> {
    let tx = conn.transaction().await.map_err(DrizzleError::from)?;
    tx.execute(
        &format!(
            "ALTER TABLE {} ADD COLUMN \"name\" text",
            set.table_ident_sql()
        ),
        (),
    )
    .await
    .map_err(DrizzleError::from)?;
    tx.execute(
        &format!(
            "ALTER TABLE {} ADD COLUMN \"applied_at\" TEXT",
            set.table_ident_sql()
        ),
        (),
    )
    .await
    .map_err(DrizzleError::from)?;

    for row in matched {
        tx.execute(&set.backfill_migration_metadata_sql(&row), ())
            .await
            .map_err(DrizzleError::from)?;
    }

    tx.commit().await.map_err(DrizzleError::from)?;
    Ok(())
}

async fn ensure_sqlite_migration_table(
    conn: &mut turso::Connection,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    if !migration_table_exists(conn, set).await? {
        conn.execute(&set.create_table_sql(), ())
            .await
            .map_err(DrizzleError::from)?;
    }

    if migration_table_has_name_column(conn, set).await? {
        return Ok(());
    }

    let applied = load_legacy_applied_migrations(conn, set).await?;
    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

    backfill_migration_name_column(conn, set, matched).await
}

async fn turso_introspect_query_tables(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<Vec<(String, Option<String>)>> {
    use drizzle_migrations::sqlite::introspect::queries;
    let err = DrizzleError::from;

    let mut tables_rows = conn.query(queries::TABLES_QUERY, ()).await.map_err(err)?;
    let mut tables: Vec<(String, Option<String>)> = Vec::new();
    while let Some(row) = tables_rows.next().await.map_err(err)? {
        let name: String = row.get(0).map_err(err)?;
        let sql: Option<String> = row.get(1).ok();
        tables.push((name, sql));
    }
    Ok(tables)
}

async fn turso_introspect_query_columns(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::sqlite::introspect::RawColumnInfo>> {
    use drizzle_migrations::sqlite::introspect::{RawColumnInfo, queries};
    let err = DrizzleError::from;

    let mut columns_rows = conn.query(queries::COLUMNS_QUERY, ()).await.map_err(err)?;
    let mut raw_columns: Vec<RawColumnInfo> = Vec::new();
    while let Some(row) = columns_rows.next().await.map_err(err)? {
        raw_columns.push(RawColumnInfo {
            table: row.get(0).map_err(err)?,
            cid: row.get(1).map_err(err)?,
            name: row.get(2).map_err(err)?,
            column_type: row.get(3).map_err(err)?,
            not_null: row.get::<i32>(4).map_err(err)? != 0,
            default_value: row.get(5).ok(),
            pk: row.get(6).map_err(err)?,
            hidden: row.get(7).map_err(err)?,
            sql: row.get(8).ok(),
        });
    }
    Ok(raw_columns)
}

async fn turso_introspect_query_indexes_and_fks(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<(
    Vec<drizzle_migrations::sqlite::introspect::RawIndexInfo>,
    Vec<drizzle_migrations::sqlite::introspect::RawIndexColumn>,
    Vec<drizzle_migrations::sqlite::introspect::RawForeignKey>,
)> {
    use drizzle_migrations::sqlite::introspect::{
        RawForeignKey, RawIndexColumn, RawIndexInfo, queries,
    };
    let err = DrizzleError::from;

    let mut all_indexes = Vec::<RawIndexInfo>::new();
    let mut index_rows = conn.query(queries::INDEXES_QUERY, ()).await.map_err(err)?;
    while let Some(row) = index_rows.next().await.map_err(err)? {
        all_indexes.push(RawIndexInfo {
            table: row.get(0).map_err(err)?,
            name: row.get(1).map_err(err)?,
            unique: row.get::<i32>(2).map_err(err)? != 0,
            origin: row.get(3).map_err(err)?,
            partial: row.get::<i32>(4).map_err(err)? != 0,
        });
    }

    let mut all_index_columns = Vec::<RawIndexColumn>::new();
    let mut column_rows = conn
        .query(queries::INDEX_COLUMNS_QUERY, ())
        .await
        .map_err(err)?;
    while let Some(row) = column_rows.next().await.map_err(err)? {
        all_index_columns.push(RawIndexColumn {
            index_name: row.get(0).map_err(err)?,
            seqno: row.get(1).map_err(err)?,
            cid: row.get(2).map_err(err)?,
            name: row.get(3).ok(),
            desc: row.get::<i32>(4).map_err(err)? != 0,
            coll: row.get(5).map_err(err)?,
            key: row.get::<i32>(6).map_err(err)? != 0,
        });
    }

    let mut all_fks = Vec::<RawForeignKey>::new();
    let mut foreign_key_rows = conn
        .query(queries::FOREIGN_KEYS_QUERY, ())
        .await
        .map_err(err)?;
    while let Some(row) = foreign_key_rows.next().await.map_err(err)? {
        all_fks.push(RawForeignKey {
            table: row.get(0).map_err(err)?,
            id: row.get(1).map_err(err)?,
            seq: row.get(2).map_err(err)?,
            to_table: row.get(3).map_err(err)?,
            from_column: row.get(4).map_err(err)?,
            to_column: row.get(5).map_err(err)?,
            on_update: row.get(6).map_err(err)?,
            on_delete: row.get(7).map_err(err)?,
            r#match: row.get(8).map_err(err)?,
        });
    }

    Ok((all_indexes, all_index_columns, all_fks))
}

async fn turso_introspect_query_views(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::sqlite::introspect::RawViewInfo>> {
    use drizzle_migrations::sqlite::introspect::{RawViewInfo, queries};

    let mut all_views: Vec<RawViewInfo> = Vec::new();
    let err = DrizzleError::from;
    let mut views_rows = conn.query(queries::VIEWS_QUERY, ()).await.map_err(err)?;
    while let Some(row) = views_rows.next().await.map_err(err)? {
        let name: String = row.get(0).map_err(err)?;
        let sql: String = row.get(1).map_err(err)?;
        all_views.push(RawViewInfo { name, sql });
    }
    Ok(all_views)
}

async fn turso_introspect_query_index_sql(
    conn: &turso::Connection,
) -> drizzle_core::error::Result<Vec<(String, String)>> {
    use drizzle_migrations::sqlite::introspect::queries;

    let mut index_sql = Vec::new();
    let err = DrizzleError::from;
    let mut rows = conn
        .query(queries::INDEX_SQL_QUERY, ())
        .await
        .map_err(err)?;
    while let Some(row) = rows.next().await.map_err(err)? {
        let name: String = row.get(0).map_err(err)?;
        let sql: String = row.get(1).map_err(err)?;
        index_sql.push((name, sql));
    }
    Ok(index_sql)
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Introspect the live database and return a [`Snapshot`] of its current schema.
    pub async fn introspect(
        &self,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        let tables = turso_introspect_query_tables(&self.conn).await?;
        let raw_columns = turso_introspect_query_columns(&self.conn).await?;
        let (all_indexes, all_index_columns, all_fks) =
            turso_introspect_query_indexes_and_fks(&self.conn).await?;
        let all_views = turso_introspect_query_views(&self.conn).await?;
        let index_sql = turso_introspect_query_index_sql(&self.conn).await?;

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

    /// Introspect the live database, diff against the desired schema, and
    /// execute the SQL statements needed to bring the database in sync.
    ///
    /// This is a no-op if the database already matches.
    pub async fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        let live = self.introspect().await?;
        let desired = schema.to_snapshot();
        let generated = drizzle_migrations::diff(&live, &desired)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let operation =
            drizzle_migrations::Migration::with_hash("push", "", 0, generated.statements);
        let execution = operation
            .sqlite_execution()
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
        let foreign_keys_were_enabled = sqlite_foreign_keys_enabled(&self.conn).await?;
        let suspend_foreign_keys = execution.suspends_foreign_keys() && foreign_keys_were_enabled;
        if suspend_foreign_keys && let Err(error) = set_sqlite_foreign_keys(&self.conn, false).await
        {
            let restore = set_sqlite_foreign_keys(&self.conn, true).await;
            return super::finish_foreign_key_scope(Err(error), restore);
        }

        let result = async {
            let tx = self
                .conn
                .unchecked_transaction()
                .await
                .map_err(DrizzleError::from)?;
            let transaction_result = async {
                for statement in execution
                    .statements()
                    .filter(|statement| !statement.trim().is_empty())
                {
                    tx.execute(statement, ())
                        .await
                        .map_err(DrizzleError::from)?;
                }
                if execution.suspends_foreign_keys() {
                    let mut rows = tx
                        .query("PRAGMA foreign_key_check", ())
                        .await
                        .map_err(DrizzleError::from)?;
                    if rows.next().await.map_err(DrizzleError::from)?.is_some() {
                        return Err(DrizzleError::Other(
                            "SQLite foreign_key_check failed after schema push".into(),
                        ));
                    }
                }
                Ok::<_, DrizzleError>(())
            }
            .await;
            match transaction_result {
                Ok(()) => tx.commit().await.map_err(DrizzleError::from),
                Err(error) => {
                    tx.rollback().await.ok();
                    Err(error)
                }
            }
        }
        .await;
        let restore = if suspend_foreign_keys {
            set_sqlite_foreign_keys(&self.conn, true).await
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

/// Runs an `AllColumns` relational query on `executor` and decodes every row.
///
/// Shared by the `&Drizzle` and `&Transaction` runner impls via
/// [`prepared::TursoExecutor`], which both `turso::Connection` and
/// `turso::transaction::Transaction` implement with cached statements.
#[cfg(feature = "query")]
pub(crate) async fn relational_find_many<'a, T, Rels, Cl>(
    executor: &impl prepared::TursoExecutor,
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
    <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
    for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
        Into<drizzle_core::error::DrizzleError>,
    Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
        + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
    <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
{
    let num_base_cols = T::COLUMN_NAMES.len();

    let mut rendered = Vec::new();
    builder.relations.render_into(&mut rendered);
    let query_sql = drizzle_core::query::build_query_sql(
        T::TABLE_NAME,
        T::COLUMN_NAMES,
        T::BLOB_COLUMNS,
        rendered,
        builder.where_sql,
        builder.order_by_sql,
        builder.limit,
        builder.offset,
        false,
    );
    let (sql, bind_params) = query_sql.build();
    drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

    let params: Vec<turso::Value> = bind_params
        .iter()
        .copied()
        .map(std::convert::Into::into)
        .collect();
    let mut raw_rows = executor
        .fetch(&sql, params)
        .await
        .with_query(|| QueryContext::new(&sql, &bind_params))?;
    let mut results = Vec::new();

    while let Some(row) = raw_rows
        .next()
        .await
        .map_err(drizzle_core::error::DrizzleError::from)
        .with_query(|| QueryContext::new(&sql, &bind_params))?
    {
        let base =
            <T as drizzle_core::query::QueryTable>::Select::try_from(&row).map_err(Into::into)?;

        let mut rel_col = num_base_cols;
        let mut next_rel = || {
            let json = row
                .get::<Option<String>>(rel_col)
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
    /// Executes the query and returns all matching rows with their relations.
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        relational_find_many(&self.runner.conn, self.builder).await
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
    /// Executes the query and returns the first matching row, or `None`.
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

/// Runs a `PartialColumns` relational query on `executor` and decodes every
/// row. Shared by the `&Drizzle` and `&Transaction` runner impls; base
/// columns are deserialized from a JSON `"__base"` column.
#[cfg(feature = "query")]
pub(crate) async fn relational_find_many_partial<'a, T, Rels, Cl>(
    executor: &impl prepared::TursoExecutor,
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
        T::TABLE_NAME,
        &col_refs,
        T::BLOB_COLUMNS,
        rendered,
        builder.where_sql,
        builder.order_by_sql,
        builder.limit,
        builder.offset,
        true,
    );
    let (sql, bind_params) = query_sql.build();
    drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

    let params: Vec<turso::Value> = bind_params
        .iter()
        .copied()
        .map(std::convert::Into::into)
        .collect();
    let mut raw_rows = executor
        .fetch(&sql, params)
        .await
        .with_query(|| QueryContext::new(&sql, &bind_params))?;
    let mut results = Vec::new();

    while let Some(row) = raw_rows
        .next()
        .await
        .map_err(drizzle_core::error::DrizzleError::from)
        .with_query(|| QueryContext::new(&sql, &bind_params))?
    {
        // Column 0 is the JSON "__base" object
        let base_json: String = row
            .get::<String>(0)
            .map_err(drizzle_core::error::DrizzleError::from)?;
        let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
            &base_json, "base",
        )?;

        let mut rel_col = 1usize;
        let mut next_rel = || {
            let json = row.get::<String>(rel_col).ok();
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
    /// Executes the query and returns all matching rows with their relations.
    ///
    /// Base columns are deserialized from a JSON `"__base"` column.
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
        relational_find_many_partial(&self.runner.conn, self.builder).await
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
    /// Executes the query and returns the first matching row, or `None`.
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

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Connection, T, Rels, drizzle_core::query::AllColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub async fn find_many<const N: usize>(
        &self,
        conn: &impl prepared::TursoExecutor,
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );

        let num_base_cols = T::COLUMN_NAMES.len();
        let (sql_str, params) = self.inner.bind(params)?;
        let mut driver_params = Vec::with_capacity(self.inner.params.len());
        driver_params.extend(params.map(Into::into));
        let mut raw_rows = conn.fetch(sql_str, driver_params).await?;
        let mut results = Vec::new();

        while let Some(row) = raw_rows.next().await? {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(&row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json = row
                    .get::<Option<String>>(rel_col)
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

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub async fn find_first<const N: usize>(
        &self,
        conn: &impl prepared::TursoExecutor,
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::turso::Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::turso::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(conn, params).await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Connection, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub async fn find_many<const N: usize>(
        &self,
        conn: &impl prepared::TursoExecutor,
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
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );

        let (sql_str, params) = self.inner.bind(params)?;
        let mut driver_params = Vec::with_capacity(self.inner.params.len());
        driver_params.extend(params.map(Into::into));
        let mut raw_rows = conn.fetch(sql_str, driver_params).await?;
        let mut results = Vec::new();

        while let Some(row) = raw_rows.next().await? {
            let base_json: String = row
                .get::<String>(0)
                .map_err(drizzle_core::error::DrizzleError::from)?;
            let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
                &base_json, "base",
            )?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let json = row
                    .get::<Option<String>>(rel_col)
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

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub async fn find_first<const N: usize>(
        &self,
        conn: &impl prepared::TursoExecutor,
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
        Ok(self.find_many(conn, params).await?.into_iter().next())
    }
}

impl<S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'_, S, QueryBuilder<'_, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(std::convert::Into::into)
            .collect();
        turso_execute_cached(&self.runner.conn, &sql_str, driver_params)
            .await
            .map_err(drizzle_core::error::DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::turso::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::turso::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(std::convert::Into::into)
            .collect();
        let mut rows = turso_query_cached(&self.runner.conn, &sql_str, driver_params)
            .await
            .map_err(drizzle_core::error::DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let mut decoded = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(drizzle_core::error::DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?
        {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::turso::Row,
                R,
            >>::decode(&row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a row cursor using the builder's row type.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r turso::Row>,
        for<'r> <Rw as TryFrom<&'r turso::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(std::convert::Into::into)
            .collect();

        let rows = turso_query_cached(&self.runner.conn, &sql_str, driver_params)
            .await
            .map_err(drizzle_core::error::DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        Ok(Rows::with_sql(rows, sql_str))
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::turso::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::turso::Row, Rw, R>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let driver_params: Vec<turso::Value> = params
            .iter()
            .copied()
            .map(std::convert::Into::into)
            .collect();
        let mut rows = turso_query_cached(&self.runner.conn, &sql_str, driver_params)
            .await
            .map_err(drizzle_core::error::DrizzleError::from)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        match turso_decode_first_and_finish(&mut rows, |row| {
            <Mk as drizzle_core::row::DecodeSelectedRef<&::turso::Row, R>>::decode(row)
        })
        .await
        {
            Err(DrizzleError::NotFound) => Err(DrizzleError::NotFound),
            result => result.with_query(|| QueryContext::new(&sql_str, &params)),
        }
    }
}
