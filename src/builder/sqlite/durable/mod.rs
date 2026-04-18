//! Cloudflare Durable Objects SQL storage driver (sync, WASM-only).
//!
//! Each Durable Object has its own embedded SQLite database accessed through
//! [`worker::SqlStorage`]. Unlike [D1](super::d1), it supports full
//! transactions and savepoints.
//!
//! # Requirements
//!
//! - `target_arch = "wasm32"` — bindings only link inside a Worker runtime.
//! - The `worker` crate (no extra feature needed for DO SQL).
//!
//! Enable the `durable` feature on `drizzle` in your Worker crate:
//!
//! ```toml
//! [dependencies]
//! drizzle = { version = "*", features = ["durable", "uuid"] }
//! worker = "*"
//! ```
//!
//! # Quick start
//!
//! Migrate inside `DurableObject::new` so the schema is current before any
//! `fetch` / `alarm` / websocket event is dispatched. The constructor is
//! synchronous and runs to completion before the runtime delivers the first
//! request.
//!
//! ```rust
//! # let _ = r####"
//! use drizzle::sqlite::prelude::*;
//! use drizzle::sqlite::durable::Drizzle;
//! use drizzle_migrations::Tracking;
//! use worker::{durable_object, DurableObject, Env, Request, Response, State};
//!
//! #[SQLiteTable]
//! struct User {
//!     #[column(primary)]
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct AppSchema { user: User }
//!
//! static MIGRATIONS: &[drizzle_migrations::Migration] =
//!     drizzle::include_migrations!("./drizzle");
//!
//! #[durable_object]
//! pub struct Counter { state: State, env: Env }
//!
//! impl DurableObject for Counter {
//!     fn new(state: State, env: Env) -> Self {
//!         // Runs once per DO instantiation (cold start / after eviction).
//!         let sql = state.storage().sql();
//!         let (db, _) = Drizzle::new(sql, AppSchema::new());
//!         db.migrate(MIGRATIONS, Tracking::SQLITE)
//!             .expect("durable migrations failed");
//!         Self { state, env }
//!     }
//!
//!     async fn fetch(&self, _req: Request) -> worker::Result<Response> {
//!         let sql = self.state.storage().sql();
//!         let (db, AppSchema { user }) = Drizzle::new(sql, AppSchema::new());
//!         db.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!         let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//!         Response::ok(format!("{} users", users.len()))
//!     }
//! }
//! # "####;
//! ```
//!
//! # Notes
//!
//! - **Row decoding is serde-based.** Rows come back as column-keyed objects,
//!   so `SelectX` models must implement `serde::Deserialize`. `SQLiteFromRow`
//!   derives this when the `serde` feature is enabled.
//! - **Transactions and nested savepoints** are supported via
//!   [`Drizzle::transaction`] and [`Transaction::savepoint`].

mod prepared;

use ::worker::{SqlStorage, SqlStorageValue};
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{self, QueryBuilder},
    values::SQLiteValue,
};

crate::drizzle_prepare_impl!();

use crate::builder::sqlite::common;

pub type Drizzle<Schema = ()> = common::Drizzle<SqlStorage, Schema>;
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, SqlStorage, Schema, Builder, State>;

/// Convert a drizzle SQLite value into a typed [`SqlStorageValue`] for
/// parameter binding.
pub(crate) fn sqlite_value_to_storage(value: &SQLiteValue<'_>) -> SqlStorageValue {
    match value {
        SQLiteValue::Null => SqlStorageValue::Null,
        SQLiteValue::Integer(i) => SqlStorageValue::Integer(*i),
        SQLiteValue::Real(r) => SqlStorageValue::Float(*r),
        SQLiteValue::Text(s) => SqlStorageValue::String(s.as_ref().to_owned()),
        SQLiteValue::Blob(b) => SqlStorageValue::Blob(b.as_ref().to_vec()),
    }
}

fn exec_query<'a, T>(
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

impl<Schema> common::Drizzle<SqlStorage, Schema> {
    /// Executes a statement and returns the number of rows written.
    pub fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let cursor = exec_query(&self.conn, &query)?;
        // Drain the cursor so `rows_written` is populated.
        let _ = cursor
            .to_array::<serde_json::Value>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs the query and returns all matching rows deserialized into `R`.
    ///
    /// `R` must implement [`serde::Deserialize`]. The `SQLiteFromRow` macro
    /// derives a matching impl when the `serde` feature is enabled.
    pub fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: Default + Extend<R>,
    {
        let cursor = exec_query(&self.conn, &query)?;
        let rows: Vec<R> = cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let mut out = C::default();
        out.extend(rows);
        Ok(out)
    }

    /// Runs the query and returns the first matching row.
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let cursor = exec_query(&self.conn, &query)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }

    /// Executes a transaction with the given callback.
    ///
    /// Commits when the callback returns `Ok` and rolls back on `Err` or a
    /// panic, then returns the callback's value (or propagates the error or
    /// panic).
    ///
    /// The callback receives a `&Transaction<Schema>` that supports the same
    /// query-builder surface as `Drizzle` (select / insert / update / delete /
    /// with) plus [`Transaction::savepoint`] for nested savepoints.
    pub fn transaction<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: FnOnce(
            &crate::transaction::sqlite::durable::Transaction<Schema>,
        ) -> drizzle_core::error::Result<R>,
    {
        self.conn
            .exec("BEGIN", None)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let tx =
            crate::transaction::sqlite::durable::Transaction::new(self.conn.clone(), self.schema);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&tx)));

        match result {
            Ok(Ok(value)) => {
                self.conn
                    .exec("COMMIT", None)
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                Ok(value)
            }
            Ok(Err(e)) => {
                // Best effort rollback — propagate the original error.
                let _ = self.conn.exec("ROLLBACK", None);
                Err(e)
            }
            Err(panic_payload) => {
                let _ = self.conn.exec("ROLLBACK", None);
                std::panic::resume_unwind(panic_payload);
            }
        }
    }
}

impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects in the DO's SQL storage.
    pub fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        for stmt in schema.create_statements()? {
            self.conn
                .exec(&stmt, None)
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        }
        Ok(())
    }
}

impl<Schema> common::Drizzle<SqlStorage, Schema>
where
    Schema: Copy,
{
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// Creates the migrations table if needed and runs pending migrations
    /// inside a single transaction for atomicity.
    ///
    /// # Call this from `DurableObject::new`, not `fetch`
    ///
    /// Each Durable Object has its own per-instance database, so migrations
    /// must run at runtime. The right place is the constructor:
    ///
    /// ```rust
    /// # let _ = r####"
    /// impl DurableObject for Counter {
    ///     fn new(state: State, env: Env) -> Self {
    ///         let sql = state.storage().sql();
    ///         let (db, _) = Drizzle::new(sql, AppSchema::new());
    ///         db.migrate(MIGRATIONS, Tracking::SQLITE)
    ///             .expect("durable migrations failed");
    ///         Self { state, env }
    ///     }
    ///
    ///     async fn fetch(&self, req: Request) -> Result<Response> {
    ///         // hot path — no migration work
    ///     }
    /// }
    /// # "####;
    /// ```
    ///
    /// This runs once per instantiation (cold start or after eviction). The
    /// runtime does not deliver events to an instance whose `new` has not
    /// returned, so no request can observe a half-migrated database.
    ///
    /// Calling `migrate` from `fetch` instead pays a tracking-table
    /// round-trip on every request and is almost always wrong.
    pub fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<()> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::SQLite,
            tracking,
        );

        ensure_durable_migration_table(&self.conn, &set)?;

        // Read already-applied migration names
        let applied_sql = set.applied_names_sql();
        let applied_cursor = self
            .conn
            .exec(&applied_sql, None)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let applied_names: Vec<String> = applied_cursor
            .to_array::<AppliedName>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .map(|r| r.name)
            .collect();

        let pending: Vec<_> = set.pending(&applied_names).collect();
        if pending.is_empty() {
            return Ok(());
        }

        self.transaction(|tx| {
            for migration in &pending {
                for stmt in migration.statements() {
                    if !stmt.trim().is_empty() {
                        tx.inner()
                            .exec(stmt, None)
                            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                    }
                }
                tx.inner()
                    .exec(&set.record_migration_sql(migration), None)
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
            }
            Ok(())
        })
    }
}

#[derive(serde::Deserialize)]
struct AppliedName {
    name: String,
}

fn ensure_durable_migration_table(
    conn: &SqlStorage,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    conn.exec(&set.create_table_sql(), None)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

    // Detect legacy (hash, created_at)-only table and upgrade it in-place.
    let table_name = set.table_name().replace('\'', "''");
    let pragma_sql = format!("SELECT name FROM pragma_table_info('{}')", table_name);
    let cols_cursor = conn
        .exec(&pragma_sql, None)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

    #[derive(serde::Deserialize)]
    struct ColName {
        name: String,
    }
    let col_rows: Vec<ColName> = cols_cursor
        .to_array()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    if col_rows.iter().any(|c| c.name == "name") {
        return Ok(());
    }

    // Legacy upgrade: ALTER TABLE ADD COLUMN + backfill via match_applied_migration_metadata.
    #[derive(serde::Deserialize)]
    struct LegacyRow {
        id: Option<i64>,
        hash: String,
        created_at: i64,
    }
    let legacy_cursor = conn
        .exec(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            None,
        )
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    let legacy_rows: Vec<LegacyRow> = legacy_cursor
        .to_array()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    let applied: Vec<drizzle_migrations::AppliedMigrationMetadata> = legacy_rows
        .into_iter()
        .map(|r| drizzle_migrations::AppliedMigrationMetadata {
            id: r.id,
            hash: r.hash,
            created_at: r.created_at,
        })
        .collect();

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

    conn.exec(
        &format!(
            "ALTER TABLE {} ADD COLUMN \"name\" text",
            set.table_ident_sql()
        ),
        None,
    )
    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    conn.exec(
        &format!(
            "ALTER TABLE {} ADD COLUMN \"applied_at\" TEXT",
            set.table_ident_sql()
        ),
        None,
    )
    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    for row in matched {
        let escaped_name = row.name.replace('\'', "''");
        let where_clause = if let Some(id) = row.id {
            format!("\"id\" = {id}")
        } else {
            format!(
                "\"created_at\" = {} AND \"hash\" = '{}'",
                row.created_at,
                row.hash.replace('\'', "''")
            )
        };
        conn.exec(
            &format!(
                "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
                set.table_ident_sql(),
                escaped_name,
                where_clause
            ),
            None,
        )
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    }
    Ok(())
}

// =============================================================================
// Terminal methods on DrizzleBuilder (execute / all / get)
// =============================================================================

#[cfg(feature = "durable")]
impl<'a, 'b, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of rows written.
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        let cursor = exec_query(&self.drizzle.conn, &self.builder.sql)?;
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
        let cursor = exec_query(&self.drizzle.conn, &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and returns the first matching row.
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_query(&self.drizzle.conn, &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }
}
