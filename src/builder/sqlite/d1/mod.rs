//! Cloudflare D1 driver (async, WASM-only).
//!
//! D1 is Cloudflare's serverless SQL database built on SQLite.
//!
//! # Requirements
//!
//! - `target_arch = "wasm32"` — D1 bindings only link inside a Worker runtime.
//! - The `worker` crate with its `d1` feature.
//!
//! Enable the `d1` feature on `drizzle` in your Worker crate:
//!
//! ```toml
//! [dependencies]
//! drizzle = { version = "0.2", features = ["d1", "uuid"] }
//! worker = { version = "0.8", features = ["d1"] }
//! ```
//!
//! # Migrations
//!
//! Run migrations at deploy time with Wrangler, not from the Worker:
//!
//! ```bash
//! wrangler d1 migrations apply <DB_NAME>
//! ```
//!
//! pointed at the migrations directory drizzle-rs generated. The Worker
//! assumes the schema is current and skips runtime migration entirely.
//!
//! [`Drizzle::migrate`] exists for the rare case where the Worker itself
//! provisions a new D1 (e.g. tenant-per-database); see that method's docs.
//!
//! # Quick start
//!
//! ```ignore
//! use drizzle::sqlite::prelude::*;
//! use drizzle::sqlite::d1::Drizzle;
//! use worker::{event, Context, Env, Request, Response};
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
//! #[event(fetch)]
//! async fn fetch(_req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
//!     // Schema is assumed current — applied out-of-band via wrangler.
//!     let d1 = env.d1("DB")?;
//!     let (db, AppSchema { user }) = Drizzle::new(d1);
//!
//!     // `worker::Error` has no `From<drizzle::error::DrizzleError>`, so
//!     // convert drizzle errors before using `?`.
//!     db.insert(user)
//!         .values([InsertUser::new("Alice")])
//!         .execute()
//!         .await
//!         .map_err(|e| worker::Error::RustError(e.to_string()))?;
//!     let users: Vec<SelectUser> = db
//!         .select(())
//!         .from(user)
//!         .all()
//!         .await
//!         .map_err(|e| worker::Error::RustError(e.to_string()))?;
//!
//!     Response::ok(format!("{} users", users.len()))
//! }
//! ```
//!
//! # Notes
//!
//! - **No transactions or savepoints.** D1 does not expose `BEGIN`/`COMMIT`.
//!   Use [`Drizzle::batch`] to submit multiple statements as a single atomic
//!   unit — D1 wraps a batch in an implicit transaction.
//! - **Row decoding is serde-based.** Rows come back as column-keyed objects,
//!   so a row type must implement `serde::Deserialize`. Generated `SelectX`
//!   and `PartialSelectX` models do when the `query` feature is enabled;
//!   derive it on your own row structs.
//! - **Integers are limited to ±2^53.** D1 parameters and results travel as
//!   JS numbers, so `i64` values outside `Number.MAX_SAFE_INTEGER` lose
//!   precision. Store larger identifiers as TEXT or BLOB.
//!
//! # Statement caching
//!
//! This driver does not keep a statement cache, and there is nothing useful to
//! cache. `D1Database::prepare` does not parse anything: it builds a JS
//! statement object inside the Worker, and the SQL text still crosses the HTTP
//! boundary to D1 on every `run`/`all`/`first`. The parse happens server side,
//! per request, and D1 hands back no handle that would let a later call skip
//! it.
//!
//! Caching `D1PreparedStatement` values would therefore save a JS object
//! allocation and nothing else, while adding a per-connection map to a type
//! that is not `Send`/`Sync`. Batch multiple statements with
//! [`Drizzle::batch`] instead — that removes whole round trips, which is where
//! the cost actually is.

pub(crate) mod prepared;

use ::worker::{D1Database, D1PreparedStatement};
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
#[cfg(feature = "query")]
use crate::builder::sqlite::common::QueryRowFormat;

