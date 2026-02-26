//! Synchronous PostgreSQL driver using [`postgres`].
//!
//! # Quick start
//!
//! ```no_run
//! use drizzle::postgres::prelude::*;
//! use drizzle::postgres::sync::Drizzle;
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
//! fn main() -> drizzle::Result<()> {
//!     let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
//!     let (mut db, AppSchema { user }) = Drizzle::new(client, AppSchema::new());
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
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::sync::Drizzle;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! use drizzle::postgres::common::PostgresTransactionType;
//!
//! let count = db.transaction(PostgresTransactionType::ReadCommitted, |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
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
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::sync::Drizzle;
//! # use drizzle::postgres::common::PostgresTransactionType;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! db.transaction(PostgresTransactionType::ReadCommitted, |tx| {
//!     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
//!
//!     // This savepoint fails — only its changes roll back
//!     let _: Result<(), _> = tx.savepoint(|stx| {
//!         stx.insert(user).values([InsertUser::new("Bad")]).execute()?;
//!         Err(drizzle::error::DrizzleError::Other("oops".into()))
//!     });
//!
//!     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
//!     assert_eq!(users.len(), 1); // only Alice
//!     Ok(())
//! })?;
//! # Ok(()) }
//! ```
//!
//! # Prepared statements
//!
//! Build a query once and execute it many times with different parameters.
//!
//! ```no_run
//! # use drizzle::postgres::prelude::*;
//! # use drizzle::postgres::sync::Drizzle;
//! # use drizzle::core::expr::eq;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
//! # let (mut db, S { user }) = Drizzle::new(client, S::new());
//! use drizzle::postgres::params;
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, Placeholder::named("find_name")))
//!     .prepare()
//!     .into_owned();
//!
//! let alice: Vec<SelectUser> = find_user
//!     .all(db.conn_mut(), params![{find_name: "Alice"}])?;
//! # Ok(()) }
//! ```

mod prepared;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use postgres::fallible_iterator::FallibleIterator;
use postgres::{Client, IsolationLevel, Row};

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

use crate::builder::postgres::common;
use crate::builder::postgres::rows::DecodeRows;

/// Postgres-specific drizzle builder
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a mut Drizzle<Schema>, Schema, Builder, State>;

use crate::transaction::postgres::postgres_sync::Transaction;

crate::drizzle_prepare_impl!();

/// Synchronous PostgreSQL database wrapper using [`postgres::Client`].
///
/// Provides query building methods (`select`, `insert`, `update`, `delete`)
/// and execution methods (`execute`, `all`, `get`, `transaction`).
pub struct Drizzle<Schema = ()> {
    client: Client,
    schema: Schema,
}

/// Lazy decoded row cursor for postgres sync queries.
pub type Rows<R> = DecodeRows<Row, R>;

