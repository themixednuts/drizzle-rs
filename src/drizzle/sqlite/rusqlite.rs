mod delete;
mod insert;
mod prepared;
mod select;
mod update;

use drizzle_core::ToSQL;
use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_sqlite::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_sqlite::traits::SQLiteTable;
use rusqlite::{Connection, params_from_iter};
use std::marker::PhantomData;

use drizzle_sqlite::{
    SQLiteTransactionType, SQLiteValue,
    builder::{
        self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
        update::UpdateBuilder,
    },
};

/// Rusqlite-specific drizzle builder
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}
use crate::transaction::sqlite::rusqlite::Transaction;

// Generic prepare method for rusqlite DrizzleBuilder
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Creates a prepared statement that can be executed multiple times
    #[inline]
    pub fn prepare(self) -> prepared::PreparedStatement<'a> {
        let inner = prepare_render(self.to_sql().clone());
        prepared::PreparedStatement { inner }
    }
}

/// Drizzle instance that provides access to the database and query builder.
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    conn: Connection,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
    #[inline]
    pub const fn new<S>(conn: Connection, schema: S) -> (Drizzle<S>, S) {
        let drizzle = Drizzle {
            conn,
            _schema: PhantomData,
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
    /// Gets a reference to the underlying connection
    #[inline]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[inline]
    pub fn mut_conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'b, SQLiteValue<'b>>,
    {
        use drizzle_sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'a, Schema, InsertInitial, Table>, InsertInitial>
    where
        Table: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'a, Schema, UpdateInitial, Table>, UpdateInitial>
    where
        Table: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'a, Schema, DeleteInitial, T>, DeleteInitial>
    where
        T: SQLiteTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression).
    pub fn with<'a, C>(
        &'a self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    pub fn execute<'a, T>(&'a self, query: T) -> rusqlite::Result<usize>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        self.conn.execute(&sql, params_from_iter(params))
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
        let sql = query.to_sql();
        let sql_str = sql.sql();

        let params = sql.params();

        let mut stmt = self
            .conn
            .prepare(&sql_str)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?;

        rows.collect::<Result<Result<C, _>, _>>()?
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();

        // Get parameters and handle potential errors from IntoParams
        let params = sql.params();

        let mut stmt = self.conn.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }

    /// Executes a transaction with the given callback
    pub fn transaction<F, R>(
        &mut self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Transaction<Schema>) -> drizzle_core::error::Result<R>,
    {
        let tx = self.conn.transaction_with_behavior(tx_type.into())?;

        let transaction = Transaction::new(tx, tx_type);

        // Use catch_unwind to handle panics and ensure rollback
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&transaction)));

        match result {
            Ok(callback_result) => match callback_result {
                Ok(value) => {
                    transaction.commit()?;
                    Ok(value)
                }
                Err(e) => {
                    transaction.rollback()?;
                    Err(e)
                }
            },
            Err(panic_payload) => {
                // Rollback on panic and resume unwinding
                let _ = transaction.rollback();
                std::panic::resume_unwind(panic_payload);
            }
        }
    }
}

// Implementation for schemas that implement SQLSchemaImpl
impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects using SQLSchemaImpl trait
    pub fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements();
        if !statements.is_empty() {
            let batch_sql = statements.join(";");
            self.conn.execute_batch(&batch_sql)?;
        }
        Ok(())
    }
}

impl<Schema> Drizzle<Schema> {
    /// Run migrations from the specified directory.
    ///
    /// This method:
    /// 1. Creates a migrations tracking table if it doesn't exist
    /// 2. Loads all migrations from the directory
    /// 3. Applies any pending migrations in order
    ///
    /// # Example
    ///
    /// ```no_run
    /// use drizzle::rusqlite::Drizzle;
    /// use std::path::Path;
    ///
    /// let conn = rusqlite::Connection::open("my.db").unwrap();
    /// let (db, _) = Drizzle::new(conn, ());
    ///
    /// // Run migrations from ./drizzle/migrations
    /// db.migrate(Path::new("./drizzle/migrations")).unwrap();
    /// ```
    pub fn migrate(&self, migrations_dir: &std::path::Path) -> drizzle_core::error::Result<usize> {
        use drizzle_schema::migrator::load_migrations_from_dir;

        // Load migrations
        let migrations = load_migrations_from_dir(migrations_dir)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        if migrations.is_empty() {
            return Ok(0);
        }

        // Create migrations table
        self.conn.execute(
            r#"CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );"#,
            [],
        )?;

        // Get applied migrations
        let mut stmt = self
            .conn
            .prepare(r#"SELECT hash FROM "__drizzle_migrations" ORDER BY id;"#)?;
        let applied: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        // Apply pending migrations
        let pending: Vec<_> = migrations
            .iter()
            .filter(|m| !applied.contains(&m.tag))
            .collect();

        let count = pending.len();

        for migration in pending {
            // Execute each statement
            for stmt in migration.statements() {
                self.conn.execute(stmt, [])?;
            }

            // Record as applied
            self.conn.execute(
                r#"INSERT INTO "__drizzle_migrations" (hash) VALUES (?1);"#,
                [&migration.tag],
            )?;
        }

        Ok(count)
    }

    /// Run migrations using a drizzle.toml config file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use drizzle::rusqlite::Drizzle;
    /// use std::path::Path;
    ///
    /// let conn = rusqlite::Connection::open("my.db").unwrap();
    /// let (db, _) = Drizzle::new(conn, ());
    ///
    /// // Run migrations using config
    /// db.migrate_with_config(Path::new("./drizzle.toml")).unwrap();
    /// ```
    pub fn migrate_with_config(
        &self,
        config_path: &std::path::Path,
    ) -> drizzle_core::error::Result<usize> {
        use drizzle_schema::Migrator;

        let migrator = Migrator::from_config_file(config_path)
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        // Create migrations table
        self.conn
            .execute(&migrator.create_migrations_table_sql(), [])?;

        // Get applied migrations
        let mut stmt = self.conn.prepare(&migrator.query_applied_sql())?;
        let applied: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        // Apply pending migrations
        let pending = migrator.pending_migrations(&applied);
        let count = pending.len();

        for migration in pending {
            for stmt in migration.statements() {
                self.conn.execute(stmt, [])?;
            }

            self.conn
                .execute(&migrator.record_migration_sql(&migration.tag), [])?;
        }

        Ok(count)
    }
}

// CTE (WITH) Builder Implementation for RusQLite
impl<'a, Schema>
    DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(
        self,
        cte: C,
    ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

// Rusqlite-specific execution methods for all ExecutableState QueryBuilders
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params().into_iter().cloned();
        Ok(self
            .drizzle
            .conn
            .execute(&sql_str, params_from_iter(params))?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params().into_iter().cloned();

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?;

        rows.collect::<Result<Result<C, _>, _>>()?
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params = self.builder.sql.params().into_iter().cloned();

        let mut stmt = self.drizzle.conn.prepare(&sql_str)?;
        stmt.query_row(params_from_iter(params), |row| {
            Ok(R::try_from(row).map_err(Into::into))
        })?
    }
}

impl<'a, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'a, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
        self.builder.to_sql()
    }
}
