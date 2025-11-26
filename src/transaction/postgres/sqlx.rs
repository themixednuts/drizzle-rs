use drizzle_core::error::DrizzleError;
use drizzle_postgres::PostgresTransactionType;
use sqlx::PgConnection;
use std::marker::PhantomData;

/// PostgreSQL transaction wrapper using sqlx
pub struct Transaction<Schema = ()> {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
    tx_type: PostgresTransactionType,
    _schema: PhantomData<Schema>,
}

impl<Schema> Transaction<Schema> {
    pub(crate) fn new(
        tx: sqlx::Transaction<'static, sqlx::Postgres>,
        tx_type: PostgresTransactionType,
    ) -> Self {
        Self {
            tx,
            tx_type,
            _schema: PhantomData,
        }
    }

    /// Commit the transaction
    pub async fn commit(self) -> drizzle_core::error::Result<()> {
        self.tx
            .commit()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Rollback the transaction
    pub async fn rollback(self) -> drizzle_core::error::Result<()> {
        self.tx
            .rollback()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Get the transaction type
    pub fn transaction_type(&self) -> PostgresTransactionType {
        self.tx_type
    }

    /// Get a reference to the underlying sqlx transaction
    pub fn as_sqlx(&self) -> &sqlx::Transaction<'static, sqlx::Postgres> {
        &self.tx
    }

    /// Get a mutable reference to the underlying sqlx transaction
    pub fn as_sqlx_mut(&mut self) -> &mut sqlx::Transaction<'static, sqlx::Postgres> {
        &mut self.tx
    }
}