/// The D1 database handle: a Cloudflare D1 binding
/// ([`worker::D1Database`]) plus the schema's table handles.
///
/// Create it with `Drizzle::new(env.d1("DB")?)`, then build queries with
/// `select`, `insert`, `update`, and `delete`.
pub type Drizzle<Schema = ()> = common::Drizzle<D1Database, Schema>;
/// A query attached to a [`Drizzle`] handle, ready to run with `.execute()`,
/// `.all()`, `.get()`, or `.rows()`.
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, common::Drizzle<D1Database, Schema>, Schema, Builder, State>;

#[cfg(feature = "query")]
impl common::private::Sealed for D1Database {}

// Column-keyed serde rows: relational queries wrap base columns into a single
// "__base" JSON text column. See `common::QueryRowFormat`.
#[cfg(feature = "query")]
impl QueryRowFormat for D1Database {
    const WRAP_BASE_JSON: bool = true;
}

/// Convert a drizzle SQLite value into a `JsValue` suitable for D1 parameter
/// binding. D1 accepts null, number, BigInt, string, and Uint8Array.
fn sqlite_value_to_js(value: &SQLiteValue<'_>) -> JsValue {
    match value {
        SQLiteValue::Null => JsValue::NULL,
        SQLiteValue::Integer(i) => {
            // D1 only accepts JS numbers as integer bind parameters (BigInt is
            // rejected), so values outside ±2^53 lose precision here. The
            // Durable Objects driver behaves the same way — the `worker` crate
            // performs an identical f64 coercion for `SqlStorageValue::Integer`.
            // Store larger identifiers as TEXT or BLOB.
            JsValue::from(*i as f64)
        }
        SQLiteValue::Real(r) => JsValue::from(*r),
        SQLiteValue::Text(s) => JsValue::from_str(s.as_ref()),
        SQLiteValue::Blob(b) => Uint8Array::from(b.as_ref()).into(),
    }
}

pub(crate) fn bind_statement(
    stmt: D1PreparedStatement,
    values: &[JsValue],
) -> drizzle_core::error::Result<D1PreparedStatement> {
    stmt.bind(values)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

fn prepare_and_bind<'a, T>(
    conn: &D1Database,
    query: &T,
) -> drizzle_core::error::Result<D1PreparedStatement>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    let sql = query.to_sql();
    let (sql_str, params) = sql.build();
    drizzle_core::drizzle_trace_query!(&sql_str, params.len());
    let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
    let stmt = conn.prepare(sql_str);
    bind_statement(stmt, &values)
}

impl<Schema> common::Drizzle<D1Database, Schema> {
    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, and
    /// returns the number of rows it changed (D1's `meta.changes`).
    ///
    /// Prefer the builder's own `.execute()`, which keeps the builder's
    /// compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] with D1's message when binding fails or
    /// D1 rejects the statement.
    pub async fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let stmt = prepare_and_bind(&self.conn, &query)?;
        let result = stmt
            .run()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        if !result.success() {
            return Err(DrizzleError::Other(
                result
                    .error()
                    .unwrap_or_else(|| "D1 statement failed".into())
                    .into(),
            ));
        }

