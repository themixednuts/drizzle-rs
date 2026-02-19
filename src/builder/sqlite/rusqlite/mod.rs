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

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Introspect the live database and return a [`Snapshot`] of its current schema.
    ///
    /// This queries `sqlite_master` and the various PRAGMAs to reconstruct the
    /// full DDL state, then packages it as a `Snapshot::Sqlite(...)`.
    pub fn introspect(&self) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        use drizzle_migrations::sqlite::{
            SQLiteDDL, Table as SqliteTable, View,
            introspect::{
                RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo, RawViewInfo,
                parse_generated_columns_from_table_sql, parse_view_sql, process_columns,
                process_foreign_keys, process_indexes, process_unique_constraints_from_indexes,
                queries,
            },
        };
        use std::collections::{HashMap, HashSet};

        // Tables
        let mut tables_stmt = self.conn.prepare(queries::TABLES_QUERY)?;
        let tables: Vec<(String, Option<String>)> = tables_stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        let table_sql_map: HashMap<String, String> = tables
            .iter()
            .filter_map(|(name, sql)| sql.as_ref().map(|s| (name.clone(), s.clone())))
            .collect();

        // Columns
        let mut columns_stmt = self.conn.prepare(queries::COLUMNS_QUERY)?;
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
            .filter_map(Result::ok)
            .collect();

        // Per-table indexes and foreign keys
        let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
        let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
        let mut all_fks: Vec<RawForeignKey> = Vec::new();

        for (table_name, _) in &tables {
            if let Ok(mut idx_stmt) = self.conn.prepare(&queries::indexes_query(table_name)) {
                let indexes: Vec<RawIndexInfo> = idx_stmt
                    .query_map([], |row| {
                        Ok(RawIndexInfo {
                            table: table_name.clone(),
                            name: row.get(1)?,
                            unique: row.get::<_, i32>(2)? != 0,
                            origin: row.get(3)?,
                            partial: row.get::<_, i32>(4)? != 0,
                        })
                    })?
                    .filter_map(Result::ok)
                    .collect();

                for idx in &indexes {
                    if let Ok(mut col_stmt) =
                        self.conn.prepare(&queries::index_info_query(&idx.name))
                        && let Ok(col_iter) = col_stmt.query_map([], |row| {
                            Ok(RawIndexColumn {
                                index_name: idx.name.clone(),
                                seqno: row.get(0)?,
                                cid: row.get(1)?,
                                name: row.get(2)?,
                                desc: row.get::<_, i32>(3)? != 0,
                                coll: row.get(4)?,
                                key: row.get::<_, i32>(5)? != 0,
                            })
                        })
                    {
                        all_index_columns.extend(col_iter.filter_map(Result::ok));
                    }
                }
                all_indexes.extend(indexes);
            }

            if let Ok(mut fk_stmt) = self.conn.prepare(&queries::foreign_keys_query(table_name))
                && let Ok(fk_iter) = fk_stmt.query_map([], |row| {
                    Ok(RawForeignKey {
                        table: table_name.clone(),
                        id: row.get(0)?,
                        seq: row.get(1)?,
                        to_table: row.get(2)?,
                        from_column: row.get(3)?,
                        to_column: row.get(4)?,
                        on_update: row.get(5)?,
                        on_delete: row.get(6)?,
                        r#match: row.get(7)?,
                    })
                })
            {
                all_fks.extend(fk_iter.filter_map(Result::ok));
            }
        }

        // Views
        let mut all_views: Vec<RawViewInfo> = Vec::new();
        if let Ok(mut views_stmt) = self.conn.prepare(queries::VIEWS_QUERY)
            && let Ok(view_iter) = views_stmt.query_map([], |row| {
                Ok(RawViewInfo {
                    name: row.get(0)?,
                    sql: row.get(1)?,
                })
            })
        {
            all_views.extend(view_iter.filter_map(Result::ok));
        }

        // Process raw → DDL entities
        let mut generated_columns: HashMap<
            String,
            drizzle_migrations::sqlite::ddl::ParsedGenerated,
        > = HashMap::new();
        for (table, sql) in &table_sql_map {
            generated_columns.extend(parse_generated_columns_from_table_sql(table, sql));
        }
        let pk_columns: HashSet<(String, String)> = raw_columns
            .iter()
            .filter(|c| c.pk > 0)
            .map(|c| (c.table.clone(), c.name.clone()))
            .collect();

        let (columns, primary_keys) =
            process_columns(&raw_columns, &generated_columns, &pk_columns);
        let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
        let foreign_keys = process_foreign_keys(&all_fks);
        let uniques = process_unique_constraints_from_indexes(&all_indexes, &all_index_columns);

        // Build DDL collection
        let mut ddl = SQLiteDDL::new();

        for (table_name, table_sql) in &tables {
            let mut table = SqliteTable::new(table_name.clone());
            if let Some(sql) = table_sql {
                let sql_upper = sql.to_uppercase();
                table.strict = sql_upper.contains(" STRICT");
                table.without_rowid = sql_upper.contains("WITHOUT ROWID");
            }
            ddl.tables.push(table);
        }
        for col in columns {
            ddl.columns.push(col);
        }
        for idx in indexes {
            ddl.indexes.push(idx);
        }
        for fk in foreign_keys {
            ddl.fks.push(fk);
        }
        for pk in primary_keys {
            ddl.pks.push(pk);
        }
        for u in uniques {
            ddl.uniques.push(u);
        }
        for v in all_views {
            let mut view = View::new(v.name);
            if let Some(def) = parse_view_sql(&v.sql) {
                view.definition = Some(def.into());
            }
            ddl.views.push(view);
        }

        // Build snapshot
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
        let stmts = drizzle_migrations::generate(&live, &desired)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for stmt in stmts {
            if !stmt.trim().is_empty() {
                self.conn.execute(&stmt, [])?;
            }
        }
        Ok(())
    }
}

impl<'a, 'b, S, Schema, State, Table, Mk, Rw>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table, Mk, Rw>, State>
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

    /// Runs the query and returns all matching rows, decoded as `R`.
    pub fn all_as<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        self.rows_as::<R>()?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a row cursor, decoded as `R`.
    pub fn rows_as<R>(self) -> drizzle_core::error::Result<Rows<R>>
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

    /// Runs the query and returns a single row, decoded as `R`.
    pub fn get_as<R>(self) -> drizzle_core::error::Result<R>
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

    /// Runs the query and returns all matching rows using the builder's row type.
    pub fn all<R, Proof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        let mut raw_rows = stmt.query(params_from_iter(params))?;
        let mut decoded = Vec::new();
        while let Some(row) = raw_rows.next()? {
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
        self.rows_as()
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub fn get<R, Proof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::rusqlite::Row<'r>, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::rusqlite::Row<'r>, Rw, R>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        stmt.query_row(params_from_iter(params), |row| {
            Ok(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::rusqlite::Row<'_>,
                R,
            >>::decode(row))
        })?
    }
}
