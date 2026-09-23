//! Cloudflare Durable Objects SQL storage driver (sync, WASM-only).
//!
//! Each Durable Object has its own embedded SQLite database. The driver runs
//! on a [`DurableStorage`], built once from the object's `State`. Unlike
//! [D1](super::d1), it supports transactions and nested savepoints.
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
//! Build the driver and migrate inside `DurableObject::new`, so the schema is
//! current before any `fetch` / `alarm` / websocket event is dispatched, and
//! keep the driver on the object. The constructor is synchronous and runs to
//! completion before the runtime delivers the first request.
//! [`examples/durable_object`] is this example as a compiled crate.
//!
//! ```rust
//! # let _ = r####"
//! use drizzle::migrations::Tracking;
//! use drizzle::sqlite::durable::{Drizzle, DurableStorage};
//! use drizzle::sqlite::prelude::*;
//! use worker::{DurableObject, Env, Request, Response, State, durable_object};
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
//! pub struct Counter {
//!     db: Drizzle<AppSchema>,
//! }
//!
//! impl DurableObject for Counter {
//!     fn new(mut state: State, _env: Env) -> Self {
//!         // Runs once per instantiation (cold start or after eviction).
//!         // `include_migrations!` embeds the migration files at compile time.
//!         let migrations = drizzle::include_migrations!("./drizzle");
//!         let (db, _) = Drizzle::new(DurableStorage::new(&mut state));
//!         db.migrate(&migrations, Tracking::SQLITE)
//!             .expect("durable migrations failed");
//!         Self { db }
//!     }
//!
//!     async fn fetch(&self, _req: Request) -> worker::Result<Response> {
//!         let AppSchema { user } = *self.db.schema();
//!         // `worker::Error` has no `From<drizzle::error::DrizzleError>`, so
//!         // convert drizzle errors before using `?`.
//!         let users: Vec<SelectUser> = self
//!             .db
//!             .transaction(|tx| {
//!                 tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!                 tx.select(()).from(user).all()
//!             })
//!             .map_err(|e| worker::Error::RustError(e.to_string()))?;
//!         Response::ok(format!("{} users", users.len()))
//!     }
//! }
//! # "####;
//! ```
//!
//! [`examples/durable_object`]: https://github.com/themixednuts/drizzle-rs/tree/main/examples/durable_object
//!
//! # Notes
//!
//! - **Row decoding is serde-based.** Rows come back as column-keyed objects,
//!   so a row type must implement `serde::Deserialize`. Generated `SelectX`
//!   and `PartialSelectX` models do when the `query` feature is enabled;
//!   derive it on your own row structs.
//! - **Transactions run through the runtime.** Durable Object SQL rejects
//!   `BEGIN`, `COMMIT` and `SAVEPOINT` statements, so [`Drizzle::transaction`]
//!   and [`Transaction::savepoint`] use the storage's `transactionSync`, which
//!   nests savepoints. Their callbacks must stay synchronous, as the rest of
//!   this driver is.
//! - **Writes are atomic per event anyway.** The runtime commits the writes an
//!   event makes without an intervening `await` together, so a transaction is
//!   for rolling back on an error, not for isolation.
//!
//! # Statement caching
//!
//! This driver does not keep a statement cache because the platform exposes no
//! statement to cache. A Durable Object's `SqlStorage` surface is
//! `exec(query, bindings)` and `exec_raw` — there is no prepare step and no
//! handle that survives a call, so drizzle has no way to hold parse work
//! across executions. Any reuse happens inside the Durable Object runtime,
//! below this API and outside drizzle's control.
//!
//! [`prepare`](crate::drizzle_prepare_impl) still helps here: it renders the
//! SQL and fixes the parameter layout once, so a loop re-binds instead of
//! re-rendering. It just cannot skip the storage engine's own parse.

pub(crate) mod prepared;
mod storage;

pub use storage::DurableStorage;

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
#[cfg(feature = "query")]
use crate::builder::sqlite::common::QueryRowFormat;
pub type Drizzle<Schema = ()> = common::Drizzle<DurableStorage, Schema>;
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, common::Drizzle<DurableStorage, Schema>, Schema, Builder, State>;

#[cfg(feature = "query")]
impl common::private::Sealed for DurableStorage {}

