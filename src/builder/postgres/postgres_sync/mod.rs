//! Synchronous `PostgreSQL` driver using [`postgres`].
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
//!     let (db, AppSchema { user }) = Drizzle::new(client);
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
//! # let (mut db, S { user }) = Drizzle::new(client);
//! use drizzle::postgres::TransactionConfig;
//!
//! let count = db.transaction(TransactionConfig::default(), |tx| {
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
//! # use drizzle::postgres::TransactionConfig;
//! # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
//! # #[derive(PostgresSchema)] struct S { user: User }
//! # fn main() -> drizzle::Result<()> {
//! # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
//! # let (mut db, S { user }) = Drizzle::new(client);
//! db.transaction(TransactionConfig::default(), |tx| {
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
//! # let (mut db, S { user }) = Drizzle::new(client);
//!
//! let find_name = user.name.placeholder("find_name");
//!
//! let find_user = db
//!     .select(())
//!     .from(user)
//!     .r#where(eq(user.name, find_name))
//!     .prepare()
//!     .into_owned();
//!
//! let alice: Vec<SelectUser> = find_user
//!     .all(db.conn_mut(), [find_name.bind("Alice")])?;
//! # Ok(()) }
//! ```

pub(crate) mod prepared;

use core::marker::PhantomData;
use std::cell::{Ref, RefCell, RefMut};
use std::sync::OnceLock;

use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use postgres::{
    Client, IsolationLevel as DriverIsolationLevel, Row, Statement,
    types::{ToSql, Type},
};

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::transaction::{AccessMode, IsolationLevel, TransactionConfig};
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

use crate::builder::postgres::bulk::{self, BulkInsertOptions};
use crate::builder::postgres::common;
use crate::builder::postgres::rows::DecodeRows;
use crate::transaction::savepoint::sync_transaction;

/// A query attached to a [`Drizzle`] handle, ready to run with `.execute()`,
/// `.all()`, `.get()`, or `.rows()`.
pub type DrizzleBuilder<'a, Schema, Builder, State> =
    common::DrizzleBuilder<'a, &'a Drizzle<Schema>, Schema, Builder, State>;

use crate::transaction::postgres::postgres_sync::Transaction;

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Drizzle<Schema> {
    type PreparedDriver = Client;
}

crate::drizzle_prepare_impl!();

/// The blocking PostgreSQL database handle: a [`postgres::Client`] plus the
/// schema's table handles.
///
/// Create it with [`Drizzle::new`], then build queries with `select`,
/// `insert`, `update`, and `delete`. As on the async drivers, query methods
/// take `&self`, so one query can hold another, such as a `count(..)`
/// subquery: the client is borrowed only while a statement runs.
/// [`transaction`](Self::transaction), [`bulk_insert`](Self::bulk_insert),
/// [`migrate`](Self::migrate), and [`conn_mut`](Self::conn_mut) take
/// `&mut self`. Statements run through a per-client statement cache.
pub struct Drizzle<Schema = ()> {
    client: RefCell<Client>,
    schema: Schema,
    client_id: OnceLock<u64>,
    statement_cache: OnceLock<prepared::StatementCache>,
    pooler: bool,
}

/// Rows returned by `.rows()`: fetched up front, decoded as you iterate.
pub type Rows<R> = DecodeRows<Row, R>;

pub(crate) fn postgres_sync_materialize_params<'p>(
    params: &[&'p PostgresValue<'_>],
) -> (SmallVec<[Type; 8]>, SmallVec<[&'p (dyn ToSql + Sync); 8]>) {
    let mut param_types = SmallVec::with_capacity(params.len());
    let mut param_refs = SmallVec::with_capacity(params.len());
    let mut collect_types = true;

    for &param in params {
        param_refs.push(param as &(dyn ToSql + Sync));
        if collect_types {
            if let Some(ty) =
                crate::builder::postgres::prepared_common::postgres_sync_param_type(param)
            {
                param_types.push(ty);
            } else {
                param_types.clear();
                collect_types = false;
            }
        }
    }

    (param_types, param_refs)
}

impl<Schema: Default> Drizzle<Schema> {
    /// Creates a new `Drizzle` instance over `client`.
    ///
    /// Returns `(Drizzle, Schema)`, with the schema built by `Default`. The
    /// pattern that destructures the schema usually names its type. When
    /// nothing else names it, put the type on the call, and use `()` for a
    /// client with no schema:
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # #[PostgresTable] struct Users { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct Schema { users: Users }
    /// # fn connect() -> drizzle::Result<::postgres::Client> {
    /// #     Ok(::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?)
    /// # }
    /// # fn main() -> drizzle::Result<()> {
    /// let (db, Schema { users }) = Drizzle::new(connect()?);
    /// db.insert(users).values([InsertUsers::new("Alice")]).execute()?;
    ///
    /// let (db, schema) = Drizzle::<Schema>::new(connect()?);
    /// db.insert(schema.users).values([InsertUsers::new("Bob")]).execute()?;
    ///
    /// let (db, ()) = Drizzle::new(connect()?);
    /// # let _ = db;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn new(client: Client) -> (Self, Schema) {
        let drizzle = Self {
            client: RefCell::new(client),
            schema: Schema::default(),
            client_id: OnceLock::new(),
            statement_cache: OnceLock::new(),
            pooler: false,
        };
        (drizzle, Schema::default())
    }
}

impl<Schema> Drop for Drizzle<Schema> {
    fn drop(&mut self) {
        if let Some(client_id) = self.client_id.get() {
            prepared::unregister_client(*client_id);
        }
    }
}