        let changes = result
            .meta()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .and_then(|m| m.changes)
            .unwrap_or(0);
        Ok(changes as u64)
    }

    /// Runs any SQL value and collects its rows into `C` (for example
    /// `Vec<R>`).
    ///
    /// D1 returns rows as objects keyed by column name, so `R` must implement
    /// [`serde::Deserialize`] with field names matching the column names.
    /// Generated `Select*` models implement it when the `query` feature is
    /// on; derive it on your own row types. This skips the builder's
    /// compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] when binding fails, D1 rejects the
    /// query, or a row cannot be deserialized into `R`.
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: Default + Extend<R>,
    {
        let stmt = prepare_and_bind(&self.conn, &query)?;
        let result = stmt
            .all()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        if !result.success() {
            return Err(DrizzleError::Other(
                result
                    .error()
                    .unwrap_or_else(|| "D1 query failed".into())
                    .into(),
            ));
        }

        let rows: Vec<R> = result
            .results::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut out = C::default();
        out.extend(rows);
        Ok(out)
    }

    /// Runs any SQL value and deserializes its first row into `R`.
    ///
    /// `R` has the same requirements as in [`all`](Self::all).
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and
    /// [`DrizzleError::Other`] when binding fails, D1 rejects the query, or
    /// the row cannot be deserialized into `R`.
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let stmt = prepare_and_bind(&self.conn, &query)?;
        let row = stmt
            .first::<R>(None)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        row.ok_or(DrizzleError::NotFound)
    }

    /// Runs several statements as one D1 batch, which D1 applies atomically:
    /// when one statement fails, the whole batch is rolled back.
    ///
    /// This is D1's replacement for a transaction, since Workers cannot issue
    /// `BEGIN`/`COMMIT`. Row results are discarded.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] when binding a statement fails or D1
    /// rejects the batch.
    pub async fn batch<'a, I, T>(&'a self, statements: I) -> drizzle_core::error::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let prepared: Vec<D1PreparedStatement> = statements
            .into_iter()
            .map(|q| prepare_and_bind(&self.conn, &q))
            .collect::<drizzle_core::error::Result<_>>()?;

        let results = self
            .conn
            .batch(prepared)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        for r in &results {
            if !r.success() {
                return Err(DrizzleError::Other(
                    r.error().unwrap_or_else(|| "D1 batch failed".into()).into(),
                ));
            }
        }
        Ok(())
    }
}

impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Creates every table, index, and view in the schema.
    ///
    /// The `CREATE` statements run as one atomic [`D1Database::batch`].
    ///
    /// # Errors
    ///
    /// Returns an error when the schema's statements cannot be generated or
    /// D1 rejects the batch.
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let stmts: Vec<String> = schema.create_statements()?.collect();
        if stmts.is_empty() {
            return Ok(());
        }
        let prepared: Vec<D1PreparedStatement> =
            stmts.into_iter().map(|s| self.conn.prepare(s)).collect();
        let results = self
            .conn
            .batch(prepared)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for r in &results {
            if !r.success() {
                return Err(DrizzleError::Other(
                    r.error()
                        .unwrap_or_else(|| "D1 create batch failed".into())
                        .into(),
                ));
            }
        }
        Ok(())
    }
}

impl<Schema> common::Drizzle<D1Database, Schema> {
    /// Applies the migrations that have not run yet.
    ///
    /// Creates the tracking table if needed, then runs every pending
    /// migration's statements and its tracking row as one atomic D1 batch.
    ///
    /// # Prefer deploy-time migration
    ///
    /// For D1, running migrations at runtime is usually the wrong choice —
    /// every cold start pays a round-trip to check the tracking table, and
    /// concurrent cold starts on a fresh database can race. Apply migrations
    /// from your deploy pipeline instead:
    ///
    /// ```bash
    /// wrangler d1 migrations apply <DB_NAME>
    /// ```
    ///
    /// Reach for this method only when the Worker itself provisions new
    /// databases (e.g. tenant-per-database setups). Gate it so it runs at
    /// most once per database rather than on every request.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::UnsupportedMigrationExecution`] when a pending
    /// migration suspends foreign keys (`PRAGMA foreign_keys=OFF`, as table
    /// rebuilds do), which D1 cannot run, and an error when the tracking table
    /// holds an unfinished ("dirty") row or the batch fails.
    pub async fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::SQLite,
            tracking,
        );

        let applied_before_table_write =
            d1_applied_names_before_migration_table_write(&self.conn, &set).await?;
        super::reject_foreign_key_suspending_migrations(
            set.pending(&applied_before_table_write),
            "D1",
        )?;
        ensure_d1_migration_table(&self.conn, &set).await?;

        // A D1 batch is atomic, so this path never writes a dirty marker
        // itself — but the same database may have been migrated by a
        // non-transactional runner (the CLI's HTTP path, another driver), so
        // refuse to stack migrations on top of an unfinished one.
        let dirty = self
            .conn
            .prepare(set.dirty_names_sql())
            .all()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let dirty_names: Vec<String> = dirty
            .results::<AppliedName>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .map(|r| r.name)
            .collect();
        if let Some(error) = set.interrupted_migration_error(&dirty_names) {
            return Err(DrizzleError::Other(error.to_string().into()));
        }

        // Read already-applied migration names
        let applied = self
            .conn
            .prepare(set.applied_names_sql())
            .all()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let applied_names: Vec<String> = applied
            .results::<AppliedName>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .map(|r| r.name)
            .collect();

        let pending: Vec<_> = set.pending(&applied_names).collect();
        if pending.is_empty() {
            return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
        }
        super::reject_foreign_key_suspending_migrations(pending.iter().copied(), "D1")?;

        // Build all statements (DDL + tracking insert) into a single batch.
        let mut batch: Vec<D1PreparedStatement> = Vec::new();
        let mut applied_tags = Vec::with_capacity(pending.len());
        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    batch.push(self.conn.prepare(stmt));
                }
            }
            batch.push(self.conn.prepare(set.record_migration_sql(migration)));
            applied_tags.push(migration.tag().to_string());
        }

        let results = match self.conn.batch(batch).await {
            Ok(results) => results,
            Err(error) => {
                if d1_migrations_are_applied(&self.conn, &set, &pending).await? {
                    return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
                }
                return Err(DrizzleError::Other(error.to_string().into()));
            }
        };
        for r in &results {
            if !r.success() {
                if d1_migrations_are_applied(&self.conn, &set, &pending).await? {
                    return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
                }
                return Err(DrizzleError::Other(
                    r.error()
                        .unwrap_or_else(|| "D1 migration batch failed".into())
                        .into(),
                ));
            }
        }
        Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied_tags })
    }
}

