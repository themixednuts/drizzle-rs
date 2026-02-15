//! PostgreSQL transaction tests
//!
//! Tests for transaction execution with both sync and async drivers.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;
use drizzle_postgres::common::PostgresTransactionType;

#[derive(Debug, PostgresFromRow, PartialEq)]
struct TxSimpleResult {
    id: i32,
    name: String,
}

postgres_test!(transaction_commit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice")])
            .execute()
    );

    // Insert inside a transaction that commits
    drizzle_exec!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("Bob")])
                    .execute()
            )?;
            Ok(())
        })
    ));

    // Both rows should be visible
    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    drizzle_assert_eq!(2, results.len());
});

postgres_test!(transaction_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice")])
            .execute()
    );

    // Transaction that returns an error should rollback
    let result: Result<(), drizzle::error::DrizzleError> = drizzle_try!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("Bob")])
                    .execute()
            )?;
            Err(drizzle::error::DrizzleError::Other("rollback".into()))
        })
    ));
    let _ = result; // Ignore the Err — we expect rollback

    // Only the first row should be visible (transaction was rolled back)
    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    drizzle_assert_eq!(1, results.len());
    drizzle_assert_eq!("Alice", results[0].name.as_str());
});

postgres_test!(transaction_update_and_select, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("Bob"),])
            .execute()
    );

    // Update inside a transaction
    drizzle_exec!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.update(simple)
                    .set(UpdateSimple::default().with_name("Charlie"))
                    .r#where(eq(simple.name, "Bob"))
                    .execute()
            )?;

            // Verify the update is visible within the transaction
            let results: Vec<TxSimpleResult> =
                drizzle_try!(tx.select((simple.id, simple.name)).from(simple).all())?;

            let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
            assert!(names.contains(&"Alice"));
            assert!(names.contains(&"Charlie"));
            assert!(!names.contains(&"Bob"));

            Ok(())
        })
    ));

    // Verify persisted after commit
    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    drizzle_assert!(names.contains(&"Charlie"));
    drizzle_assert!(!names.contains(&"Bob"));
});

postgres_test!(transaction_delete, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("Alice"),
                InsertSimple::new("Bob"),
                InsertSimple::new("Charlie"),
            ])
            .execute()
    );

    // Delete inside a transaction
    drizzle_exec!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            drizzle_try!(tx.delete(simple).r#where(eq(simple.name, "Bob")).execute())?;
            Ok(())
        })
    ));

    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    drizzle_assert_eq!(2, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    drizzle_assert!(!names.contains(&"Bob"));
});

// --- Savepoint (nested transaction) tests ---

postgres_test!(savepoint_commit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            // Insert in outer transaction
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("outer")])
                    .execute()
            )?;

            // Savepoint that commits
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("inner")])
                        .execute()
                )?;
                Ok(())
            })))?;

            Ok(())
        })
    ));

    // Both records should exist
    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    drizzle_assert_eq!(2, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    drizzle_assert!(names.contains(&"outer"));
    drizzle_assert!(names.contains(&"inner"));
});

postgres_test!(savepoint_rollback_preserves_outer, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            // Insert in outer transaction
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("outer")])
                    .execute()
            )?;

            // Savepoint that rolls back
            let sp_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("inner_rollback")])
                        .execute()
                )?;
                Err(drizzle::error::DrizzleError::Other("rollback inner".into()))
            })));

            // Savepoint error should not abort the outer transaction
            assert!(sp_result.is_err());

            // Insert another record after the rolled-back savepoint
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("after_sp")])
                    .execute()
            )?;

            Ok(())
        })
    ));

    // Only outer + after_sp should exist, inner_rollback should be gone
    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    drizzle_assert_eq!(2, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    drizzle_assert!(names.contains(&"outer"));
    drizzle_assert!(names.contains(&"after_sp"));
    drizzle_assert!(!names.contains(&"inner_rollback"));
});

postgres_test!(savepoint_nested_two_levels, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(db.transaction(
        PostgresTransactionType::default(),
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("level0")])
                    .execute()
            )?;

            // First savepoint
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("level1")])
                        .execute()
                )?;

                // Nested savepoint
                drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                    drizzle_try!(
                        tx.insert(simple)
                            .values([InsertSimple::new("level2")])
                            .execute()
                    )?;
                    Ok(())
                })))?;

                Ok(())
            })))?;

            Ok(())
        })
    ));

    let results: Vec<TxSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple).all());
    drizzle_assert_eq!(3, results.len());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    drizzle_assert!(names.contains(&"level0"));
    drizzle_assert!(names.contains(&"level1"));
    drizzle_assert!(names.contains(&"level2"));
});
