//! Synchronous `SQLite` driver using [`rusqlite`].
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
//! Use `column.placeholder("name")` for type-safe bind parameters.
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

mod prepared;

use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
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
    common::DrizzleBuilder<'a, common::Drizzle<Connection, Schema>, Schema, Builder, State>;

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

        let mut stmt = self
            .conn
            .prepare(&sql_str)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        stmt.query_row(params_from_iter(params.iter().copied()), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })
        .with_query(|| QueryContext::new(&sql_str, &params))?
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
    /// Apply pending migrations from an embedded migration slice.
    ///
    /// Creates the migrations table if needed and runs pending migrations in a transaction.
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

        ensure_sqlite_migration_table(&self.conn, &set)?;
        self.conn.busy_timeout(std::time::Duration::from_secs(30))?;
        self.conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
            let mut statement = self.conn.prepare(&set.applied_names_sql())?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            let applied_names = rows.collect::<Result<Vec<_>, _>>()?;
            let pending: Vec<_> = set.pending(&applied_names).collect();
            if pending.is_empty() {
                return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
            }

            let mut applied = Vec::with_capacity(pending.len());
            for migration in &pending {
                for stmt in migration.statements() {
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

        match result {
            Ok(outcome) => {
                self.conn.execute("COMMIT", [])?;
                Ok(outcome)
            }
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }
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
            let update_sql = format!(
                "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
                set.table_ident_sql(),
                escaped_name,
                where_clause
            );
            conn.execute(&update_sql, [])?;
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

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Introspect the live database and return a [`Snapshot`] of its current schema.
    ///
    /// This queries `sqlite_master` and the various PRAGMAs to reconstruct the
    /// full DDL state, then packages it as a `Snapshot::Sqlite(...)`.
    pub fn introspect(&self) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        let tables = introspect_query_tables(&self.conn)?;
        let raw_columns = introspect_query_columns(&self.conn)?;
        let (all_indexes, all_index_columns, all_fks) =
            introspect_query_indexes_and_fks(&self.conn)?;
        let all_views = introspect_query_views(&self.conn)?;

        let ddl = drizzle_migrations::sqlite::introspect::assemble_ddl(
            drizzle_migrations::sqlite::introspect::RawIntrospection {
                tables,
                columns: raw_columns,
                indexes: all_indexes,
                index_columns: all_index_columns,
                foreign_keys: all_fks,
                views: all_views,
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
    pub fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        let live = self.introspect()?;
        let desired = schema.to_snapshot();
        let generated = drizzle_migrations::diff(&live, &desired)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for stmt in generated.statements {
            if !stmt.trim().is_empty() {
                self.conn.execute(&stmt, [])?;
            }
        }
        Ok(())
    }
}

// =============================================================================
// Query API: find_many / find_first
// =============================================================================

#[cfg(feature = "query")]
use drizzle_core::query::DeserializeStore as _;
#[cfg(feature = "query")]
use drizzle_core::query::FromJsonObject as _;

// AllColumns: read base from individual row columns via TryFrom<Row>
#[cfg(feature = "query")]
impl<'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        '_,
        'a,
        Connection,
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
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let num_base_cols = T::COLUMN_NAMES.len();

        let builder = self.builder;
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

        let mut stmt = self
            .runner
            .conn
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
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(row)
                .map_err(Into::into)?;

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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }
}

// AllColumns find_first: requires no LIMIT set yet (internally adds LIMIT 1)
#[cfg(feature = "query")]
impl<'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        '_,
        'a,
        Connection,
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
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

// PartialColumns: read base from a single JSON "__base" column via FromJsonObject
#[cfg(feature = "query")]
impl<'a, Schema, T, Rels, Cl>
    common::DrizzleQueryBuilder<
        '_,
        'a,
        Connection,
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
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore
            + drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let builder = self.builder;
        let column_names = &builder.cols.columns;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = column_names.clone();
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

        let mut stmt = self
            .runner
            .conn
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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }
}

// PartialColumns find_first: requires no LIMIT set yet
#[cfg(feature = "query")]
impl<'a, Schema, T, Rels, W, Ord>
    common::DrizzleQueryBuilder<
        '_,
        'a,
        Connection,
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
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore
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
    /// Executes the prepared relational query and returns all matching rows.
    pub fn find_many<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Vec<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore,
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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub fn find_first<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Option<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::Select,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildStore,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(conn, params)?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Connection, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Executes the prepared relational query and returns all matching rows.
    pub fn find_many<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Vec<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore,
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

            results.push(drizzle_core::query::QueryRow::new(base, store));
        }

        Ok(results)
    }

    /// Executes the prepared relational query and returns the first row, if any.
    ///
    /// To apply `LIMIT 1` in SQL, call `.limit(1)` before `.prepare()`.
    pub fn find_first<const N: usize>(
        &self,
        conn: &Connection,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Option<
            drizzle_core::query::QueryRow<
                <T as drizzle_core::query::QueryTable>::PartialSelect,
                <Rels as drizzle_core::query::BuildStore>::Store,
            >,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::PartialSelect: drizzle_core::query::FromJsonObject,
        Rels: drizzle_core::query::BuildStore,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(conn, params)?.into_iter().next())
    }
}

impl<S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'_, S, QueryBuilder<'_, Schema, State, Table, Mk, Rw, Grouped>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());
        self.runner
            .conn
            .execute(&sql_str, params_from_iter(params.iter().copied()))
            .with_query(|| QueryContext::new(&sql_str, &params))
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R>,
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

    /// Runs the query and returns a row cursor using the builder's row type.
    pub fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
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

    /// Runs the query and returns a single row using the builder's row type.
    pub fn get<R, Proof, AggProof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R>,
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
