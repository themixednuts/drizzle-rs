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
//!     .all(db.mut_client(), params![{find_name: "Alice"}])?;
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
    /// Gets a reference to the underlying client
    #[inline]
    pub fn client(&self) -> &Client {
        &self.client
    }

    #[inline]
    pub fn mut_client(&mut self) -> &mut Client {
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