impl<S> AsRef<Self> for Drizzle<S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    fn client_id(&self) -> u64 {
        *self.client_id.get_or_init(prepared::next_client_id)
    }

    fn statement_cache(&self) -> prepared::StatementCache {
        self.statement_cache
            .get_or_init(prepared::StatementCache::default)
            .clone()
    }

    /// Declares whether the client's connection goes through a
    /// transaction-mode pooler, such as PgBouncer, Supabase's Supavisor on
    /// port 6543, or a Neon `-pooler` host.
    ///
    /// Such a pooler hands each transaction whichever server connection is
    /// free, so a named statement prepared on one may be missing on the next.
    /// With `pooler` set, queries, transactions, and the prepared statements
    /// run on [`conn_mut`](Self::conn_mut) go out as unnamed statements,
    /// parsed and run in one round trip, and nothing is cached. drizzle-orm
    /// detects poolers from the host and port; this handle wraps a client
    /// that is already connected, so declare it here, right after
    /// [`new`](Self::new):
    ///
    /// ```no_run
    /// # use drizzle::postgres::sync::Drizzle;
    /// # fn main() -> drizzle::Result<()> {
    /// let client = ::postgres::Client::connect(
    ///     "host=localhost port=6432 user=postgres", ::postgres::NoTls,
    /// )?;
    /// let (db, ()) = Drizzle::new(client);
    /// let db = db.pooler(true);
    /// # let _ = db;
    /// # Ok(()) }
    /// ```
    ///
    /// [`migrate`](Self::migrate), [`push`](Self::push), and the
    /// introspection methods still need a direct connection: they hold a
    /// session-level advisory lock or run catalog queries as named statements.
    /// The driver also prepares a named statement of its own the first time a
    /// result carries a type it has not seen, such as an enum.
    #[must_use]
    pub fn pooler(mut self, pooler: bool) -> Self {
        self.pooler = pooler;
        if let Some(client_id) = self.client_id.get() {
            self.statement_cache().clear_client(*client_id);
        }
        self
    }

    /// Registers the client, with its pooler mode, for the statement cache
    /// and the prepared statements run on it.
    /// Borrows the client while a statement runs.
    fn registered_client(&self) -> RefMut<'_, Client> {
        let client = self.client.borrow_mut();
        prepared::register_client(&client, self.client_id(), self.pooler);
        client
    }

    /// Runs the statement and returns the number of rows it changed.
    fn run_execute(
        &self,
        sql: &str,
        params: &[&PostgresValue<'_>],
    ) -> Result<u64, postgres::Error> {
        let statement_cache = self.statement_cache();
        statement_cache.execute(&mut self.registered_client(), sql, params)
    }

    /// Runs the query and returns its rows.
    fn run_query(
        &self,
        sql: &str,
        params: &[&PostgresValue<'_>],
    ) -> Result<Vec<Row>, postgres::Error> {
        let statement_cache = self.statement_cache();
        statement_cache.query(&mut self.registered_client(), sql, params)
    }

    /// Runs the query and returns its row, failing on more than one.
    fn run_query_opt(
        &self,
        sql: &str,
        params: &[&PostgresValue<'_>],
    ) -> Result<Option<Row>, postgres::Error> {
        let statement_cache = self.statement_cache();
        statement_cache.query_opt(&mut self.registered_client(), sql, params)
    }

    /// Borrows the wrapped client, for calls drizzle does not cover.
    ///
    /// The handle's own queries borrow the client while they run, so drop
    /// this borrow before running one through the handle; a query run while
    /// it is held panics.
    #[inline]
    pub fn conn(&self) -> Ref<'_, Client> {
        self.client.borrow()
    }

    /// Returns the wrapped client mutably, for example to run a prepared
    /// statement.
    #[inline]
    pub fn conn_mut(&mut self) -> &mut Client {
        let client_id = self.client_id();
        let client = self.client.get_mut();
        prepared::register_client(client, client_id, self.pooler);
        client
    }

    /// Returns the schema value this handle was created with.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    postgres_builder_constructors!();

    /// Runs any SQL value, such as a raw [`sql!`](crate::sql) fragment, and
    /// returns the number of rows it changed.
    ///
    /// Prefer the builder's own `.execute()`. This method takes anything that
    /// renders to SQL, so it skips the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the statement or the
    /// connection fails. The error carries the SQL and its parameters, as the
    /// builder's do ([`DrizzleError::QueryFailed`](drizzle_core::error::DrizzleError::QueryFailed)).
    pub fn execute<'a, T>(&'a self, query: T) -> drizzle_core::error::Result<u64>
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

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "drizzle.execute.db");
        self.run_execute(&sql, &params)
            .with_query(|| QueryContext::new(&sql, &params))
    }

    /// Runs any SQL value and collects its rows into `C` (for example
    /// `Vec<R>`).
    ///
    /// Each row is decoded with `R: TryFrom<&Row>`, which the generated
    /// `Select*` models and `PostgresFromRow` types implement. Prefer the
    /// builder's own `.all()`: this method skips its compile-time scope and
    /// `NULL` checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded into
    /// `R`.
    pub fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs any SQL value and returns its rows, decoded into `R` as you
    /// iterate.
    ///
    /// Every row is fetched before this returns. Like [`all`](Self::all),
    /// this skips the builder's compile-time checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails. Decoding errors surface per row.
    pub fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
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

        let rows = self
            .run_query(&sql_str, &params)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        Ok(Rows::new(rows))
    }

    /// Runs any SQL value and decodes its single row into `R`.
    ///
    /// Like [`all`](Self::all), this skips the builder's compile-time checks.
    ///
    /// Returns `None` when no row matches.
    ///
    /// # Errors
    ///
    /// Returns an error when the query returns several rows,
    /// when it fails, or when the row cannot be
    /// decoded into `R`.
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Option<R>>
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

        let row = self
            .run_query_opt(&sql_str, &params)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        row.map(|row| R::try_from(&row).map_err(Into::into))
            .transpose()
    }

    /// Starts a relational query on `table` (requires the `query` feature).
    ///
    /// Relations come from foreign keys: `#[column(references = Users::id)]` on
    /// `Posts::author_id` gives `users.author_posts()` (one-to-many) and
    /// `posts.author()` (many-to-one). Results nest the related rows as fields.
    #[cfg(feature = "query")]
    pub fn query<'a, T>(&self, _table: T) -> common::DrizzleQueryBuilder<'_, 'a, &Self, Schema, T>
    where
        T: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            _schema: PhantomData,
        }
    }

    fn start(
        &mut self,
        config: impl Into<TransactionConfig>,
    ) -> drizzle_core::error::Result<Transaction<'_, Schema>>
    where
        Schema: Copy,
    {
        let config = config.into();
        let applied_config = if config.uses_server_default_isolation() {
            let mut applied = TransactionConfig::new();
            if let Some(access) = config.access() {
                applied = applied.access_mode(access);
            }
            applied
        } else {
            config
        };
        let client_id = self.client_id();
        let statement_cache = self.statement_cache();
        let mut builder = self.client.get_mut().build_transaction();

        if let Some(isolation) = applied_config.isolation() {
            builder = builder.isolation_level(match isolation {
                IsolationLevel::ReadUncommitted => DriverIsolationLevel::ReadUncommitted,
                IsolationLevel::ReadCommitted => DriverIsolationLevel::ReadCommitted,
                IsolationLevel::RepeatableRead => DriverIsolationLevel::RepeatableRead,
                IsolationLevel::Serializable => DriverIsolationLevel::Serializable,
            });
        }
        if let Some(access) = applied_config.access() {
            builder = builder.read_only(access == AccessMode::ReadOnly);
        }
        if applied_config.is_deferrable() {
            builder = builder.deferrable(true);
        }

        drizzle_core::drizzle_trace_tx!("begin", "postgres.sync");
        let mut tx = builder.start()?;
        if let Some(snapshot) = applied_config.snapshot() {
            // The id holds only hex digits and `-`, so it is safe to inline.
            tx.batch_execute(&format!("SET TRANSACTION SNAPSHOT '{snapshot}'"))?;
        }
        Ok(Transaction::new(
            tx,
            applied_config,
            self.schema,
            client_id,
            statement_cache,
            self.pooler,
        ))
    }

    /// Runs `f` inside a transaction and returns its value.
    ///
    /// The transaction commits when `f` returns `Ok` and rolls back when it
    /// returns `Err` or panics (the panic then continues). `config` sets the
    /// isolation level, access mode, and `DEFERRABLE`; the default keeps the
    /// server's defaults.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # use drizzle::postgres::TransactionConfig;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
    /// # let (mut db, S { user }) = Drizzle::new(client);
    /// let count = db.transaction(TransactionConfig::default(), |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     Ok(users.len())
    /// })?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the error from `f`, or an error when `BEGIN`, `COMMIT`, or
    /// `ROLLBACK` fails. When the rollback after an `Err` also fails, both
    /// errors are reported together.
    pub fn transaction<F, R>(
        &mut self,
        config: impl Into<TransactionConfig>,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        Schema: Copy,
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let transaction = self.start(config)?;
        sync_transaction(
            transaction,
            "postgres.sync",
            || {
                drizzle_core::drizzle_trace_tx!("commit", "postgres.sync");
            },
            || {
                drizzle_core::drizzle_trace_tx!("rollback", "postgres.sync");
            },
            |transaction| f(transaction),
            |transaction| transaction.commit(),
            |transaction| transaction.rollback(),
        )
    }

    /// Inserts `rows` into `table` in bulk, as multi-row `VALUES` statements,
    /// and returns the number of rows inserted.
    ///
    /// The rows are the table's `Insert*` models, so `default_fn` values are
    /// filled as for [`insert`](Self::insert). A cell a row leaves unset gets
    /// the column's database default (`DEFAULT`), and a column no row sets is
    /// left out of the statement. The rows are split into statements of at
    /// most 65535 parameters, PostgreSQL's limit; by default those run in one
    /// transaction. [`BulkInsertOptions::atomic`] set to `false` commits each
    /// statement on its own instead.
    ///
    /// # Errors
    ///
    /// Returns the first statement's error. An atomic insert then inserted
    /// nothing; one that is not keeps the statements before the failing one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # use drizzle::postgres::BulkInsertOptions;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
    /// # let (mut db, S { user }) = Drizzle::new(client);
    /// let rows = (0..100_000).map(|i| InsertUser::new(format!("user {i}")));
    /// let inserted = db.bulk_insert(user, rows, BulkInsertOptions::default())?;
    /// # Ok(()) }
    /// ```
    pub fn bulk_insert<'b, Table, T>(
        &mut self,
        table: Table,
        rows: impl IntoIterator<Item = Table::Insert<T>>,
        options: BulkInsertOptions,
    ) -> drizzle_core::error::Result<u64>
    where
        Schema: Copy,
        Table: PostgresTable<'b> + Copy,
    {
        let chunks = bulk::insert_chunks::<Table, T>(rows);
        if options.atomic && chunks.len() > 1 {
            return self.transaction(TransactionConfig::default(), |tx| {
                let mut inserted = 0;
                for chunk in chunks {
                    inserted += tx.insert(table).values(chunk).execute()?;
                }
                Ok(inserted)
            });
        }
        let mut inserted = 0;
        for chunk in chunks {
            inserted += self.insert(table).values(chunk).execute()?;
        }
        Ok(inserted)
    }
}

impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Creates every table, index, and view in the schema.
    ///
    /// Useful for tests and throwaway databases; use `migrate` to evolve a
    /// real one.
    ///
    /// The statements run one by one, outside a transaction.
    ///
    /// # Errors
    ///
    /// Returns an error when the database rejects one of the statements;
    /// earlier statements stay applied.
    pub fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements()?;

        let mut client = self.client.borrow_mut();
        for statement in statements {
            client.batch_execute(&statement)?;
        }

        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Applies the migrations that have not run yet.
    ///
    /// Creates the tracking table (and its schema) if needed, then runs each
    /// pending migration in its own transaction together with its tracking
    /// row. When one fails, it rolls back and the migrations before it stay
    /// applied. Migrations that already ran are skipped. An advisory lock
    /// (`pg_advisory_lock`) keeps concurrent `migrate` calls from overlapping.
    ///
    /// This differs from drizzle-orm, whose PostgreSQL `migrate` (like
    /// `drizzle migrate`) runs all pending migrations in one transaction, so
    /// a failure there leaves none of them applied.
    ///
    /// Each `--> statement-breakpoint` chunk of a migration file runs whole,
    /// through the simple query protocol, as drizzle-orm runs it.
    ///
    /// Load the migrations with [`include_migrations!`](crate::include_migrations)
    /// or [`MigrationDir`](drizzle_migrations::MigrationDir).
    ///
    /// # Two-phase tracking on the CONCURRENTLY path
    ///
    /// A migration containing `CREATE/DROP INDEX CONCURRENTLY` cannot run
    /// inside a transaction, so it executes autocommit statement-by-statement.
    /// That path uses two-phase tracking: the tracking row is inserted with
    /// `applied_at` NULL *before* the statements run (marking the migration
    /// **dirty**) and updated to set `applied_at` after they succeed. A crash
    /// in between leaves a dirty row instead of an untracked partial schema,
    /// and the next `migrate()` says which migration was interrupted rather
    /// than re-running its DDL. See [`Self::migrate_with_repair`].
    ///
    /// Non-concurrent migrations keep the single-transaction flow (statements
    /// and tracking insert commit together), which is already atomic.
    ///
    /// # Errors
    ///
    /// Returns an error when a migration statement fails, or when the
    /// tracking table holds an unfinished ("dirty") row left by an interrupted
    /// run. Use [`migrate_with_repair`](Self::migrate_with_repair) for that
    /// case.
    pub fn migrate(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        self.migrate_inner(migrations, tracking, false)
    }

    /// Finishes any interrupted migration, then applies pending ones like
    /// [`migrate`](Self::migrate).
    ///
    /// For each migration marked dirty by the two-phase CONCURRENTLY flow in
    /// [`Self::migrate`], this introspects the live catalogs and classifies
    /// every statement: `CREATE TABLE`, `CREATE [UNIQUE] INDEX [CONCURRENTLY]`,
    /// `CREATE VIEW` and `CREATE TYPE ... AS ENUM` statements whose object
    /// already exists with a matching definition are skipped, and the rest are
    /// executed. Anything that cannot be proven either way aborts with a list
    /// of what needs manual resolution.
    ///
    /// # Errors
    ///
    /// Returns an error listing what needs manual attention when a statement
    /// cannot be proven applied or not applied, and for the same failures as
    /// [`migrate`](Self::migrate).
    pub fn migrate_with_repair(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        self.migrate_inner(migrations, tracking, true)
    }

    fn migrate_inner(
        &mut self,
        migrations: &[drizzle_migrations::Migration],
        tracking: drizzle_migrations::Tracking,
        repair: bool,
    ) -> drizzle_core::error::Result<drizzle_migrations::MigrateOutcome> {
        // Migrations change tables, and PostgreSQL rejects a cached statement
        // whose result columns changed; drop the connection's statements.
        self.statement_cache().clear_client(self.client_id());
        let set = drizzle_migrations::Migrations::with_tracking(
            migrations.to_vec(),
            drizzle_types::Dialect::PostgreSQL,
            tracking,
        );

        // Take the lock before creating anything: concurrent first runs
        // otherwise race on `CREATE SCHEMA/TABLE IF NOT EXISTS`, which is not
        // safe against concurrent creation in PostgreSQL.
        let lock_key = set.postgres_advisory_lock_key();
        let client = self.client.get_mut();
        client.query_one("SELECT pg_advisory_lock($1)", &[&lock_key])?;

        let result = (|| {
            if let Some(schema_sql) = set.create_schema_sql() {
                client.execute(&schema_sql, &[])?;
            }
            ensure_postgres_migration_table(client, &set)?;
            let mut applied = repair_dirty_migrations(client, &set, repair)?;

            let rows = client.query(&set.applied_names_sql(), &[])?;
            let applied_names = rows
                .iter()
                .map(|row| row.try_get::<_, String>(0))
                .collect::<Result<Vec<_>, postgres::Error>>()?;
            let pending: Vec<_> = set.pending(&applied_names).collect();

            if pending.is_empty() {
                if applied.is_empty() {
                    return Ok(drizzle_migrations::MigrateOutcome::UpToDate);
                }
                return Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied });
            }

            for migration in &pending {
                if migration.has_postgres_concurrent_index() {
                    // CREATE/DROP INDEX CONCURRENTLY cannot run inside a
                    // transaction; only this migration runs autocommit, so it
                    // needs the dirty marker to stay recoverable.
                    client.execute(&set.record_migration_started_sql(migration), &[])?;

                    let mut executed = 0usize;
                    for statement in migration.statements() {
                        if statement.trim().is_empty() {
                            continue;
                        }
                        if let Err(error) = client.batch_execute(statement) {
                            // Nothing can have been applied yet, so drop the
                            // marker rather than demand a pointless repair.
                            if executed == 0 {
                                let _ = client
                                    .execute(&set.clear_migration_started_sql(migration), &[]);
                            }
                            return Err(migration_statement_err(migration, statement, &error));
                        }
                        executed += 1;
                    }

                    client.execute(&set.record_migration_finished_sql(migration), &[])?;
                } else {
                    let mut tx = client.transaction()?;
                    for statement in migration.statements() {
                        if !statement.trim().is_empty() {
                            // Simple-query protocol: a breakpoint chunk runs
                            // whole, even when it holds several statements.
                            tx.batch_execute(statement).map_err(|error| {
                                migration_statement_err(migration, statement, &error)
                            })?;
                        }
                    }
                    tx.execute(&set.record_migration_sql(migration), &[])?;
                    tx.commit()?;
                }
                applied.push(migration.tag().to_string());
            }

            Ok(drizzle_migrations::MigrateOutcome::Applied { tags: applied })
        })();

        let unlock_result = client
            .query_one("SELECT pg_advisory_unlock($1)", &[&lock_key])
            .map(|_| ());

        match (result, unlock_result) {
            (Ok(outcome), Ok(())) => Ok(outcome),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
        }
    }
}

