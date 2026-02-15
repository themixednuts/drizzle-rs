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

    /// Executes a transaction with the given callback
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

#[cfg(feature = "libsql")]
impl<'a, 'b, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();
        Ok(self.drizzle.conn.execute(&sql_str, params).await?)
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub async fn all<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: Default + Extend<R>,
    {
        self.rows::<R>().await?.collect().await
    }

    /// Runs the query and returns a row cursor.
    pub async fn rows<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        let (sql_str, params) = self.builder.sql.build();
        let params: Vec<libsql::Value> = params.into_iter().map(|p| p.into()).collect();

        let rows = self.drizzle.conn.query(&sql_str, params).await?;
        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
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
}
