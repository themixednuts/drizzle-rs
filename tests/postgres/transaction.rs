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
                    .r#where(true)
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

#[derive(PostgresFromRow)]
struct RowCount {
    n: i64,
}

fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
    })
}

// A transaction that imports another's snapshot sees that transaction's data,
// not rows committed after the export.
#[cfg(feature = "postgres-sync")]
#[test]
fn an_imported_snapshot_reads_the_exporters_data_sync() {
    use drizzle::postgres::sync::Drizzle;

    let connect = || postgres::Client::connect(&database_url(), postgres::NoTls).unwrap();
    let mut writer = connect();
    writer
        .batch_execute(
            "DROP TABLE IF EXISTS snapshot_probe_sync; \
             CREATE TABLE snapshot_probe_sync (id integer); \
             INSERT INTO snapshot_probe_sync VALUES (1)",
        )
        .unwrap();
    let (mut exporter, ()) = Drizzle::<()>::new(connect());
    let (mut importer, ()) = Drizzle::<()>::new(connect());
    let count = SQL::raw("SELECT count(*) AS n FROM snapshot_probe_sync");
    let repeatable_read = TransactionConfig::builder().repeatable_read().build();

    exporter
        .transaction(repeatable_read, |tx| {
            let snapshot = tx.export_snapshot()?;
            writer
                .execute("INSERT INTO snapshot_probe_sync VALUES (2)", &[])
                .unwrap();

            let config = TransactionConfig::builder()
                .repeatable_read()
                .snapshot(snapshot)
                .build();
            let seen: RowCount = importer.transaction(config, |other| other.get(count.clone()))?;
            assert_eq!(seen.n, 1);

            let fresh: RowCount =
                importer.transaction(repeatable_read, |other| other.get(count.clone()))?;
            assert_eq!(fresh.n, 2);
            Ok(())
        })
        .unwrap();

    writer
        .batch_execute("DROP TABLE snapshot_probe_sync")
        .unwrap();
}

#[cfg(feature = "tokio-postgres")]
#[tokio::test]
async fn an_imported_snapshot_reads_the_exporters_data_tokio() {
    use drizzle::postgres::tokio::Drizzle;

    async fn connect() -> tokio_postgres::Client {
        let (client, connection) = tokio_postgres::connect(&database_url(), tokio_postgres::NoTls)
            .await
            .unwrap();
        tokio::spawn(connection);
        client
    }

    let writer = connect().await;
    writer
        .batch_execute(
            "DROP TABLE IF EXISTS snapshot_probe_tokio; \
             CREATE TABLE snapshot_probe_tokio (id integer); \
             INSERT INTO snapshot_probe_tokio VALUES (1)",
        )
        .await
        .unwrap();
    let (mut exporter, ()) = Drizzle::<()>::new(connect().await);
    let (mut importer, ()) = Drizzle::<()>::new(connect().await);
    let count = SQL::raw("SELECT count(*) AS n FROM snapshot_probe_tokio");
    let repeatable_read = TransactionConfig::builder().repeatable_read().build();

    exporter
        .transaction(repeatable_read, async |tx| {
            let snapshot = tx.export_snapshot().await?;
            writer
                .execute("INSERT INTO snapshot_probe_tokio VALUES (2)", &[])
                .await
                .unwrap();

            // The id is `Copy + 'static`, so it can reach a spawned task.
            let count = count.clone();
            let task = tokio::spawn(async move {
                let config = TransactionConfig::builder()
                    .repeatable_read()
                    .snapshot(snapshot)
                    .build();
                let seen: RowCount = importer
                    .transaction(config, async |other| other.get(count.clone()).await)
                    .await?;
                let fresh: RowCount = importer
                    .transaction(repeatable_read, async |other| {
                        other.get(count.clone()).await
                    })
                    .await?;
                drizzle::Result::Ok((seen.n, fresh.n))
            });
            assert_eq!(task.await.unwrap()?, (1, 2));
            Ok(())
        })
        .await
        .unwrap();

    writer
        .batch_execute("DROP TABLE snapshot_probe_tokio")
        .await
        .unwrap();
}