impl Drizzle {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub const fn new<S: Copy>(client: Client, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle { client, schema };
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
    #[inline]
    pub fn conn_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    postgres_builder_constructors!(mut);

    pub fn execute<'a, T>(&'a mut self, query: T) -> Result<u64, postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.execute");
        let query = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.execute.build");
        let (sql, params) = query.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());

        let param_refs = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.execute.param_refs");
            let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
                SmallVec::with_capacity(params.len());
            param_refs.extend(
                params
                    .iter()
                    .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
            );
            param_refs
        };

        let mut typed_params: SmallVec<
            [(&(dyn postgres::types::ToSql + Sync), postgres::types::Type); 8],
        > = SmallVec::with_capacity(params.len());
        let mut all_typed = true;
        for p in &params {
            if let Some(ty) = crate::builder::postgres::prepared_common::postgres_sync_param_type(p)
            {
                typed_params.push((*p as &(dyn postgres::types::ToSql + Sync), ty));
            } else {
                all_typed = false;
                break;
            }
        }

        if all_typed {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.execute.db_typed");
            let mut rows = self.client.query_typed_raw(&sql, typed_params)?;
            while rows.next()?.is_some() {}
            return Ok(rows.rows_affected().unwrap_or(0));
        }

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.execute.db");
        self.client.execute(&sql, &param_refs[..])
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<'a, T, R, C>(&'a mut self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    pub fn rows<'a, T, R>(&'a mut self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.all");
        let sql = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.all.build");
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let rows = self.client.query(&sql_str, &param_refs[..])?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a mut self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.get");
        let sql = query.to_sql();
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.get.build");
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let row = self.client.query_one(&sql_str, &param_refs[..])?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Executes a transaction with the given callback.
    ///
    /// The transaction is committed when the callback returns `Ok` and
    /// rolled back on `Err` or panic.
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # use drizzle::postgres::common::PostgresTransactionType;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
    /// # let (mut db, S { user }) = Drizzle::new(client, S::new());
    /// let count = db.transaction(PostgresTransactionType::ReadCommitted, |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     Ok(users.len())
    /// })?;
    /// # Ok(()) }
    /// ```
    pub fn transaction<F, R>(
        &mut self,
        tx_type: PostgresTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let builder = self.client.build_transaction();
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
        drizzle_core::drizzle_trace_tx!("begin", "postgres.sync");
        let tx = builder.start()?;

        let transaction = Transaction::new(tx, tx_type, self.schema);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&transaction)));

        match result {
            Ok(callback_result) => match callback_result {
                Ok(value) => {
                    drizzle_core::drizzle_trace_tx!("commit", "postgres.sync");
                    transaction.commit()?;
                    Ok(value)
                }
                Err(e) => {
                    drizzle_core::drizzle_trace_tx!("rollback", "postgres.sync");
                    transaction.rollback()?;
                    Err(e)
                }
            },
            Err(panic_payload) => {
                drizzle_core::drizzle_trace_tx!("rollback", "postgres.sync");
                let _ = transaction.rollback();
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
    pub fn create(&mut self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements()?;

        for statement in statements {
            self.client.execute(&statement, &[])?;
        }

        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Apply pending migrations from a MigrationSet.
    ///
    /// Creates the drizzle schema if needed and runs pending migrations in a transaction.
    pub fn migrate(
        &mut self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        if let Some(schema_sql) = migrations.create_schema_sql() {
            self.client.execute(&schema_sql, &[])?;
        }
        self.client.execute(&migrations.create_table_sql(), &[])?;
        let rows = self
            .client
            .query(&migrations.query_all_created_at_sql(), &[])?;
        let applied_created_at: Vec<i64> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();
        let pending: Vec<_> = migrations
            .pending_by_created_at(&applied_created_at)
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        let mut tx = self.client.transaction()?;

        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    tx.execute(stmt, &[])?;
                }
            }
            tx.execute(
                &migrations.record_migration_sql(migration.hash(), migration.created_at()),
                &[],
            )?;
        }

        tx.commit()?;

        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Introspect the connected PostgreSQL database and return a [`Snapshot`](drizzle_migrations::schema::Snapshot).
    ///
    /// Queries the `pg_catalog` and `information_schema` to extract tables, columns,
    /// indexes, foreign keys, primary keys, unique/check constraints, enums, sequences,
    /// views, roles, and policies.
    pub fn introspect(
        &mut self,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        self.introspect_impl(None)
    }

    /// Inner introspection with optional schema filter.
    ///
    /// When `schema_filter` is `Some`, queries that use `pg_get_indexdef()` or
    /// `pg_get_expr()` are scoped to those schemas.  These functions call
    /// `relation_open()` which is not MVCC-protected and can fail when
    /// concurrent DDL drops objects in other schemas.
    fn introspect_impl(
        &mut self,
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

        let err = |msg: &str, e: postgres::Error| -> DrizzleError {
            DrizzleError::Other(format!("{msg}: {e}").into())
        };

        // Schemas
        let schemas: Vec<PgSchema> = self
            .client
            .query(queries::SCHEMAS_QUERY, &[])
            .map_err(|e| err("Failed to query schemas", e))?
            .into_iter()
            .map(|row| PgSchema::new(row.get::<_, String>(0)))
            .collect();

        // Tables
        let raw_tables: Vec<RawTableInfo> = self
            .client
            .query(queries::TABLES_QUERY, &[])
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
                .map_err(|e| err("Failed to query indexes", e))?
        } else {
            self.client
                .query(queries::INDEXES_QUERY, &[])
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
                .map_err(|e| err("Failed to query check constraints", e))?
        } else {
            self.client
                .query(queries::CHECKS_QUERY, &[])
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
    pub fn push<S: drizzle_migrations::Schema>(
        &mut self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        let desired = schema.to_snapshot();
        // Scope introspection to only our schemas. pg_get_indexdef() /
        // pg_get_expr() call relation_open() which is not MVCC-protected
        // and will fail if a concurrent session drops objects.
        let target_schemas: Vec<String> = match &desired {
            drizzle_migrations::schema::Snapshot::Postgres(pg) => pg.schema_names(),
            _ => Vec::new(),
        };
        let live = self.introspect_impl(if target_schemas.is_empty() {
            None
        } else {
            Some(&target_schemas)
        })?;
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
                self.client.execute(&*stmt, &[])?;
            }
        }
        Ok(())
    }
}

impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let param_refs = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.execute.param_refs");
            let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
                SmallVec::with_capacity(params.len());
            param_refs.extend(
                params
                    .iter()
                    .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
            );
            param_refs
        };

        let mut typed_params: SmallVec<
            [(&(dyn postgres::types::ToSql + Sync), postgres::types::Type); 8],
        > = SmallVec::with_capacity(params.len());
        let mut all_typed = true;
        for p in &params {
            if let Some(ty) = crate::builder::postgres::prepared_common::postgres_sync_param_type(p)
            {
                typed_params.push((*p as &(dyn postgres::types::ToSql + Sync), ty));
            } else {
                all_typed = false;
                break;
            }
        }

        if all_typed {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.execute.db_typed");
            let mut rows = self
                .drizzle
                .client
                .query_typed_raw(&sql_str, typed_params)?;
            while rows.next()?.is_some() {}
            return Ok(rows.rows_affected().unwrap_or(0));
        }

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.execute.db");
        Ok(self.drizzle.client.execute(&sql_str, &param_refs[..])?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        self.rows::<R>()?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    pub fn rows<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let rows = self.drizzle.client.query(&sql_str, &param_refs[..])?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let row = self.drizzle.client.query_one(&sql_str, &param_refs[..])?;

        R::try_from(&row).map_err(Into::into)
    }
}
