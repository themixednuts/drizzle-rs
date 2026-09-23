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

// Static assertion: a tokio-postgres transaction future is Send, so it can run
// inside `tokio::spawn` or a web handler. The transaction used to keep its
// connection in a `RefCell`, which made every future holding `&Transaction`
// across an `.await` `!Send`.
#[cfg(feature = "tokio-postgres")]
#[test]
fn test_tokio_postgres_transaction_futures_are_send() {
    use drizzle::postgres::TransactionConfig;

    fn assert_send<T: Send>(_: T) {}

    fn transaction_future(db: &mut drizzle::postgres::tokio::Drizzle<SimpleSchema>) {
        let SimpleSchema { simple } = SimpleSchema::new();
        assert_send(
            db.transaction(TransactionConfig::default(), async move |tx| {
                tx.insert(simple)
                    .values([InsertSimple::new("sent")])
                    .execute()
                    .await?;
                tx.update(simple)
                    .set(UpdateSimple::default().with_name("still sent"))
                    .execute()
                    .await?;
                let rows: Vec<SelectSimple> = tx.select(()).from(simple).all().await?;
                Ok(rows.len())
            }),
        );
    }

    let _ = transaction_future;
}
