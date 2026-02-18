//! Async PostgreSQL driver using [`tokio_postgres`].
//!
//! # Quick start
//!
//! ```no_run
//! use drizzle::postgres::prelude::*;
//! use drizzle::postgres::tokio::Drizzle;
//!
//! #[PostgresTable]
//! struct User {
//!     #[column(serial, primary)]
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(PostgresSchema)]
//! struct AppSchema {
//!     user: User,
//! }
//!
//! #[tokio::main]
//! async fn main() -> drizzle::Result<()> {
//!     let (client, connection) = ::tokio_postgres::connect(
//!         "host=localhost user=postgres", ::tokio_postgres::NoTls,
//!     ).await?;
//!     tokio::spawn(async move { connection.await.unwrap() });
//!
//!     let (db, AppSchema { user }) = Drizzle::new(client, AppSchema::new());
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
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! use drizzle::postgres::common::PostgresTransactionType;
//!
//! let count = db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
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
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # use drizzle::postgres::common::PostgresTransactionType;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
//!
//!     // This savepoint fails — only its changes roll back
//!     let _: Result<(), _> = tx.savepoint(async |stx| {
//!         stx.insert(user).values([InsertUser::new("Bad")]).execute().await?;
//!         Err(drizzle::error::DrizzleError::Other("oops".into()))
//!     }).await;
//!
//!     // Alice is still there
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
//!     assert_eq!(users.len(), 1);
//!     Ok(())
//! }).await?;
//! # Ok(()) }
//! ```
//!
//! # Cloning for `tokio::spawn`
//!
//! `Drizzle` is cheaply cloneable (the underlying client is behind an
//! [`Arc`]). Move a clone into spawned tasks for concurrent queries.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (db, S { user }) = Drizzle::new(client, S::new());
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
//!
//! # Prepared statements
//!
//! Build a query once and execute it many times with different parameters.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::tokio::Drizzle;
//! # use drizzle::core::expr::eq;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # #[tokio::main] async fn main() -> drizzle::Result<()> {
//! # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
//! # tokio::spawn(async move { conn.await.unwrap() });
//! # let (db, S { user }) = Drizzle::new(client, S::new());
//! use drizzle::postgres::params;
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, Placeholder::named("find_name")))
//!     .prepare();
//!
//! let alice: Vec<SelectUser> = find_user
//!     .all(db.conn(), params![{find_name: "Alice"}])
//!     .await?;
//! # Ok(()) }
//! ```

mod prepared;

use std::sync::Arc;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use smallvec::SmallVec;
use tokio_postgres::{Client, IsolationLevel, Row};

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;

use crate::builder::postgres::common;
use crate::builder::postgres::rows::DecodeRows;

/// Tokio-postgres-specific drizzle builder
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

use crate::transaction::postgres::tokio_postgres::Transaction;

crate::drizzle_prepare_impl!();

/// Async PostgreSQL database wrapper using [`tokio_postgres::Client`].
///
/// Provides query building methods (`select`, `insert`, `update`, `delete`)
/// and execution methods (`execute`, `all`, `get`, `transaction`).
///
/// The client is stored behind an [`Arc`], making `Drizzle` cheaply cloneable
/// for sharing across tasks (e.g. with [`tokio::spawn`]).
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    client: Arc<Client>,
    schema: Schema,
}

impl<S: Clone> Clone for Drizzle<S> {
    #[inline]
    fn clone(&self) -> Self {
        Drizzle {
            client: self.client.clone(),
            schema: self.schema.clone(),
        }
    }
}

/// Lazy decoded row cursor for tokio-postgres queries.
pub type Rows<R> = DecodeRows<Row, R>;

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub fn new<S: Copy>(client: Client, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            client: Arc::new(client),
            schema,
        };
        (drizzle, schema)
    }
}

