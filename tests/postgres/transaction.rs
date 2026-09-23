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

#[drizzle::test]
fn swallowed_statement_errors_fail_the_commit(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let outcome = result!(db.transaction(TransactionConfig::default(), |tx| {
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("kept")])
                .execute()
        )?;
        // The error is ignored, but PostgreSQL has aborted the transaction,
        // so COMMIT rolls back. This used to report success.
        let _ = result!(tx.execute(SQL::raw("SELECT 1 / 0")));
        Ok(())
    }));
    assert!(outcome.is_err(), "{outcome:?}");

    let rows: Vec<SelectSimple> = db.select(()).from(simple).all();
    assert!(rows.is_empty());
}

#[drizzle::test]
fn a_failed_statement_rolls_back_only_its_savepoint(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    result!(db.transaction(TransactionConfig::default(), |tx| {
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("outer")])
                .execute()
        )?;
        // The savepoint body ignores the error and returns Ok, but only
        // rolling back to the savepoint recovers the transaction.
        let savepoint = result!(tx.savepoint(|sp| {
            result!(
                sp.insert(simple)
                    .values([InsertSimple::new("inner")])
                    .execute()
            )?;
            let _ = result!(sp.execute(SQL::raw("SELECT 1 / 0")));
            Ok(())
        }));
        assert!(savepoint.is_err());
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("after")])
                .execute()
        )?;
        Ok(())
    }))?;

    let names: Vec<String> = db
        .select(simple.name)
        .from(simple)
        .order_by(asc(simple.name))
        .all();
    assert_eq!(names, ["after", "outer"]);
}

#[drizzle::test]
fn transactions_select_distinct_on(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    db.insert(simple)
        .values([
            InsertSimple::new("a"),
            InsertSimple::new("a"),
            InsertSimple::new("b"),
        ])
        .execute();

    let names = result!(db.transaction(TransactionConfig::default(), |tx| {
        let rows: Vec<SelectSimple> = result!(
            tx.select_distinct_on(simple.name, (simple.id, simple.name))
                .from(simple)
                .order_by((asc(simple.name), asc(simple.id)))
                .all()
        )?;
        Ok(rows.into_iter().map(|row| row.name).collect::<Vec<_>>())
    }))?;
    assert_eq!(names, ["a", "b"]);
}
