//! Async SQLite driver using [`libsql`].
//!
//! # Example
//!
//! ```ignore
//! use drizzle::prelude::*;
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

mod prepared;

use drizzle_core::error::DrizzleError;
use drizzle_core::prepared::prepare_render;
use drizzle_core::traits::ToSQL;
use libsql::{Connection, Row};
use std::future::Future;
use std::pin::Pin;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::{
    builder::{self, QueryBuilder},
    connection::SQLiteTransactionType,
    values::SQLiteValue,
};

// Generic prepare method for DrizzleBuilder
crate::drizzle_prepare_impl!();

use crate::builder::sqlite::common;
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
        let sql = query.sql();
        let params: Vec<libsql::Value> = query.params().map(|p| p.into()).collect();

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
        C: std::iter::FromIterator<R>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params: Vec<libsql::Value> = sql.params().map(|p| p.into()).collect();

        let mut rows = self
            .conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = query.to_sql();
        let sql_str = sql.sql();
        let params: Vec<libsql::Value> = sql.params().map(|p| p.into()).collect();

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

    /// Executes a transaction with the given callback
    pub async fn transaction<F, R>(
        &self,
        tx_type: SQLiteTransactionType,
        f: F,
    ) -> drizzle_core::error::Result<R>
    where
        F: for<'t> FnOnce(
            &'t Transaction<Schema>,
        )
            -> Pin<Box<dyn Future<Output = Result<R, DrizzleError>> + Send + 't>>,
    {
        let tx = self.conn.transaction_with_behavior(tx_type.into()).await?;
        let transaction = Transaction::new(tx, tx_type);

        // run the closure
        let result = f(&transaction).await;

        // now commit/rollback by moving transaction safely
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

// Implementation for schemas that implement SQLSchemaImpl
impl<Schema> Drizzle<Schema>
where
    Schema: drizzle_core::traits::SQLSchemaImpl + Default,
{
    /// Create schema objects using SQLSchemaImpl trait
    pub async fn create(&self) -> drizzle_core::error::Result<()> {
        let schema = Schema::default();
        let statements = schema.create_statements();
        if !statements.is_empty() {
            let batch_sql = statements.join(";");
            self.conn.execute_batch(&batch_sql).await?;
        }
        Ok(())
    }
}

// Migration support
impl<Schema> common::Drizzle<Connection, Schema> {
    /// Run pending migrations from a MigrationSet.
    ///
    /// This method follows the drizzle-orm migration spec:
    /// - Creates the migrations table if it doesn't exist (idempotent)
    /// - Queries the last applied migration by `created_at`
    /// - Runs all pending migrations in a single transaction
    /// - Records each migration after execution
    ///
    /// # Example
    ///
    /// ```ignore
    /// use drizzle::sqlite::libsql::Drizzle;
    /// use drizzle_migrations::{migrations, MigrationSet};
    /// use drizzle_types::Dialect;
    ///
    /// let migrations = migrations![
    ///     ("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
    ///     ("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
    /// ];
    /// let set = MigrationSet::new(migrations, Dialect::SQLite);
    ///
    /// let db = libsql::Builder::new_local("./dev.db").build().await?;
    /// let conn = db.connect()?;
    /// let (drizzle, _) = Drizzle::new(conn, ());
    ///
    /// drizzle.migrate(&set).await?;
    /// ```
    pub async fn migrate(
        &self,
        migrations: &drizzle_migrations::MigrationSet,
    ) -> drizzle_core::error::Result<()> {
        // 1. Create migrations table (idempotent)
        self.conn
            .execute(&migrations.create_table_sql(), ())
            .await?;

        // 2. Query all applied hashes
        let mut rows = self
            .conn
            .query(&migrations.query_all_hashes_sql(), ())
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut applied_hashes: Vec<String> = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        {
            if let Ok(hash) = row.get::<String>(0) {
                applied_hashes.push(hash);
            }
        }

        // 3. Get pending migrations
        let pending: Vec<_> = migrations.pending(&applied_hashes).collect();

        if pending.is_empty() {
            return Ok(());
        }

        // 4. Execute all pending in a single transaction
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
            // Record migration
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

// Generic execution methods for all ExecutableState QueryBuilders (LibSQL)
#[cfg(feature = "libsql")]
impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let sql_str = self.builder.sql.sql();
        let params: Vec<libsql::Value> = self.builder.sql.params().map(|p| p.into()).collect();
        Ok(self.drizzle.conn.execute(&sql_str, params).await?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: std::iter::FromIterator<R>,
    {
        let sql_str = self.builder.sql.sql();
        let params: Vec<libsql::Value> = self.builder.sql.params().map(|p| p.into()).collect();

        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }
        Ok(results.into_iter().collect())
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let sql_str = self.builder.sql.sql();
        let params: Vec<libsql::Value> = self.builder.sql.params().map(|p| p.into()).collect();

        let mut rows = self.drizzle.conn.query(&sql_str, params).await?;
        if let Some(row) = rows.next().await? {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(drizzle_core::error::DrizzleError::NotFound)
        }
    }
}
