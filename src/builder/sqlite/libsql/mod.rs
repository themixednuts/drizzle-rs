//! Async SQLite driver using [`libsql`].
//!
//! # Quick start
//!
//! ```ignore
//! use drizzle::sqlite::prelude::*;
//! use drizzle::sqlite::libsql::Drizzle;
//! use libsql::Builder;
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
//! ```ignore
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::sqlite::libsql::Drizzle;
//! use drizzle::sqlite::connection::SQLiteTransactionType;
//!
//! let count = db.transaction(SQLiteTransactionType::Deferred, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     Ok(users.len())
//! }).await?;
//! ```
//!
//! # Savepoints
//!
//! Savepoints nest inside transactions — a failed savepoint rolls back
//! without aborting the outer transaction.
//!
//! ```ignore
//! # use drizzle::sqlite::prelude::*;
//! # use drizzle::sqlite::libsql::Drizzle;
//! # use drizzle::sqlite::connection::SQLiteTransactionType;
//! db.transaction(SQLiteTransactionType::Deferred, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!
//!     // This savepoint fails — only its changes roll back
//!     let _ = tx.savepoint(async |stx| {
//!         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
//!         Err(drizzle::error::DrizzleError::Other("oops".into()))
//!     }).await;
//!
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     assert_eq!(users.len(), 1); // only Alice
//!     Ok(())
//! }).await?;
//! ```

mod prepared;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use libsql::{Connection, Row};

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{self, QueryBuilder},
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

crate::drizzle_prepare_impl!();

use crate::builder::sqlite::common;
use crate::builder::sqlite::rows::LibsqlRows as Rows;
use crate::transaction::sqlite::libsql::Transaction;

pub type Drizzle<Schema = ()> = common::Drizzle<Connection, Schema>;
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, Connection, Schema, Builder, State>;

impl<Schema> common::Drizzle<Connection, Schema> {
    pub async fn execute<'a, T>(
        &'a self,
        query: T,
    ) -> Result<u64, drizzle_core::error::DrizzleError>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let (sql, params) = query.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        self.conn
            .execute(&sql, params)
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
        C: Default + Extend<R>,
    {
        self.rows(query).await?.collect().await
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
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

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
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let mut rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }

    /// Executes a transaction with the given callback.
    ///
    /// The transaction is committed when the callback returns `Ok` and
    /// rolled back on `Err`. Unlike the sync rusqlite driver, `transaction`
    /// takes `&self` (not `&mut self`).
    ///
    /// ```ignore
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::libsql::Drizzle;
    /// # use drizzle::sqlite::connection::SQLiteTransactionType;
    /// let count = db.transaction(SQLiteTransactionType::Deferred, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     Ok(users.len())
    /// }).await?;
    /// ```
    pub async fn transaction<F, R>(
        &self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: AsyncFnOnce(&Transaction<Schema>) -> Result<R, DrizzleError>,
    {
        let tx = self.conn.transaction_with_behavior(tx_type.into()).await?;
        let transaction = Transaction::new(tx, tx_type, self.schema);

        let result = f(&transaction).await;
        match result {
            Ok(val) => {
                transaction.commit().await?;
                Ok(val)
            }
            Err(e) => {
                let _ = transaction.rollback().await;
                Err(e)
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
        let statements: Vec<_> = schema.create_statements()?.collect();
        if !statements.is_empty() {
            let batch_sql = statements.join(";");
            self.conn.execute_batch(&batch_sql).await?;
        }
        Ok(())
    }
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Apply pending migrations from a MigrationSet.
    ///
    /// Creates the migrations table if needed and runs pending migrations in a transaction.
    pub async fn migrate(
        &self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        self.conn
            .execute(&migrations.create_table_sql(), ())
            .await?;
        let mut rows = self
            .conn
            .query(&migrations.query_all_created_at_sql(), ())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut applied_created_at: Vec<i64> = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        {
            if let Ok(created_at) = row.get::<i64>(0) {
                applied_created_at.push(created_at);
            }
        }

        let pending: Vec<_> = migrations
            .pending_by_created_at(&applied_created_at)
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    tx.execute(stmt, ())
                        .await
                        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
                }
            }
            tx.execute(
                &migrations.record_migration_sql(migration.hash(), migration.created_at()),
                (),
            )
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        }

        tx.commit()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        Ok(())
    }
}

