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
//! drizzle = { version = "*", features = ["d1", "uuid"] }
//! worker = { version = "*", features = ["d1"] }
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
//! ```rust
//! # let _ = r####"
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
//!     let (db, AppSchema { user }) = Drizzle::new(d1, AppSchema::new());
//!
//!     db.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!     let users: Vec<SelectUser> = db.select(()).from(user).all().await?;
//!
//!     Response::ok(format!("{} users", users.len()))
//! }
//! # "####;
//! ```
//!
//! # Notes
//!
//! - **No transactions or savepoints.** D1 does not expose `BEGIN`/`COMMIT`.
//!   Use [`Drizzle::batch`] to submit multiple statements as a single atomic
//!   unit — D1 wraps a batch in an implicit transaction.
//! - **Row decoding is serde-based.** Rows come back as column-keyed objects,
//!   so `SelectX` models must implement `serde::Deserialize`. `SQLiteFromRow`
//!   derives this when the `serde` feature is enabled.

mod prepared;

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

pub type Drizzle<Schema = ()> = common::Drizzle<D1Database, Schema>;
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, D1Database, Schema, Builder, State>;

/// Convert a drizzle SQLite value into a `JsValue` suitable for D1 parameter
/// binding. D1 accepts null, number, BigInt, string, and Uint8Array.
fn sqlite_value_to_js(value: &SQLiteValue<'_>) -> JsValue {
    match value {
        SQLiteValue::Null => JsValue::NULL,
        SQLiteValue::Integer(i) => {
            // D1 accepts JS number for integers within Number.MAX_SAFE_INTEGER,
            // otherwise BigInt. We always coerce to f64 for compatibility — D1
            // rounds-trip large ints through BigInt where needed on its side.
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
    let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
    let stmt = conn.prepare(sql_str);
    bind_statement(stmt, &values)
}

impl<Schema> common::Drizzle<D1Database, Schema> {
    /// Executes a statement and returns the number of affected rows.
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

    /// Runs the query and returns all matching rows deserialized into `R`.
    ///
    /// D1 returns rows as JSON objects keyed by column name, so `R` must
    /// implement [`serde::Deserialize`]. The `SQLiteFromRow` macro emits a
    /// matching `Deserialize` impl when the `serde` feature is enabled.
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

    /// Runs the query and returns the first matching row.
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

    /// Submits multiple statements as a single D1 batch. D1 wraps the batch in
    /// an implicit transaction: if any statement fails, all preceding
    /// statements in the batch are rolled back.
    ///
    /// This is D1's equivalent of a transaction — Workers cannot issue
    /// `BEGIN`/`COMMIT` directly.
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
    /// Create schema objects in the D1 database.
    ///
    /// D1 does not expose `executeMultiple` to Workers, so statements are run
    /// through [`D1Database::batch`] for atomicity.
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let stmts: Vec<String> = schema.create_statements()?.collect();
        if stmts.is_empty() {
            return Ok(());
        }
        let prepared: Vec<D1PreparedStatement> =
            stmts.into_iter().map(|s| self.conn.prepare(s)).collect();
        self.conn
            .batch(prepared)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(())
    }
}

impl<Schema> common::Drizzle<D1Database, Schema> {
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// Creates the migrations table if needed and applies pending migrations
    /// as a single atomic batch.
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
    pub async fn migrate(
        &self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<()> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::SQLite,
            tracking,
        );

        ensure_d1_migration_table(&self.conn, &set).await?;

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
            return Ok(());
        }

        // Build all statements (DDL + tracking insert) into a single batch.
        let mut batch: Vec<D1PreparedStatement> = Vec::new();
        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    batch.push(self.conn.prepare(stmt));
                }
            }
            batch.push(self.conn.prepare(set.record_migration_sql(migration)));
        }

        let results = self
            .conn
            .batch(batch)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for r in &results {
            if !r.success() {
                return Err(DrizzleError::Other(
                    r.error()
                        .unwrap_or_else(|| "D1 migration batch failed".into())
                        .into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct AppliedName {
    name: String,
}

async fn ensure_d1_migration_table(
    conn: &D1Database,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    conn.prepare(set.create_table_sql())
        .run()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

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
        batch.push(conn.prepare(format!(
            "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
            set.table_ident_sql(),
            escaped_name,
            where_clause
        )));
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
    Ok(())
}

// =============================================================================
// Terminal methods on DrizzleBuilder (execute / all / get / rows)
// =============================================================================

#[cfg(feature = "d1")]
impl<'a, 'b, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows.
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = self.builder.sql.build();
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let stmt = self.drizzle.conn.prepare(sql_str);
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

    /// Runs the query and returns all matching rows deserialized into `R`.
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let stmt = self.drizzle.conn.prepare(sql_str);
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

    /// Runs the query and returns the first matching row.
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let values: Vec<JsValue> = params.into_iter().map(sqlite_value_to_js).collect();
        let stmt = self.drizzle.conn.prepare(sql_str);
        let stmt = bind_statement(stmt, &values)?;
        stmt.first::<R>(None)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .ok_or(DrizzleError::NotFound)
    }
}