#[derive(serde::Deserialize)]
struct AppliedName {
    name: String,
}

async fn d1_applied_names_before_migration_table_write(
    conn: &D1Database,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<Vec<String>> {
    let table_name = set.table_name().replace('\'', "''");
    let columns = conn
        .prepare(format!(
            "SELECT name FROM pragma_table_info('{}')",
            table_name
        ))
        .all()
        .await
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;

    #[derive(serde::Deserialize)]
    struct ColumnName {
        name: String,
    }

    let columns: Vec<ColumnName> = columns
        .results()
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
    if columns.is_empty() {
        return Ok(Vec::new());
    }
    if columns.iter().any(|column| column.name == "name") {
        let applied = conn
            .prepare(set.applied_names_sql())
            .all()
            .await
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
        return applied
            .results::<AppliedName>()
            .map(|rows| rows.into_iter().map(|row| row.name).collect())
            .map_err(|error| DrizzleError::Other(error.to_string().into()));
    }

    #[derive(serde::Deserialize)]
    struct LegacyRow {
        id: Option<i64>,
        hash: String,
        created_at: i64,
    }

    let legacy = conn
        .prepare(format!(
            "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
            set.table_ident_sql()
        ))
        .all()
        .await
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
    let applied = legacy
        .results::<LegacyRow>()
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?
        .into_iter()
        .map(|row| drizzle_migrations::AppliedMigrationMetadata {
            id: row.id,
            hash: row.hash,
            created_at: row.created_at,
        })
        .collect::<Vec<_>>();
    drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map(|rows| rows.into_iter().map(|row| row.name).collect())
        .map_err(|error| DrizzleError::Other(error.to_string().into()))
}

async fn d1_migrations_are_applied(
    conn: &D1Database,
    set: &drizzle_migrations::Migrations,
    migrations: &[&drizzle_migrations::Migration],
) -> drizzle_core::error::Result<bool> {
    let applied = conn
        .prepare(set.applied_names_sql())
        .all()
        .await
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
    let names = applied
        .results::<AppliedName>()
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?
        .into_iter()
        .map(|record| record.name)
        .collect::<std::collections::HashSet<_>>();
    Ok(migrations
        .iter()
        .all(|migration| names.contains(migration.name())))
}

async fn ensure_d1_migration_table(
    conn: &D1Database,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    let created = conn
        .prepare(set.create_table_sql())
        .run()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    if !created.success() {
        return Err(DrizzleError::Other(
            created
                .error()
                .unwrap_or_else(|| "D1 migration-table creation failed".into())
                .into(),
        ));
    }

    // Check whether the `name` column already exists — if yes, nothing else to
    // do. Otherwise we need to upgrade the legacy (hash, created_at)-only
    // tracking table.
    let table_name = set.table_name().replace('\'', "''");
    let pragma_sql = format!("SELECT name FROM pragma_table_info('{}')", table_name);
    let cols = conn
        .prepare(pragma_sql)
        .all()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

    #[derive(serde::Deserialize)]
    struct ColName {
        name: String,
    }
    let col_rows: Vec<ColName> = cols
        .results()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    if col_rows.iter().any(|c| c.name == "name") {
        ensure_d1_migration_name_index(conn, set).await?;
        return Ok(());
    }

    // Legacy upgrade: load existing rows, match them to local migrations, then
    // ALTER TABLE + backfill, all in a single batch for atomicity.
    #[derive(serde::Deserialize)]
    struct LegacyRow {
        id: Option<i64>,
        hash: String,
        created_at: i64,
    }
    let legacy = conn
        .prepare(format!(
            "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
            set.table_ident_sql()
        ))
        .all()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    let legacy_rows: Vec<LegacyRow> = legacy
        .results()
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

    let mut batch: Vec<D1PreparedStatement> = Vec::new();
    batch.push(conn.prepare(format!(
        "ALTER TABLE {} ADD COLUMN \"name\" text",
        set.table_ident_sql()
    )));
    batch.push(conn.prepare(format!(
        "ALTER TABLE {} ADD COLUMN \"applied_at\" TEXT",
        set.table_ident_sql()
    )));
    for row in matched {
        batch.push(conn.prepare(set.backfill_migration_metadata_sql(&row)));
    }

    let results = conn
        .batch(batch)
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    for r in &results {
        if !r.success() {
            return Err(DrizzleError::Other(
                r.error()
                    .unwrap_or_else(|| "D1 migration upgrade batch failed".into())
                    .into(),
            ));
        }
    }
    ensure_d1_migration_name_index(conn, set).await?;
    Ok(())
}

async fn ensure_d1_migration_name_index(
    conn: &D1Database,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    if let Some(sql) = set.create_name_unique_index_sql() {
        let result = conn
            .prepare(sql)
            .run()
            .await
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
        if !result.success() {
            return Err(DrizzleError::Other(
                result
                    .error()
                    .unwrap_or_else(|| "D1 migration-name index creation failed".into())
                    .into(),
            ));
        }
    }
    Ok(())
}

// =============================================================================
// Terminal methods on DrizzleBuilder (execute / all / get)
// =============================================================================

#[cfg(feature = "d1")]
impl<'a, 'b, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the statement and returns the number of rows it changed (D1's
    /// `meta.changes`).
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] with D1's message when binding fails or
    /// D1 rejects the statement.
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let stmt = self.runner.conn.prepare(sql_str);
        let stmt = bind_statement(stmt, &values)?;
        let result = stmt
            .run()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        if !result.success() {
            return Err(DrizzleError::Other(
                result
                    .error()
                    .unwrap_or_else(|| "D1 statement failed".into())
                    .into(),
            ));
        }
        let changes = result
            .meta()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .and_then(|m| m.changes)
            .unwrap_or(0);
        Ok(changes as u64)
    }

    /// Runs the query and deserializes every row into `R`.
    ///
    /// D1 returns rows as objects keyed by column name, so `R` must implement
    /// [`serde::Deserialize`] with field names matching the selected column
    /// names. The scope and grouping checks apply as on the native drivers,
    /// but `R` itself is not checked against the selection at compile time:
    /// a mismatch, including a non-`Option` field for a `NULL` value, is a
    /// runtime deserialization error.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::Other`] when binding fails, D1 rejects the
    /// query, or a row cannot be deserialized into `R`.
    pub async fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
        R: for<'de> serde::Deserialize<'de>,
    {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let stmt = self.runner.conn.prepare(sql_str);
        let stmt = bind_statement(stmt, &values)?;
        let result = stmt
            .all()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        if !result.success() {
            return Err(DrizzleError::Other(
                result
                    .error()
                    .unwrap_or_else(|| "D1 query failed".into())
                    .into(),
            ));
        }
        result
            .results::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and deserializes its first row into `R`.
    ///
    /// `R` has the same requirements as in [`all`](Self::all).
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::NotFound`] when no row matches, and
    /// [`DrizzleError::Other`] when binding fails, D1 rejects the query, or
    /// the row cannot be deserialized into `R`.
    pub async fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
        R: for<'de> serde::Deserialize<'de>,
    {
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let stmt = self.runner.conn.prepare(sql_str);
        let stmt = bind_statement(stmt, &values)?;
        stmt.first::<R>(None)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .ok_or(DrizzleError::NotFound)
    }
}

// =============================================================================
// Query API: find_many / find_first
// =============================================================================
//
// D1 returns rows as column-keyed JSON objects rather than positional
// columns, so the relational query is always built with `WRAP_BASE_JSON`:
// the base row arrives as a single JSON `"__base"` column (BLOBs hex-encoded
// by SQL) and each relation as a JSON `"__rel_<name>"` column, decoded via
// [`drizzle_core::query::JsonQueryRow`].

#[cfg(feature = "query")]
async fn query_json_rows(
    conn: &D1Database,
    sql: &str,
    values: &[JsValue],
) -> drizzle_core::error::Result<Vec<drizzle_core::query::JsonQueryRow>> {
    let stmt = bind_statement(conn.prepare(sql), values)?;
    let result = stmt
        .all()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    if !result.success() {
        return Err(DrizzleError::Other(
            result
                .error()
                .unwrap_or_else(|| "D1 query failed".into())
                .into(),
        ));
    }
    result
        .results::<drizzle_core::query::JsonQueryRow>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

// AllColumns: base decoded from the JSON "__base" column
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let builder = self.builder;
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
            D1Database::WRAP_BASE_JSON,
        );
        let (sql, bind_params) = query_sql.build();
        let values: Vec<JsValue> = bind_params.into_iter().map(sqlite_value_to_js).collect();

        let rows = query_json_rows(&self.runner.conn, &sql, &values).await?;
        rows.into_iter()
            .map(|row| row.into_row::<_, Rels>())
            .collect()
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

// PartialColumns: base decoded from the JSON "__base" column of selected columns
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
        let builder = self.builder;
        let col_refs: Vec<&str> = builder.cols.columns.clone();
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
        let values: Vec<JsValue> = bind_params.into_iter().map(sqlite_value_to_js).collect();

        let rows = query_json_rows(&self.runner.conn, &sql, &values).await?;
        rows.into_iter()
            .map(|row| row.into_row::<_, Rels>())
            .collect()
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
    common::DrizzlePreparedQuery<'a, D1Database, T, Rels, drizzle_core::query::AllColumns>
{
    /// Runs the prepared relational query with `params` bound and returns
    /// every root row with its loaded relations.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub async fn find_many<const N: usize>(
        &self,
        conn: &D1Database,
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
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

        let (sql, bound) = self.inner.bind(params)?;
        let values: Vec<JsValue> = bound.map(|v| sqlite_value_to_js(&v)).collect();
        let rows = query_json_rows(conn, sql, &values).await?;
        rows.into_iter()
            .map(|row| row.into_row::<_, Rels>())
            .collect()
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
    pub async fn find_first<const N: usize>(
        &self,
        conn: &D1Database,
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(conn, params).await?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, D1Database, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Runs the prepared relational query with `params` bound and returns
    /// every root row with its loaded relations.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub async fn find_many<const N: usize>(
        &self,
        conn: &D1Database,
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

        let (sql, bound) = self.inner.bind(params)?;
        let values: Vec<JsValue> = bound.map(|v| sqlite_value_to_js(&v)).collect();
        let rows = query_json_rows(conn, sql, &values).await?;
        rows.into_iter()
            .map(|row| row.into_row::<_, Rels>())
            .collect()
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
    pub async fn find_first<const N: usize>(
        &self,
        conn: &D1Database,
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