// Column-keyed serde rows: relational queries wrap base columns into a single
// "__base" JSON text column. See `common::QueryRowFormat`.
#[cfg(feature = "query")]
impl QueryRowFormat for DurableStorage {
    const WRAP_BASE_JSON: bool = true;
}

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
    drizzle_core::drizzle_trace_query!(&sql_str, params.len());
    let values: Vec<SqlStorageValue> = params.into_iter().map(sqlite_value_to_storage).collect();
    conn.exec(&sql_str, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

impl<Schema> common::Drizzle<DurableStorage, Schema> {
    /// Executes a statement and returns the number of rows written.
    pub fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let cursor = exec_query(self.conn.sql(), &query)?;
        // Drain the cursor so `rows_written` is populated.
        let _ = cursor
            .to_array::<serde::de::IgnoredAny>()
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
        let cursor = exec_query(self.conn.sql(), &query)?;
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
        let cursor = exec_query(self.conn.sql(), &query)?;
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
    /// panic). The transaction runs through the storage's `transactionSync`,
    /// because Durable Object SQL rejects `BEGIN`.
    ///
    /// The callback receives a `&Transaction<Schema>` that supports the same
    /// query-builder surface as `Drizzle` (select / insert / update / delete /
    /// with) plus [`Transaction::savepoint`] for nested savepoints.
    ///
    /// [`Transaction::savepoint`]: crate::transaction::sqlite::durable::Transaction::savepoint
    pub fn transaction<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: FnOnce(
            &crate::transaction::sqlite::durable::Transaction<Schema>,
        ) -> drizzle_core::error::Result<R>,
    {
        drizzle_core::drizzle_trace_tx!("begin", "sqlite.durable");
        let tx =
            crate::transaction::sqlite::durable::Transaction::new(self.conn.clone(), self.schema);
        let result = self.conn.transaction(|| f(&tx));
        if result.is_ok() {
            drizzle_core::drizzle_trace_tx!("commit", "sqlite.durable");
        } else {
            drizzle_core::drizzle_trace_tx!("rollback", "sqlite.durable");
        }
        result
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
                .sql()
                .exec(&stmt, None)
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        }
        Ok(())
    }
}

impl<Schema> common::Drizzle<DurableStorage, Schema>
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
    ///     fn new(mut state: State, _env: Env) -> Self {
    ///         let migrations = drizzle::include_migrations!("./drizzle");
    ///         let (db, _) = Drizzle::<AppSchema>::new(DurableStorage::new(&mut state));
    ///         db.migrate(&migrations, drizzle::migrations::Tracking::SQLITE)
    ///             .expect("durable migrations failed");
    ///         Self { db }
    ///     }
    ///
    ///     async fn fetch(&self, req: Request) -> worker::Result<Response> {
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
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::SQLite,
            tracking,
        );

        let sql = self.conn.sql();
        let applied_before_table_write =
            durable_applied_names_before_migration_table_write(sql, &set)?;
        super::reject_foreign_key_suspending_migrations(
            set.pending(&applied_before_table_write),
            "Durable Object",
        )?;
        ensure_durable_migration_table(sql, &set)?;

        // Durable Object storage runs this whole flow in one transaction, so
        // this path never writes a dirty marker itself. It can still inherit
        // one from a non-transactional runner against the same SQLite file, and
        // stacking migrations on an unfinished one is exactly what we refuse.
        let dirty_cursor = sql
            .exec(&set.dirty_names_sql(), None)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let dirty_names: Vec<String> = dirty_cursor
            .to_array::<AppliedName>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .map(|r| r.name)
            .collect();
        if let Some(error) = set.interrupted_migration_error(&dirty_names) {
            return Err(DrizzleError::Other(error.to_string().into()));
        }

        // Read already-applied migration names
        let applied_sql = set.applied_names_sql();
        let applied_cursor = sql
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
            return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
        }
        super::reject_foreign_key_suspending_migrations(pending.iter().copied(), "Durable Object")?;

        let applied = self.transaction(|tx| {
            let mut applied = Vec::with_capacity(pending.len());
            for migration in &pending {
                for stmt in migration.statements() {
                    if !stmt.trim().is_empty() {
                        tx.inner()
                            .sql()
                            .exec(stmt, None)
                            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                    }
                }
                tx.inner()
                    .sql()
                    .exec(&set.record_migration_sql(migration), None)
                    .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                applied.push(migration.tag().to_string());
            }
            Ok(applied)
        })?;
        Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied })
    }
}

#[derive(serde::Deserialize)]
struct AppliedName {
    name: String,
}