impl<S> AsRef<Drizzle<S>> for Drizzle<S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    /// Gets a reference to the underlying connection.
    #[inline]
    pub fn conn(&self) -> &Client {
        &self.client
    }

    /// Gets a mutable reference to the underlying connection.
    ///
    /// Returns `None` if there are outstanding clones of this `Drizzle` instance.
    #[inline]
    pub fn conn_mut(&mut self) -> Option<&mut Client> {
        Arc::get_mut(&mut self.client)
    }

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    postgres_builder_constructors!();

    pub async fn execute<'a, T>(&'a self, query: T) -> Result<u64, tokio_postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let query = query.to_sql();
        let (sql, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.execute");
            let (sql, params) = query.build();
            drizzle_core::drizzle_trace_query!(&sql, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql, param_refs)
        };

        self.client.execute(&sql, &param_refs[..]).await
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)
            .await?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    pub async fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.all");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let rows = self.client.query(&sql_str, &param_refs[..]).await?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = query.to_sql();
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "drizzle.get");
            let (sql_str, params) = sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let row = self.client.query_one(&sql_str, &param_refs[..]).await?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Executes a transaction with the given callback.
    ///
    /// The transaction is committed when the callback returns `Ok` and
    /// rolled back on `Err`. Requires `&mut self` because the underlying
    /// client must not be shared during a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if there are outstanding clones of this `Drizzle` instance,
    /// since exclusive access to the underlying client is required for transactions.
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::tokio::Drizzle;
    /// # use drizzle::postgres::common::PostgresTransactionType;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # #[tokio::main] async fn main() -> drizzle::Result<()> {
    /// # let (client, conn) = ::tokio_postgres::connect("host=localhost user=postgres", ::tokio_postgres::NoTls).await?;
    /// # tokio::spawn(async move { conn.await.unwrap() });
    /// # let (mut db, S { user }) = Drizzle::new(client, S::new());
    /// let count = db.transaction(PostgresTransactionType::ReadCommitted, async |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute().await?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all().await?;
    ///     Ok(users.len())
    /// }).await?;
    /// # Ok(()) }
    /// ```
    pub async fn transaction<F, R>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: AsyncFnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let client = Arc::get_mut(&mut self.client).ok_or_else(|| {
            DrizzleError::Other("cannot start transaction: outstanding Drizzle clones exist".into())
        })?;
        let builder = client.build_transaction();
        let builder = if tx_type != PostgresTransactionType::default() {
            let isolation = match tx_type {
                PostgresTransactionType::ReadUncommitted => IsolationLevel::ReadUncommitted,
                PostgresTransactionType::ReadCommitted => IsolationLevel::ReadCommitted,
                PostgresTransactionType::RepeatableRead => IsolationLevel::RepeatableRead,
                PostgresTransactionType::Serializable => IsolationLevel::Serializable,
            };
            builder.isolation_level(isolation)
        } else {
            builder
        };
        drizzle_core::drizzle_trace_tx!("begin", "postgres.tokio");
        let tx = builder.start().await?;

        let transaction = Transaction::new(tx, tx_type, self.schema);

        match f(&transaction).await {
            Ok(value) => {
                drizzle_core::drizzle_trace_tx!("commit", "postgres.tokio");
                transaction.commit().await?;
                Ok(value)
            }
            Err(e) => {
                drizzle_core::drizzle_trace_tx!("rollback", "postgres.tokio");
                transaction.rollback().await?;
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
        let statements = schema.create_statements()?;

        for statement in statements {
            self.client.execute(&statement, &[]).await?;
        }

        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Apply pending migrations from a MigrationSet.
    ///
    /// Creates the drizzle schema if needed and runs pending migrations in a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if there are outstanding clones of this `Drizzle` instance,
    /// since exclusive access to the underlying client is required for the migration transaction.
    pub async fn migrate(
        &mut self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        if let Some(schema_sql) = migrations.create_schema_sql() {
            self.client.execute(&schema_sql, &[]).await?;
        }
        self.client
            .execute(&migrations.create_table_sql(), &[])
            .await?;
        let rows = self
            .client
            .query(&migrations.query_all_created_at_sql(), &[])
            .await?;
        let applied_created_at: Vec<i64> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();
        let pending: Vec<_> = migrations
            .pending_by_created_at(&applied_created_at)
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        let client = Arc::get_mut(&mut self.client).ok_or_else(|| {
            DrizzleError::Other("cannot run migrations: outstanding Drizzle clones exist".into())
        })?;
        let tx = client.transaction().await?;

        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    tx.execute(stmt, &[]).await?;
                }
            }
            tx.execute(
                &migrations.record_migration_sql(migration.hash(), migration.created_at()),
                &[],
            )
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Introspect the connected PostgreSQL database and return a [`Snapshot`](drizzle_migrations::schema::Snapshot).
    ///
    /// Queries the `pg_catalog` and `information_schema` to extract tables, columns,
    /// indexes, foreign keys, primary keys, unique/check constraints, enums, sequences,
    /// views, roles, and policies.
    pub async fn introspect(
        &self,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        self.introspect_impl(None).await
    }

    /// Inner introspection with optional schema filter.
    ///
    /// When `schema_filter` is `Some`, queries that use `pg_get_indexdef()` or
    /// `pg_get_expr()` are scoped to those schemas.  These functions call
    /// `relation_open()` which is not MVCC-protected and can fail when
    /// concurrent DDL drops objects in other schemas.
    async fn introspect_impl(
        &self,
        schema_filter: Option<&[String]>,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        use drizzle_migrations::postgres::introspect::{
            RawCheckInfo, RawColumnInfo, RawEnumInfo, RawForeignKeyInfo, RawIndexInfo,
            RawPolicyInfo, RawPrimaryKeyInfo, RawRoleInfo, RawSequenceInfo, RawTableInfo,
            RawUniqueInfo, RawViewInfo, parse_index_columns, pg_action_code_to_string,
            process_check_constraints, process_columns, process_enums, process_foreign_keys,
            process_indexes, process_policies, process_primary_keys, process_roles,
            process_sequences, process_tables, process_unique_constraints, process_views, queries,
        };
        use drizzle_migrations::postgres::{PostgresDDL, ddl::Schema as PgSchema};

        let err = |msg: &str, e: tokio_postgres::Error| -> DrizzleError {
            DrizzleError::Other(format!("{msg}: {e}").into())
        };

        // Schemas
        let schemas: Vec<PgSchema> = self
            .client
            .query(queries::SCHEMAS_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query schemas", e))?
            .into_iter()
            .map(|row| PgSchema::new(row.get::<_, String>(0)))
            .collect();

        // Tables
        let raw_tables: Vec<RawTableInfo> = self
            .client
            .query(queries::TABLES_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query tables", e))?
            .into_iter()
            .map(|row| RawTableInfo {
                schema: row.get(0),
                name: row.get(1),
                is_rls_enabled: row.get(2),
            })
            .collect();

        // Columns
        let raw_columns: Vec<RawColumnInfo> = self
            .client
            .query(queries::COLUMNS_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query columns", e))?
            .into_iter()
            .map(|row| RawColumnInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                column_type: row.get(3),
                type_schema: row.get(4),
                not_null: row.get(5),
                default_value: row.get(6),
                is_identity: row.get(7),
                identity_type: row.get(8),
                is_generated: row.get(9),
                generated_expression: row.get(10),
                ordinal_position: row.get(11),
            })
            .collect();

        // Enums
        let raw_enums: Vec<RawEnumInfo> = self
            .client
            .query(queries::ENUMS_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query enums", e))?
            .into_iter()
            .map(|row| RawEnumInfo {
                schema: row.get(0),
                name: row.get(1),
                values: row.get(2),
            })
            .collect();

        // Sequences
        let raw_sequences: Vec<RawSequenceInfo> = self
            .client
            .query(queries::SEQUENCES_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query sequences", e))?
            .into_iter()
            .map(|row| RawSequenceInfo {
                schema: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                min_value: row.get(4),
                max_value: row.get(5),
                increment: row.get(6),
                cycle: row.get(7),
                cache_value: row.get(8),
            })
            .collect();

        // Views
        let raw_views: Vec<RawViewInfo> = self
            .client
            .query(queries::VIEWS_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query views", e))?
            .into_iter()
            .map(|row| RawViewInfo {
                schema: row.get(0),
                name: row.get(1),
                definition: row.get(2),
                is_materialized: row.get(3),
            })
            .collect();

        // Indexes — use schema-filtered variant when available to avoid
        // pg_get_indexdef() failures from concurrent DDL in other schemas.
        let raw_indexes: Vec<RawIndexInfo> = if let Some(schemas) = schema_filter {
            self.client
                .query(queries::INDEXES_QUERY_FILTERED, &[&schemas])
                .await
                .map_err(|e| err("Failed to query indexes", e))?
        } else {
            self.client
                .query(queries::INDEXES_QUERY, &[])
                .await
                .map_err(|e| err("Failed to query indexes", e))?
        }
        .into_iter()
        .map(|row| RawIndexInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            is_unique: row.get(3),
            is_primary: row.get(4),
            method: row.get(5),
            columns: parse_index_columns(row.get(6)),
            where_clause: row.get(7),
            concurrent: false,
        })
        .collect();

        // Foreign keys
        let raw_fks: Vec<RawForeignKeyInfo> = self
            .client
            .query(queries::FOREIGN_KEYS_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query foreign keys", e))?
            .into_iter()
            .map(|row| RawForeignKeyInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                columns: row.get(3),
                schema_to: row.get(4),
                table_to: row.get(5),
                columns_to: row.get(6),
                on_update: pg_action_code_to_string(&row.get::<_, String>(7)),
                on_delete: pg_action_code_to_string(&row.get::<_, String>(8)),
            })
            .collect();

        // Primary keys
        let raw_pks: Vec<RawPrimaryKeyInfo> = self
            .client
            .query(queries::PRIMARY_KEYS_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query primary keys", e))?
            .into_iter()
            .map(|row| RawPrimaryKeyInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                columns: row.get(3),
            })
            .collect();

        // Unique constraints
        let raw_uniques: Vec<RawUniqueInfo> = self
            .client
            .query(queries::UNIQUES_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query unique constraints", e))?
            .into_iter()
            .map(|row| RawUniqueInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                columns: row.get(3),
                nulls_not_distinct: row.get(4),
            })
            .collect();

        // Check constraints — use schema-filtered variant when available.
        let raw_checks: Vec<RawCheckInfo> = if let Some(schemas) = schema_filter {
            self.client
                .query(queries::CHECKS_QUERY_FILTERED, &[&schemas])
                .await
                .map_err(|e| err("Failed to query check constraints", e))?
        } else {
            self.client
                .query(queries::CHECKS_QUERY, &[])
                .await
                .map_err(|e| err("Failed to query check constraints", e))?
        }
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            expression: row.get(3),
        })
        .collect();

        // Roles
        let raw_roles: Vec<RawRoleInfo> = self
            .client
            .query(queries::ROLES_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query roles", e))?
            .into_iter()
            .map(|row| RawRoleInfo {
                name: row.get(0),
                create_db: row.get(1),
                create_role: row.get(2),
                inherit: row.get(3),
            })
            .collect();

        // Policies
        let raw_policies: Vec<RawPolicyInfo> = self
            .client
            .query(queries::POLICIES_QUERY, &[])
            .await
            .map_err(|e| err("Failed to query policies", e))?
            .into_iter()
            .map(|row| RawPolicyInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                as_clause: row.get(3),
                for_clause: row.get(4),
                to: row.get(5),
                using: row.get(6),
                with_check: row.get(7),
            })
            .collect();

        // Process raw → DDL entities
        let mut ddl = PostgresDDL::new();
        for s in schemas {
            ddl.schemas.push(s);
        }
        for e in process_enums(&raw_enums) {
            ddl.enums.push(e);
        }
        for s in process_sequences(&raw_sequences) {
            ddl.sequences.push(s);
        }
        for r in process_roles(&raw_roles) {
            ddl.roles.push(r);
        }
        for p in process_policies(&raw_policies) {
            ddl.policies.push(p);
        }
        for t in process_tables(&raw_tables) {
            ddl.tables.push(t);
        }
        for c in process_columns(&raw_columns) {
            ddl.columns.push(c);
        }
        for i in process_indexes(&raw_indexes) {
            ddl.indexes.push(i);
        }
        for fk in process_foreign_keys(&raw_fks) {
            ddl.fks.push(fk);
        }
        for pk in process_primary_keys(&raw_pks) {
            ddl.pks.push(pk);
        }
        for u in process_unique_constraints(&raw_uniques) {
            ddl.uniques.push(u);
        }
        for c in process_check_constraints(&raw_checks) {
            ddl.checks.push(c);
        }
        for v in process_views(&raw_views) {
            ddl.views.push(v);
        }

        // Build snapshot
        let mut snap = drizzle_migrations::postgres::PostgresSnapshot::new();
        for entity in ddl.to_entities() {
            snap.add_entity(entity);
        }

        Ok(drizzle_migrations::schema::Snapshot::Postgres(snap))
    }

    /// Introspect the live database, diff against the desired schema, and
    /// execute the SQL statements needed to bring the database in sync.
    ///
    /// This is a no-op if the database already matches.
    pub async fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        let desired = schema.to_snapshot();
        let target_schemas: Vec<String> = match &desired {
            drizzle_migrations::schema::Snapshot::Postgres(pg) => pg.schema_names(),
            _ => Vec::new(),
        };
        let live = self
            .introspect_impl(if target_schemas.is_empty() {
                None
            } else {
                Some(&target_schemas)
            })
            .await?;
        let live = match (live, &desired) {
            (
                drizzle_migrations::schema::Snapshot::Postgres(live_pg),
                drizzle_migrations::schema::Snapshot::Postgres(desired_pg),
            ) => {
                drizzle_migrations::schema::Snapshot::Postgres(live_pg.prepare_for_push(desired_pg))
            }
            (other, _) => other,
        };
        let stmts = drizzle_migrations::generate(&live, &desired)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        for stmt in stmts {
            if !stmt.trim().is_empty() {
                self.client.execute(&*stmt, &[]).await?;
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
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.execute");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        Ok(self
            .drizzle
            .client
            .execute(&sql_str, &param_refs[..])
            .await?)
    }

    /// Runs the query and returns all matching rows, decoded as the given type `R`.
    pub async fn all_as<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        self.rows_as::<R>()
            .await?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor, decoded as the given type `R`.
    pub async fn rows_as<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.all");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..]).await?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row, decoded as the given type `R`.
    pub async fn get_as<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.get");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let row = self
            .drizzle
            .client
            .query_one(&sql_str, &param_refs[..])
            .await?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub async fn all<R, Proof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::tokio_postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.all");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..]).await?;
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::tokio_postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a lazy row cursor using the builder's row type.
    pub async fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        self.rows_as().await
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub async fn get<R, Proof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::tokio_postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>,
    {
        let (sql_str, param_refs) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.tokio", "builder.get");
            let (sql_str, params) = self.builder.sql.build();
            drizzle_core::drizzle_trace_query!(&sql_str, params.len());

            let param_refs: SmallVec<[&(dyn tokio_postgres::types::ToSql + Sync); 8]> = params
                .iter()
                .map(|&p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();
            (sql_str, param_refs)
        };

        let row = self
            .drizzle
            .client
            .query_one(&sql_str, &param_refs[..])
            .await?;
        <Mk as drizzle_core::row::DecodeSelectedRef<&::tokio_postgres::Row, R>>::decode(&row)
    }
}
