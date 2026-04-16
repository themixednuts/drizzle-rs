//! Cloudflare Durable Objects SQL storage driver (sync, WASM-only).
//!
//! Durable Objects expose a synchronous SQLite database at `state.storage().sql()`
//! (see [`worker::SqlStorage`]). Unlike [D1](super::d1), the Durable Object
//! SQL backend supports regular SQLite transactions via `BEGIN`/`COMMIT` and
//! savepoints via `SAVEPOINT`/`RELEASE`/`ROLLBACK TO`.
//!
//! # Requirements
//!
//! - `target_arch = "wasm32"` — bindings only link inside a Worker runtime.
//! - `worker = "0.8"` (no extra feature needed for Durable Objects SQL).
//!
//! Enable the `durable` feature on the `drizzle` crate when building your DO:
//!
//! ```toml
//! [dependencies]
//! drizzle = { version = "*", features = ["durable", "uuid"] }
//! worker = { version = "0.8" }
//! ```
//!
//! # Quick start
//!
//! ```rust
//! # let _ = r####"
//! use drizzle::sqlite::prelude::*;
//! use drizzle::sqlite::durable::Drizzle;
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
//! struct AppSchema {
//!     user: User,
//! }
//!
//! #[durable_object]
//! pub struct Counter { state: State, env: Env }
//!
//! impl DurableObject for Counter {
//!     fn new(state: State, env: Env) -> Self { Self { state, env } }
//!     async fn fetch(&mut self, _req: Request) -> worker::Result<Response> {
//!         let sql = self.state.storage().sql();
//!         let (db, AppSchema { user, .. }) = Drizzle::new(sql, AppSchema::new());
//!         db.create()?;
//!         db.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!         let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//!         Response::ok(format!("{} users", users.len()))
//!     }
//! }
//! # "####;
//! ```
//!
//! # Differences from other SQLite drivers
//!
//! - **Row decoding is serde-based.** The DO SQL API returns rows as JS
//!   objects keyed by column name, so `SelectX` models must implement
//!   `serde::Deserialize`. The `SQLiteFromRow` macro emits this when the
//!   `serde` feature is enabled.
//! - **Transactions use raw SQL.** `worker` 0.8 does not expose
//!   `transactionSync`, so this driver issues `BEGIN`/`COMMIT`/`ROLLBACK` via
//!   `SqlStorage::exec`. DO SQL is single-threaded inside the isolate, so
//!   concurrency is not a concern.

mod prepared;

use ::worker::{SqlStorage, SqlStorageValue};
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;

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

/// Convert a drizzle SQLite value into a `JsValue` suitable for DO SQL
/// parameter binding. Matches the D1 conversion: null, number, BigInt-as-f64,
/// string, or Uint8Array.
pub(crate) fn sqlite_value_to_js(value: &SQLiteValue<'_>) -> JsValue {
    match value {
        SQLiteValue::Null => JsValue::NULL,
        SQLiteValue::Integer(i) => JsValue::from(*i as f64),
        SQLiteValue::Real(r) => JsValue::from(*r),
        SQLiteValue::Text(s) => JsValue::from_str(s.as_ref()),
        SQLiteValue::Blob(b) => Uint8Array::from(b.as_ref()).into(),
    }
}

/// Convert a drizzle SQLite value into a typed [`SqlStorageValue`].
///
/// Used by the prepared-statement path. Text/Blob require copying because
/// `SqlStorageValue` is owned (`String` / `Vec<u8>`).
pub(crate) fn sqlite_value_to_storage(value: &SQLiteValue<'_>) -> SqlStorageValue {
    match value {
        SQLiteValue::Null => SqlStorageValue::Null,
        SQLiteValue::Integer(i) => SqlStorageValue::Integer(*i),
        SQLiteValue::Real(r) => SqlStorageValue::Float(*r),
        SQLiteValue::Text(s) => SqlStorageValue::String(s.as_ref().to_owned()),
        SQLiteValue::Blob(b) => SqlStorageValue::Blob(b.as_ref().to_vec()),
    }
}

/// Execute the given query via [`SqlStorage::exec_raw`] returning the raw
/// cursor. This is the core path shared by `all`/`get`/`execute`.
fn exec_query<'a, T>(
    conn: &SqlStorage,
    query: &T,
) -> drizzle_core::error::Result<::worker::SqlCursor>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    let sql = query.to_sql();
    let (sql_str, params) = sql.build();
    let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
    conn.exec_raw(&sql_str, values)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

impl<Schema> common::Drizzle<SqlStorage, Schema> {
    /// Executes a statement and returns the number of rows written.
    pub fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let cursor = exec_query(&self.conn, &query)?;
        // Drain the cursor so the statement fully executes; `rows_written`
        // is only populated once the cursor has been consumed.
        let _ = cursor
            .to_array::<serde_json::Value>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs the query and returns all matching rows deserialized into `R`.
    ///
    /// DO SQL returns rows as JS objects keyed by column name, so `R` must
    /// implement [`serde::Deserialize`]. The `SQLiteFromRow` macro emits a
    /// matching `Deserialize` impl when the `serde` feature is enabled.
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
    /// Returns the value produced by the callback on success. The
    /// transaction is committed when the callback returns `Ok` and rolled
    /// back on `Err`. The DO SQL backend is single-threaded within the
    /// isolate, so nesting via savepoints is safe.
    ///
    /// `worker` 0.8 does not yet expose `transactionSync`, so this method
    /// drives the transaction via raw `BEGIN`/`COMMIT`/`ROLLBACK`.
    pub fn transaction<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        self.conn
            .exec("BEGIN", None)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let result = f(self);
        match result {
            Ok(value) => {
                self.conn
                    .exec("COMMIT", None)
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                Ok(value)
            }
            Err(e) => {
                // Best effort rollback — propagate the original error.
                let _ = self.conn.exec("ROLLBACK", None);
                Err(e)
            }
        }
    }

    /// Executes a savepoint block. Nested savepoints use a counter-based
    /// name so they don't collide. Failure rolls back to the savepoint
    /// without aborting any enclosing transaction.
    pub fn savepoint<F, R>(&self, name: &str, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        let sp = format!("SAVEPOINT {}", quote_identifier(name));
        let rel = format!("RELEASE {}", quote_identifier(name));
        let roll = format!("ROLLBACK TO {}", quote_identifier(name));
        self.conn
            .exec(&sp, None)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let result = f(self);
        match result {
            Ok(value) => {
                self.conn
                    .exec(&rel, None)
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                Ok(value)
            }
            Err(e) => {
                let _ = self.conn.exec(&roll, None);
                let _ = self.conn.exec(&rel, None);
                Err(e)
            }
        }
    }
}

/// Quote a SQL identifier with double-quotes, doubling any embedded quotes.
fn quote_identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    out.push('"');
    for ch in name.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
    out
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

impl<Schema> common::Drizzle<SqlStorage, Schema> {
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// Creates the migrations table if needed and runs pending migrations
    /// inside a single transaction for atomicity.
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

        self.transaction(|db| {
            for migration in &pending {
                for stmt in migration.statements() {
                    if !stmt.trim().is_empty() {
                        db.conn
                            .exec(stmt, None)
                            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                    }
                }
                db.conn
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
        let (sql_str, params) = self.builder.sql.build();
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let cursor = self
            .drizzle
            .conn
            .exec_raw(&sql_str, values)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
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
        let (sql_str, params) = self.builder.sql.build();
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let cursor = self
            .drizzle
            .conn
            .exec_raw(&sql_str, values)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and returns the first matching row.
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let cursor = self
            .drizzle
            .conn
            .exec_raw(&sql_str, values)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }
}
