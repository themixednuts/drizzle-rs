//! PostgreSQL transaction tests
//!
//! Tests for transaction execution with both sync and async drivers.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_postgres::common::PostgresTransactionType;

#[derive(Debug, PostgresFromRow, PartialEq)]
struct TxSimpleResult {
    id: i32,
    name: String,
}

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

#[drizzle::test]
fn transaction_commit(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial data

    db.insert(simple)
        .values([InsertSimple::new("Alice")])
        .execute();

    // Insert inside a transaction that commits
    db.transaction(PostgresTransactionType::default(), |tx| {
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("Bob")])
                .execute()
        )?;
        Ok(())
    });

    // Both rows should be visible
    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(2, results.len());
}

#[drizzle::test]
fn transaction_rollback(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial data

    db.insert(simple)
        .values([InsertSimple::new("Alice")])
        .execute();

    // Transaction that returns an error should rollback
    let result: Result<(), drizzle::error::DrizzleError> =
        result!(db.transaction(PostgresTransactionType::default(), |tx| {
            result!(
                tx.insert(simple)
                    .values([InsertSimple::new("Bob")])
                    .execute()
            )?;
            Err(drizzle::error::DrizzleError::Other("rollback".into()))
        }));
    let _ = result; // Ignore the Err — we expect rollback

    // Only the first row should be visible (transaction was rolled back)
    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(1, results.len());
    assert_eq!("Alice", results[0].name.as_str());
}

#[drizzle::test]
fn transaction_update_and_select(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial data

    db.insert(simple)
        .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
        .execute();

    // Update inside a transaction
    db.transaction(PostgresTransactionType::default(), |tx| {
        result!(
            tx.update(simple)
                .set(UpdateSimple::default().with_name("Charlie"))
                .r#where(eq(simple.name, "Bob"))
                .execute()
        )?;

        // Verify the update is visible within the transaction
        let results: Vec<TxSimpleResult> =
            result!(tx.select((simple.id, simple.name)).from(simple).all())?;

        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Alice"));
        assert!(names.contains(&"Charlie"));
        assert!(!names.contains(&"Bob"));

        Ok(())
    });

    // Verify persisted after commit
    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Charlie"));
    assert!(!names.contains(&"Bob"));
}

#[drizzle::test]
fn transaction_delete(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial data

    db.insert(simple)
        .values([
            InsertSimple::new("Alice"),
            InsertSimple::new("Bob"),
            InsertSimple::new("Charlie"),
        ])
        .execute();

    // Delete inside a transaction
    db.transaction(PostgresTransactionType::default(), |tx| {
        result!(tx.delete(simple).r#where(eq(simple.name, "Bob")).execute())?;
        Ok(())
    });

    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(2, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(!names.contains(&"Bob"));
}

// --- Savepoint (nested transaction) tests ---

#[drizzle::test]
fn savepoint_commit(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.transaction(PostgresTransactionType::default(), |tx| {
        // Insert in outer transaction
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("outer")])
                .execute()
        )?;

        // Savepoint that commits
        result!(tx.savepoint(|tx| {
            result!(
                tx.insert(simple)
                    .values([InsertSimple::new("inner")])
                    .execute()
            )?;
            Ok(())
        }))?;

        Ok(())
    });

    // Both records should exist
    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(2, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"outer"));
    assert!(names.contains(&"inner"));
}

#[drizzle::test]
fn savepoint_rollback_preserves_outer(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.transaction(PostgresTransactionType::default(), |tx| {
        // Insert in outer transaction
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("outer")])
                .execute()
        )?;

        // Savepoint that rolls back
        let sp_result: Result<(), _> = result!(tx.savepoint(|tx| {
            result!(
                tx.insert(simple)
                    .values([InsertSimple::new("inner_rollback")])
                    .execute()
            )?;
            Err(drizzle::error::DrizzleError::Other("rollback inner".into()))
        }));

        // Savepoint error should not abort the outer transaction
        assert!(sp_result.is_err());

        // Insert another record after the rolled-back savepoint
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("after_sp")])
                .execute()
        )?;

        Ok(())
    });

    // Only outer + after_sp should exist, inner_rollback should be gone
    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(2, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"outer"));
    assert!(names.contains(&"after_sp"));
    assert!(!names.contains(&"inner_rollback"));
}

#[drizzle::test]
fn savepoint_nested_two_levels(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.transaction(PostgresTransactionType::default(), |tx| {
        result!(
            tx.insert(simple)
                .values([InsertSimple::new("level0")])
                .execute()
        )?;

        // First savepoint
        result!(tx.savepoint(|tx| {
            result!(
                tx.insert(simple)
                    .values([InsertSimple::new("level1")])
                    .execute()
            )?;

            // Nested savepoint
            result!(tx.savepoint(|tx| {
                result!(
                    tx.insert(simple)
                        .values([InsertSimple::new("level2")])
                        .execute()
                )?;
                Ok(())
            }))?;

            Ok(())
        }))?;

        Ok(())
    });

    let results: Vec<TxSimpleResult> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(3, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"level0"));
    assert!(names.contains(&"level1"));
    assert!(names.contains(&"level2"));
}

// Static assertion: OwnedPreparedStatement is Send + Sync
#[cfg(feature = "tokio-postgres")]
#[test]
fn test_pg_owned_prepared_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<drizzle_postgres::builder::prepared::OwnedPreparedStatement>();
}