fn ensure_postgres_migration_table(
    client: &mut postgres::Client,
    set: &drizzle_migrations::Migrations,
) -> drizzle_core::error::Result<()> {
    client.execute(&set.create_table_sql(), &[])?;

    let schema = set.schema_name().unwrap_or("public");
    let rows = client.query(
        "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2",
        &[&schema, &set.table_name()],
    )?;
    let column_names = rows
        .iter()
        .map(|row| row.try_get::<_, String>(0))
        .collect::<Result<Vec<_>, postgres::Error>>()?;
    // Check each column: an earlier, non-transactional upgrade may have
    // added `name` and died before adding `applied_at`.
    let has_name = column_names.iter().any(|column| column == "name");
    let has_applied_at = column_names.iter().any(|column| column == "applied_at");
    if has_name && has_applied_at {
        return Ok(());
    }

    let rows = client.query(
        &format!(
            "SELECT id::bigint, hash, created_at FROM {} ORDER BY id ASC",
            set.table_ident_sql()
        ),
        &[],
    )?;
    let applied = rows
        .iter()
        .map(|row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row.try_get::<_, Option<i64>>(0).ok().flatten(),
                hash: row.try_get::<_, String>(1)?,
                created_at: row.try_get::<_, i64>(2)?,
            })
        })
        .collect::<Result<Vec<_>, postgres::Error>>()?;

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string().into()))?;

    // One simple-query batch runs as a single implicit transaction, so the
    // column additions and backfills land together or not at all.
    let upgrade = set
        .tracking_upgrade_sql(has_name, has_applied_at, &matched)
        .join(";\n");
    client
        .batch_execute(&upgrade)
        .map_err(|e| pg_sync_err("failed to upgrade the migrations tracking table", &e))?;

    Ok(())
}