impl<Schema> common::Drizzle<Connection, Schema> {
    /// Introspect the live database and return a [`Snapshot`] of its current schema.
    pub async fn introspect(
        &self,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
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
        let mut tables_rows = self
            .conn
            .query(queries::TABLES_QUERY, ())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut tables: Vec<(String, Option<String>)> = Vec::new();
        while let Ok(Some(row)) = tables_rows.next().await {
            let name: String = row.get(0).unwrap_or_default();
            let sql: Option<String> = row.get(1).ok();
            tables.push((name, sql));
        }

        let table_sql_map: HashMap<String, String> = tables
            .iter()
            .filter_map(|(name, sql)| sql.as_ref().map(|s| (name.clone(), s.clone())))
            .collect();

        // Columns
        let mut columns_rows = self
            .conn
            .query(queries::COLUMNS_QUERY, ())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut raw_columns: Vec<RawColumnInfo> = Vec::new();
        while let Ok(Some(row)) = columns_rows.next().await {
            raw_columns.push(RawColumnInfo {
                table: row.get(0).unwrap_or_default(),
                cid: row.get(1).unwrap_or(0),
                name: row.get(2).unwrap_or_default(),
                column_type: row.get(3).unwrap_or_default(),
                not_null: row.get::<i32>(4).unwrap_or(0) != 0,
                default_value: row.get(5).ok(),
                pk: row.get(6).unwrap_or(0),
                hidden: row.get(7).unwrap_or(0),
                sql: row.get(8).ok(),
            });
        }

        // Per-table indexes and foreign keys
        let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
        let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
        let mut all_fks: Vec<RawForeignKey> = Vec::new();

        for (table_name, _) in &tables {
            if let Ok(mut idx_rows) = self
                .conn
                .query(&queries::indexes_query(table_name), ())
                .await
            {
                while let Ok(Some(row)) = idx_rows.next().await {
                    let idx = RawIndexInfo {
                        table: table_name.clone(),
                        name: row.get(1).unwrap_or_default(),
                        unique: row.get::<i32>(2).unwrap_or(0) != 0,
                        origin: row.get(3).unwrap_or_default(),
                        partial: row.get::<i32>(4).unwrap_or(0) != 0,
                    };

                    if let Ok(mut col_rows) = self
                        .conn
                        .query(&queries::index_info_query(&idx.name), ())
                        .await
                    {
                        while let Ok(Some(col_row)) = col_rows.next().await {
                            all_index_columns.push(RawIndexColumn {
                                index_name: idx.name.clone(),
                                seqno: col_row.get(0).unwrap_or(0),
                                cid: col_row.get(1).unwrap_or(0),
                                name: col_row.get(2).ok(),
                                desc: col_row.get::<i32>(3).unwrap_or(0) != 0,
                                coll: col_row.get(4).unwrap_or_default(),
                                key: col_row.get::<i32>(5).unwrap_or(0) != 0,
                            });
                        }
                    }

                    all_indexes.push(idx);
                }
            }

            if let Ok(mut fk_rows) = self
                .conn
                .query(&queries::foreign_keys_query(table_name), ())
                .await
            {
                while let Ok(Some(row)) = fk_rows.next().await {
                    all_fks.push(RawForeignKey {
                        table: table_name.clone(),
                        id: row.get(0).unwrap_or(0),
                        seq: row.get(1).unwrap_or(0),
                        to_table: row.get(2).unwrap_or_default(),
                        from_column: row.get(3).unwrap_or_default(),
                        to_column: row.get(4).unwrap_or_default(),
                        on_update: row.get(5).unwrap_or_default(),
                        on_delete: row.get(6).unwrap_or_default(),
                        r#match: row.get(7).unwrap_or_default(),
                    });
                }
            }
        }

        // Views
        let mut all_views: Vec<RawViewInfo> = Vec::new();
        if let Ok(mut views_rows) = self.conn.query(queries::VIEWS_QUERY, ()).await {
            while let Ok(Some(row)) = views_rows.next().await {
                let name: String = row.get(0).unwrap_or_default();
                let sql: String = row.get(1).unwrap_or_default();
                all_views.push(RawViewInfo { name, sql });
            }
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
    pub async fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        let live = self.introspect().await?;
        let desired = schema.to_snapshot();
        let stmts = drizzle_migrations::generate(&live, &desired)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for stmt in stmts {
            if !stmt.trim().is_empty() {
                self.conn.execute(&stmt, ()).await?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "libsql")]
impl<'a, 'b, S, Schema, State, Table, Mk, Rw>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table, Mk, Rw>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();
        Ok(self.drizzle.conn.execute(&sql_str, params).await?)
    }

    /// Runs the query and returns all matching rows, decoded as `R`.
    pub async fn all_as<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: Default + Extend<R>,
    {
        self.rows_as::<R>().await?.collect().await
    }

    /// Runs the query and returns a row cursor, decoded as `R`.
    pub async fn rows_as<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self.drizzle.conn.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row, decoded as `R`.
    pub async fn get_as<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(drizzle_core::error::DrizzleError::NotFound)
        }
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();
        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        let mut decoded = Vec::new();
        while let Some(row) = rows.next().await? {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::libsql::Row,
                R,
            >>::decode(&row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a row cursor using the builder's row type.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <Rw as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        self.rows_as().await
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::libsql::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();
        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        if let Some(row) = rows.next().await? {
            <Mk as drizzle_core::row::DecodeSelectedRef<&::libsql::Row, R>>::decode(&row)
        } else {
            Err(drizzle_core::error::DrizzleError::NotFound)
        }
    }
}