fn durable_applied_names_before_migration_table_write(
    conn: &SqlStorage,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<Vec<String>> {
    let table_name = set.table_name().replace('\'', "''");
    let columns = conn
        .exec(
            &format!("SELECT name FROM pragma_table_info('{}')", table_name),
            None,
        )
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;

    #[derive(serde::Deserialize)]
    struct ColumnName {
        name: String,
    }

    let columns: Vec<ColumnName> = columns
        .to_array()
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
    if columns.is_empty() {
        return Ok(Vec::new());
    }
    if columns.iter().any(|column| column.name == "name") {
        return conn
            .exec(&set.applied_names_sql(), None)
            .map_err(|error| DrizzleError::Other(error.to_string().into()))?
            .to_array::<AppliedName>()
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
        .exec(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            None,
        )
        .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
    let applied = legacy
        .to_array::<LegacyRow>()
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
        conn.exec(&set.backfill_migration_metadata_sql(&row), None)
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
        let cursor = exec_query(self.runner.conn.sql(), &self.builder.sql)?;
        let _ = cursor
            .to_array::<serde::de::IgnoredAny>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(cursor.rows_written() as u64)
    }

    /// Runs the query and returns all matching rows deserialized into `R`.
    pub fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_query(self.runner.conn.sql(), &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and returns the first matching row.
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let cursor = exec_query(self.runner.conn.sql(), &self.builder.sql)?;
        cursor
            .to_array::<R>()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
            .into_iter()
            .next()
            .ok_or(DrizzleError::NotFound)
    }
}

// =============================================================================
// Query API: find_many / find_first
// =============================================================================
//
// Durable Object SQL storage returns rows as column-keyed JSON objects rather
// than positional columns, so the relational query is always built with
// `WRAP_BASE_JSON`: the base row arrives as a single JSON `"__base"` column
// (BLOBs hex-encoded by SQL) and each relation as a JSON `"__rel_<name>"`
// column, decoded via [`drizzle_core::query::JsonQueryRow`].

#[cfg(feature = "query")]
fn query_json_rows(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<SqlStorageValue>,
) -> drizzle_core::error::Result<Vec<drizzle_core::query::JsonQueryRow>> {
    let cursor = conn
        .exec(sql, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    cursor
        .to_array::<drizzle_core::query::JsonQueryRow>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

/// Runs an `AllColumns` relational query on `conn` and decodes every row.
///
/// Shared by the `&Drizzle` and `&Transaction` runner impls.
#[cfg(feature = "query")]
pub(crate) fn relational_find_many<'a, T, Rels, Cl>(
    conn: &SqlStorage,
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
    <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
    Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
        + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
    <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
{
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
        DurableStorage::WRAP_BASE_JSON,
    );
    let (sql, bind_params) = query_sql.build();
    let values: Vec<SqlStorageValue> = bind_params
        .into_iter()
        .map(sqlite_value_to_storage)
        .collect();

    let rows = query_json_rows(conn, &sql, values)?;
    rows.into_iter()
        .map(|row| row.into_row::<_, Rels>())
        .collect()
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
    /// Executes the query and returns all matching rows with their relations.
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        relational_find_many(self.runner.conn.sql(), self.builder)
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
        <T as drizzle_core::query::QueryTable>::Select: drizzle_core::query::FromJsonObject,
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
    conn: &SqlStorage,
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
    let values: Vec<SqlStorageValue> = bind_params
        .into_iter()
        .map(sqlite_value_to_storage)
        .collect();

    let rows = query_json_rows(conn, &sql, values)?;
    rows.into_iter()
        .map(|row| row.into_row::<_, Rels>())
        .collect()
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
    /// Executes the query and returns all matching rows with their relations.
    ///
    /// Base columns are deserialized from a JSON `"__base"` column.
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
        relational_find_many_partial(self.runner.conn.sql(), self.builder)
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
    common::DrizzlePreparedQuery<'a, DurableStorage, T, Rels, drizzle_core::query::AllColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub fn find_many<const N: usize>(
        &self,
        conn: &DurableStorage,
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
        let values: Vec<SqlStorageValue> = bound.map(|v| sqlite_value_to_storage(&v)).collect();
        let rows = query_json_rows(conn.sql(), sql, values)?;
        rows.into_iter()
            .map(|row| row.into_row::<_, Rels>())
            .collect()
    }

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub fn find_first<const N: usize>(
        &self,
        conn: &DurableStorage,
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
        Ok(self.find_many(conn, params)?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, DurableStorage, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub fn find_many<const N: usize>(
        &self,
        conn: &DurableStorage,
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
        let values: Vec<SqlStorageValue> = bound.map(|v| sqlite_value_to_storage(&v)).collect();
        let rows = query_json_rows(conn.sql(), sql, values)?;
        rows.into_iter()
            .map(|row| row.into_row::<_, Rels>())
            .collect()
    }

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub fn find_first<const N: usize>(
        &self,
        conn: &DurableStorage,
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
