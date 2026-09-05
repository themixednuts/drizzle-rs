//! PostgreSQL-specific transaction contracts.
//!
//! Portable commit/rollback/savepoint behavior lives in
//! `crate::common::transaction`; this file covers configuration that only
//! PostgreSQL understands.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::postgres::prelude::*;

#[derive(PostgresFromRow)]
struct TxSettings {
    isolation: String,
    read_only: String,
    deferrable: String,
}

#[drizzle::test]
fn transaction_config_reaches_the_server(db: &mut TestDb<SimpleSchema>) {
    let config = TransactionConfig::builder()
        .serializable()
        .read_only()
        .deferrable()
        .build();

    result!(db.transaction(config, |tx| {
        let settings: TxSettings = result!(tx.get(SQL::raw(
            "SELECT current_setting('transaction_isolation') AS isolation, \
             current_setting('transaction_read_only') AS read_only, \
             current_setting('transaction_deferrable') AS deferrable"
        )))?;

        assert_eq!(settings.isolation, "serializable");
        assert_eq!(settings.read_only, "on");
        assert_eq!(settings.deferrable, "on");
        Ok(())
    }))?;
}

// Static assertion: OwnedPreparedStatement is Send + Sync
#[cfg(feature = "tokio-postgres")]
#[test]
fn test_pg_owned_prepared_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<drizzle_postgres::builder::prepared::OwnedPreparedStatement>();
}