/// Read the live PostgreSQL catalogs into a repair [`Catalog`](drizzle_migrations::repair::Catalog).
fn introspect_catalog(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<drizzle_migrations::repair::Catalog> {
    use drizzle_migrations::repair::postgres as pg;

    let two = |client: &mut postgres::Client,
               sql: &str|
     -> Result<Vec<(String, String)>, postgres::Error> {
        Ok(client
            .query(sql, &[])?
            .iter()
            .map(|row| (row.get::<_, String>(0), row.get::<_, String>(1)))
            .collect())
    };
    let three = |client: &mut postgres::Client,
                 sql: &str|
     -> Result<Vec<(String, String, String)>, postgres::Error> {
        Ok(client
            .query(sql, &[])?
            .iter()
            .map(|row| {
                (
                    row.get::<_, String>(0),
                    row.get::<_, String>(1),
                    row.get::<_, String>(2),
                )
            })
            .collect())
    };

    let indexes = client
        .query(pg::INDEXES_QUERY, &[])?
        .iter()
        .map(|row| {
            (
                row.get::<_, String>(0),
                row.get::<_, String>(1),
                row.get::<_, bool>(2),
            )
        })
        .collect::<Vec<_>>();

    let tables = two(client, pg::TABLES_QUERY)?;
    let columns = three(client, pg::COLUMNS_QUERY)?;
    let views = two(client, pg::VIEWS_QUERY)?;
    let index_columns = three(client, pg::INDEX_COLUMNS_QUERY)?;
    let enums = three(client, pg::ENUMS_QUERY)?;

    Ok(pg::catalog(
        &tables,
        &columns,
        &views,
        &indexes,
        &index_columns,
        &enums,
    ))
}

/// Reject or reconcile interrupted migrations. Returns the repaired tags.
fn repair_dirty_migrations(
    client: &mut postgres::Client,
    set: &drizzle_migrations::Migrations,
    repair: bool,
) -> drizzle_core::error::Result<Vec<String>> {
    let dirty = client
        .query(&set.dirty_names_sql(), &[])?
        .iter()
        .map(|row| row.try_get::<_, String>(0))
        .collect::<Result<Vec<_>, postgres::Error>>()?;

    if dirty.is_empty() {
        return Ok(Vec::new());
    }

    let migrator_error =
        |error: drizzle_migrations::MigratorError| DrizzleError::Other(error.to_string().into());

    if !repair {
        return Err(migrator_error(
            set.interrupted_migration_error(&dirty)
                .expect("dirty list is non-empty"),
        ));
    }

    let table_ident = set.table_ident_sql();
    let dirty_migrations = set
        .resolve_dirty_migrations(&dirty)
        .map_err(migrator_error)?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    let mut repaired = Vec::new();
    for migration in &dirty_migrations {
        let catalog = introspect_catalog(client)?;
        let plan = drizzle_migrations::repair::plan(
            drizzle_types::Dialect::PostgreSQL,
            migration,
            &catalog,
        );
        for statement in plan.into_executable(&table_ident).map_err(migrator_error)? {
            client.execute(statement.as_str(), &[])?;
        }
        client.execute(&set.record_migration_finished_sql(migration), &[])?;
        repaired.push(migration.tag().to_string());
    }

    Ok(repaired)
}

/// Error for a failed migration statement, carrying the server's message
/// (the bare `postgres::Error` only displays `db error`).
fn migration_statement_err(
    migration: &drizzle_migrations::Migration,
    statement: &str,
    error: &postgres::Error,
) -> DrizzleError {
    pg_sync_err(
        &format!(
            "migration `{}` failed\n  statement: {}\n  error",
            migration.tag(),
            statement.trim()
        ),
        error,
    )
}

fn pg_sync_err(msg: &str, e: &postgres::Error) -> DrizzleError {
    // `postgres::Error`'s Display is just "db error"; the server's actual
    // message lives in the DbError source. Its DETAIL line is left out: for
    // a constraint violation it holds the row's values.
    match e.as_db_error() {
        Some(db) => {
            DrizzleError::Other(format!("{msg}: {} ({})", db.message(), db.code().code()).into())
        }
        None => DrizzleError::Other(format!("{msg}: {e}").into()),
    }
}

fn pg_sync_query_schemas(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::ddl::Schema>> {
    use drizzle_migrations::postgres::ddl::Schema as PgSchema;
    use drizzle_migrations::postgres::introspect::queries;

    Ok(client
        .query(queries::SCHEMAS_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query schemas", &e))?
        .into_iter()
        .map(|row| PgSchema::new(row.get::<_, String>(0)))
        .collect())
}

fn pg_sync_query_tables(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawTableInfo>> {
    use drizzle_migrations::postgres::introspect::{RawTableInfo, queries};

    Ok(client
        .query(queries::TABLES_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query tables", &e))?
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get(0),
            name: row.get(1),
            is_rls_enabled: row.get(2),
            is_unlogged: row.get(3),
            is_temporary: row.get(4),
            tablespace: row.get(5),
            comment: row.get(6),
        })
        .collect())
}

fn pg_sync_query_columns(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawColumnInfo>> {
    use drizzle_migrations::postgres::introspect::{RawColumnInfo, queries};

    Ok(client
        .query(queries::COLUMNS_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query columns", &e))?
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
            generated_stored: row.get(11),
            dimensions: row.get(12),
            comment: row.get(13),
            ordinal_position: row.get(14),
        })
        .collect())
}

fn pg_sync_query_enums(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawEnumInfo>> {
    use drizzle_migrations::postgres::introspect::{RawEnumInfo, queries};

    Ok(client
        .query(queries::ENUMS_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query enums", &e))?
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get(0),
            name: row.get(1),
            values: row.get(2),
        })
        .collect())
}

fn pg_sync_query_sequences(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawSequenceInfo>> {
    use drizzle_migrations::postgres::introspect::{RawSequenceInfo, queries};

    Ok(client
        .query(queries::SEQUENCES_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query sequences", &e))?
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
            owned_by: row.get(9),
        })
        .collect())
}

fn pg_sync_query_views(
    client: &mut postgres::Client,
    schema_filter: Option<&[String]>,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawViewInfo>> {
    use drizzle_migrations::postgres::introspect::{RawViewInfo, queries};

    Ok(client
        .query(queries::VIEWS_QUERY, &[&schema_filter])
        .map_err(|e| pg_sync_err("Failed to query views", &e))?
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get(0),
            name: row.get(1),
            definition: row.get(2),
            is_materialized: row.get(3),
        })
        .collect())
}

fn pg_sync_query_indexes(
    client: &mut postgres::Client,
    schema_filter: Option<&[String]>,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawIndexInfo>> {
    use drizzle_migrations::postgres::introspect::{RawIndexInfo, parse_index_columns, queries};

    let rows = if let Some(schemas) = schema_filter {
        client
            .query(queries::INDEXES_QUERY_FILTERED, &[&schemas])
            .map_err(|e| pg_sync_err("Failed to query indexes", &e))?
    } else {
        client
            .query(queries::INDEXES_QUERY, &[])
            .map_err(|e| pg_sync_err("Failed to query indexes", &e))?
    };
    Ok(rows
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
        .collect())
}

fn pg_sync_query_foreign_keys(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawForeignKeyInfo>> {
    use drizzle_migrations::postgres::introspect::{
        RawForeignKeyInfo, action_code_to_string, queries,
    };

    Ok(client
        .query(queries::FOREIGN_KEYS_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query foreign keys", &e))?
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
            schema_to: row.get(4),
            table_to: row.get(5),
            columns_to: row.get(6),
            on_update: action_code_to_string(&row.get::<_, String>(7)),
            on_delete: action_code_to_string(&row.get::<_, String>(8)),
            deferrable: row.get(9),
            initially_deferred: row.get(10),
        })
        .collect())
}

fn pg_sync_query_primary_keys(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawPrimaryKeyInfo>> {
    use drizzle_migrations::postgres::introspect::{RawPrimaryKeyInfo, queries};

    Ok(client
        .query(queries::PRIMARY_KEYS_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query primary keys", &e))?
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
        })
        .collect())
}

fn pg_sync_query_uniques(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawUniqueInfo>> {
    use drizzle_migrations::postgres::introspect::{RawUniqueInfo, queries};

    Ok(client
        .query(queries::UNIQUES_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query unique constraints", &e))?
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
            nulls_not_distinct: row.get(4),
            deferrable: row.get(5),
            initially_deferred: row.get(6),
        })
        .collect())
}

fn pg_sync_query_checks(
    client: &mut postgres::Client,
    schema_filter: Option<&[String]>,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawCheckInfo>> {
    use drizzle_migrations::postgres::introspect::{RawCheckInfo, queries};

    let rows = if let Some(schemas) = schema_filter {
        client
            .query(queries::CHECKS_QUERY_FILTERED, &[&schemas])
            .map_err(|e| pg_sync_err("Failed to query check constraints", &e))?
    } else {
        client
            .query(queries::CHECKS_QUERY, &[])
            .map_err(|e| pg_sync_err("Failed to query check constraints", &e))?
    };
    Ok(rows
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            expression: row.get(3),
        })
        .collect())
}

fn pg_sync_query_roles(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawRoleInfo>> {
    use drizzle_migrations::postgres::introspect::{RawRoleInfo, queries};

    Ok(client
        .query(queries::ROLES_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query roles", &e))?
        .into_iter()
        .map(|row| RawRoleInfo {
            name: row.get(0),
            create_db: row.get(1),
            create_role: row.get(2),
            inherit: row.get(3),
        })
        .collect())
}

fn pg_sync_query_policies(
    client: &mut postgres::Client,
) -> drizzle_core::error::Result<Vec<drizzle_migrations::postgres::introspect::RawPolicyInfo>> {
    use drizzle_migrations::postgres::introspect::{RawPolicyInfo, queries};

    Ok(client
        .query(queries::POLICIES_QUERY, &[])
        .map_err(|e| pg_sync_err("Failed to query policies", &e))?
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
        .collect())
}

impl<Schema> Drizzle<Schema> {
    /// Reads the live database's schema into a
    /// [`Snapshot`](drizzle_migrations::schema::Snapshot).
    ///
    /// Queries `pg_catalog` and `information_schema` for tables, columns,
    /// indexes, foreign keys, primary keys, unique and check constraints,
    /// enums, sequences, views, roles, and policies, and returns them as
    /// `Snapshot::Postgres(..)`.
    ///
    /// # Errors
    ///
    /// Returns an error when one of the catalog queries fails.
    pub fn introspect(&self) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        self.introspect_impl(None)
    }

    /// Like [`introspect`](Self::introspect), but reads only the named
    /// schemas (and the roles and privileges, which belong to none).
    ///
    /// Prefer it when other sessions may be changing other schemas at the
    /// same time: some catalog functions PostgreSQL offers (such as
    /// `pg_get_indexdef`) are not MVCC-safe and fail when a relation they
    /// reach is dropped concurrently, which this keeps out of reach.
    ///
    /// # Errors
    ///
    /// Returns an error when one of the catalog queries fails.
    pub fn introspect_schemas<S: AsRef<str>>(
        &self,
        schemas: &[S],
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        let names: Vec<String> = schemas
            .iter()
            .map(|name| name.as_ref().to_owned())
            .collect();
        Ok(match self.introspect_impl(Some(&names))? {
            drizzle_migrations::schema::Snapshot::Postgres(snapshot) => {
                drizzle_migrations::schema::Snapshot::Postgres(snapshot.scoped_to_schemas(&names))
            }
            other => other,
        })
    }

    /// Inner introspection with optional schema filter.
    ///
    /// When `schema_filter` is `Some`, queries that use `pg_get_indexdef()` or
    /// `pg_get_expr()` are scoped to those schemas.  These functions call
    /// `relation_open()` which is not MVCC-protected and can fail when
    /// concurrent DDL drops objects in other schemas.
    fn introspect_impl(
        &self,
        schema_filter: Option<&[String]>,
    ) -> drizzle_core::error::Result<drizzle_migrations::schema::Snapshot> {
        use drizzle_migrations::postgres::ddl::Schema as PgSchema;
        use drizzle_migrations::postgres::introspect::{RawIntrospection, assemble_ddl};

        let mut client = self.client.borrow_mut();
        let schemas: Vec<PgSchema> = pg_sync_query_schemas(&mut client)?;
        let accessible_schema_names = schemas
            .iter()
            .map(|schema| schema.name().to_string())
            .collect::<Vec<_>>();
        let effective_schema_filter = schema_filter.or(Some(accessible_schema_names.as_slice()));
        let raw_tables = pg_sync_query_tables(&mut client)?;
        let raw_columns = pg_sync_query_columns(&mut client)?;
        let raw_enums = pg_sync_query_enums(&mut client)?;
        let raw_sequences = pg_sync_query_sequences(&mut client)?;
        let raw_views = pg_sync_query_views(&mut client, effective_schema_filter)?;
        let raw_indexes = pg_sync_query_indexes(&mut client, effective_schema_filter)?;
        let raw_fks = pg_sync_query_foreign_keys(&mut client)?;
        let raw_primary_keys = pg_sync_query_primary_keys(&mut client)?;
        let raw_uniques = pg_sync_query_uniques(&mut client)?;
        let raw_checks = pg_sync_query_checks(&mut client, effective_schema_filter)?;
        let raw_roles = pg_sync_query_roles(&mut client)?;
        let raw_policies = pg_sync_query_policies(&mut client)?;

        let ddl = assemble_ddl(RawIntrospection {
            schemas,
            tables: raw_tables,
            columns: raw_columns,
            enums: raw_enums,
            sequences: raw_sequences,
            views: raw_views,
            indexes: raw_indexes,
            foreign_keys: raw_fks,
            primary_keys: raw_primary_keys,
            unique_constraints: raw_uniques,
            check_constraints: raw_checks,
            roles: raw_roles,
            policies: raw_policies,
        });

        // Build snapshot
        let mut snap = drizzle_migrations::postgres::PostgresSnapshot::new();
        for entity in ddl.to_entities() {
            snap.add_entity(entity);
        }

        Ok(drizzle_migrations::schema::Snapshot::Postgres(snap))
    }

    /// Changes the live database to match `schema`, without migration files.
    ///
    /// Introspects the database, diffs it against `schema`, and runs the
    /// resulting statements. Does nothing when they already match. Meant for
    /// local development: nothing is recorded in the migration tracking
    /// table.
    ///
    /// When a schema, enum, table or column may have been renamed, `push`
    /// fails rather than guess, and the error gives the hint for each answer;
    /// pass the answers to [`push_with`](Self::push_with).
    ///
    /// A push that would drop a table or column holding rows fails and
    /// applies nothing, the way drizzle-kit's push stops to ask; dropping an
    /// empty one goes ahead.
    ///
    /// # Errors
    ///
    /// Returns an error when introspection or diffing fails, or when a
    /// statement fails. The statements do not run in a transaction, so the
    /// ones before a failure stay applied.
    pub fn push<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
    ) -> drizzle_core::error::Result<()> {
        self.push_with(schema, &drizzle_migrations::RenameHints::new())
    }

    /// [`push`](Self::push), with answers to its rename-or-create questions.
    ///
    /// # Errors
    ///
    /// Same as [`push`](Self::push).
    pub fn push_with<S: drizzle_migrations::Schema>(
        &self,
        schema: &S,
        renames: &drizzle_migrations::RenameHints,
    ) -> drizzle_core::error::Result<()> {
        let desired = schema.to_snapshot();
        // Scope introspection to only our schemas. pg_get_indexdef() /
        // pg_get_expr() call relation_open() which is not MVCC-protected
        // and will fail if a concurrent session drops objects.
        let target_schemas: Vec<String> = match &desired {
            drizzle_migrations::schema::Snapshot::Postgres(pg) => pg.schema_names(),
            drizzle_migrations::schema::Snapshot::Sqlite(_)
            | drizzle_migrations::schema::Snapshot::MySQL(_) => Vec::new(),
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
        let generated = drizzle_migrations::diff_with(
            &live,
            &desired,
            &drizzle_migrations::DiffOptions::new().with_renames(renames.clone()),
        )
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        let mut lost = Vec::with_capacity(generated.data_loss.len());
        for drop in &generated.data_loss {
            let sql = drop.count_sql(drizzle_types::Dialect::PostgreSQL);
            let rows: i64 = self.client.borrow_mut().query_one(&*sql, &[])?.get(0);
            lost.push((drop.clone(), rows));
        }
        crate::builder::refuse_data_loss(&lost)?;
        // The push changes tables, and PostgreSQL rejects a cached statement
        // whose result columns changed; drop the connection's statements.
        self.statement_cache().clear_client(self.client_id());
        let mut client = self.client.borrow_mut();
        for stmt in generated.statements {
            if !stmt.trim().is_empty() {
                client.execute(&*stmt, &[])?;
            }
        }
        Ok(())
    }
}

impl<S, Schema, State, Table, Mk, Rw, Grouped>
    DrizzleBuilder<'_, S, QueryBuilder<'_, Schema, State, Table, Mk, Rw, Grouped>, State>
{
    /// Runs the statement and returns the number of rows it changed.
    ///
    /// Use it for `INSERT`, `UPDATE`, and `DELETE`. With a `RETURNING`
    /// clause, the returned rows are counted, not decoded (use
    /// [`all`](Self::all) to read them).
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the statement, for example on
    /// a constraint violation. The error carries the SQL and its parameters
    /// ([`DrizzleError::QueryFailed`](drizzle_core::error::DrizzleError::QueryFailed)).
    pub fn execute(self) -> drizzle_core::error::Result<u64>
    where
        State: builder::ExecutableState,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.execute.db");
        self.runner
            .run_execute(&sql_str, &params)
            .with_query(|| QueryContext::new(&sql_str, &params))
    }

    /// Runs the query and decodes every row into `R`.
    ///
    /// `R` is usually the generated `Select*` model (for `select(())`), a
    /// tuple matching the selected columns, or a type deriving
    /// `PostgresFromRow`.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or a row cannot be decoded into
    /// `R`. Query errors carry the SQL and its parameters
    /// ([`DrizzleError::QueryFailed`](drizzle_core::error::DrizzleError::QueryFailed)).
    ///
    /// # Compile-time checks
    ///
    /// The call does not compile unless:
    ///
    /// - every column the query reads (in `SELECT`, `WHERE`, `ORDER BY`, ...)
    ///   belongs to a table in its `FROM`/`JOIN` list;
    /// - `R` matches the selection: one field per selected column, with a
    ///   compatible type, and `Option<T>` wherever the value can be `NULL`
    ///   (a nullable column, or any column of an outer-joined table);
    /// - with `GROUP BY`, each column in a selected tuple is grouped or
    ///   aggregated;
    /// - a raw `sql!` selection carries an explicit result type.
    pub fn all<R, Proof, AggProof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        State: builder::ExecutableState,
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let rows = self
            .runner
            .run_query(&sql_str, &params)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns its rows, decoded into the row type the
    /// query infers from its selection.
    ///
    /// Unlike [`all`](Self::all), you do not pick the row type: `select(())`
    /// yields the table's `Select*` model and a column tuple yields a tuple.
    /// Every row is fetched before this returns; each is decoded as you
    /// iterate.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails. Decoding errors surface per row.
    ///
    /// # Compile-time checks
    ///
    /// The same scope and grouping checks as [`all`](Self::all).
    pub fn rows<Proof, AggProof>(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        State: builder::ExecutableState,
        for<'r> Mk: drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, Rw, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.rows");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let rows = self
            .runner
            .run_query(&sql_str, &params)
            .with_query(|| QueryContext::new(&sql_str, &params))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and decodes its single row into `R`.
    ///
    /// The query must not return more than one row; add `.limit(1)` if it
    /// could.
    ///
    /// Returns `None` when no row matches. A query that always returns one
    /// row (only aggregates, with no GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET
    /// or set operation) returns the row itself, and fails with
    /// [`DrizzleError::NotFound`](drizzle_core::error::DrizzleError::NotFound)
    /// if the database returns none.
    ///
    /// # Errors
    ///
    /// Returns an error when the query returns more than one row,
    /// when it fails, or when the row
    /// cannot be decoded into `R`.
    ///
    /// # Compile-time checks
    ///
    /// The same scope, `NULL`, and grouping checks as [`all`](Self::all).
    pub fn get<R, Proof, AggProof>(
        self,
    ) -> drizzle_core::error::Result<drizzle_core::row::GetRow<Mk, Grouped, State, R>>
    where
        Mk: drizzle_core::row::QueryRows<Grouped, State>,
        State: builder::ExecutableState,
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R, Proof>,
        Mk: drizzle_core::row::MarkerAggValidFor<Grouped, AggProof>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let row = self
            .runner
            .run_query_opt(&sql_str, &params)
            .with_query(|| QueryContext::new(&sql_str, &params))?;
        let row = row
            .map(|row| {
                <Mk as drizzle_core::row::DecodeSelectedRef<&::postgres::Row, R>>::decode(&row)
            })
            .transpose()?;
        drizzle_core::row::first_row::<Mk, Grouped, State, R>(row)
    }
}

// =============================================================================
// Relational Query API
// =============================================================================

#[cfg(feature = "query")]
use drizzle_core::query::DeserializeStore;
#[cfg(feature = "query")]
use drizzle_core::query::FromJsonObject as _;

// AllColumns: read base from individual row columns via TryFrom<Row>
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let num_base_cols = T::COLUMN_NAMES.len();

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
            false,
        );
        let (sql, bind_params) = query_sql.build();

        drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

        let rows = self
            .runner
            .run_query(&sql, &bind_params)
            .with_query(|| QueryContext::new(&sql, &bind_params))?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
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
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

// PartialColumns: read base from a single JSON "__base" column via FromJsonObject
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
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let builder = self.builder;
        let column_names = &builder.cols.columns;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = column_names.clone();
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

        drizzle_core::drizzle_trace_query!(&sql, bind_params.len());

        let rows = self
            .runner
            .run_query(&sql, &bind_params)
            .with_query(|| QueryContext::new(&sql, &bind_params))?;
        let mut results = Vec::with_capacity(rows.len());

        for row in &rows {
            // Column 0 is the JSON "__base" object
            let base_json: String = row.get(0);
            let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
                &base_json, "base",
            )?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
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
            + drizzle_core::query::RenderRelations<'a, PostgresValue<'a>>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Client, T, Rels, drizzle_core::query::AllColumns>
{
    /// Runs the prepared relational query with `params` bound and returns
    /// every root row with its loaded relations.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub fn find_many<const N: usize>(
        &self,
        client: &mut Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Vec<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::Select,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        let num_base_cols = T::COLUMN_NAMES.len();
        let (sql_str, bound_params) = self.inner.bind(params)?;
        let (lower, upper) = bound_params.size_hint();
        let mut params_vec: SmallVec<[PostgresValue<'a>; 8]> =
            SmallVec::with_capacity(upper.unwrap_or(lower));
        params_vec.extend(bound_params);
        let param_refs: SmallVec<[&PostgresValue<'_>; 8]> = params_vec.iter().collect();
        let rows = prepared::query_uncached(client, sql_str, &param_refs)?;
        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
            let base = <T as drizzle_core::query::QueryTable>::Select::try_from(&row)
                .map_err(Into::into)?;

            let mut rel_col = num_base_cols;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
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
    pub fn find_first<const N: usize>(
        &self,
        client: &mut Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
    ) -> drizzle_core::error::Result<
        Option<
            <Rels as drizzle_core::query::BuildRow<
                <T as drizzle_core::query::QueryTable>::Select,
            >>::Row,
        >,
    >
    where
        T: drizzle_core::query::QueryTable,
        <T as drizzle_core::query::QueryTable>::Select: for<'r> TryFrom<&'r Row>,
        for<'r> <<T as drizzle_core::query::QueryTable>::Select as TryFrom<&'r Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        Rels: drizzle_core::query::BuildRow<<T as drizzle_core::query::QueryTable>::Select>,
        <Rels as drizzle_core::query::BuildStore>::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.find_many(client, params)?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'a, T, Rels>
    common::DrizzlePreparedQuery<'a, Client, T, Rels, drizzle_core::query::PartialColumns>
{
    /// Runs the prepared relational query with `params` bound and returns
    /// every root row with its loaded relations.
    ///
    /// # Errors
    ///
    /// Returns an error when a placeholder is missing or unknown, when the
    /// query fails, or when a row cannot be decoded.
    pub fn find_many<const N: usize>(
        &self,
        client: &mut Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
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
        let (sql_str, bound_params) = self.inner.bind(params)?;
        let (lower, upper) = bound_params.size_hint();
        let mut params_vec: SmallVec<[PostgresValue<'a>; 8]> =
            SmallVec::with_capacity(upper.unwrap_or(lower));
        params_vec.extend(bound_params);
        let param_refs: SmallVec<[&PostgresValue<'_>; 8]> = params_vec.iter().collect();
        let rows = prepared::query_uncached(client, sql_str, &param_refs)?;
        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
            let base_json: String = row.get(0);
            let base = <T as drizzle_core::query::QueryTable>::PartialSelect::from_json_str(
                &base_json, "base",
            )?;

            let mut rel_col = 1usize;
            let mut next_rel = || {
                let json: Option<String> = row.get(rel_col);
                rel_col += 1;
                Ok(json)
            };
            let store =
                <Rels as drizzle_core::query::BuildStore>::Store::from_json_columns(&mut next_rel)?;

            results.push(<Rels as drizzle_core::query::BuildRow<_>>::assemble(
                base, store,
            ));
        }

        Ok(results)
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
    pub fn find_first<const N: usize>(
        &self,
        client: &mut Client,
        params: [drizzle_core::param::ParamBind<'a, PostgresValue<'a>>; N],
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
        Ok(self.find_many(client, params)?.into_iter().next())
    }
}
