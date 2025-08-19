#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use common::{InsertSimple, SelectSimple, Simple, SimpleSchema, UpdateSimple};
use drizzle_rs::error::DrizzleError;
use drizzle_rs::prelude::*;

mod common;

#[tokio::test]
async fn test_transaction_commit() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(feature = "rusqlite")]
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Insert first record
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("user1")])
                .execute()
        )?;

        // Insert second record
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("user2")])
                .execute()
        )?;

        Ok(())
    });

    // Test successful transaction
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .transaction(SQLiteTransactionType::Deferred, |tx| {
            Box::pin(async move {
                // Insert first record
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("user1")])
                        .execute()
                )?;

                // Insert second record
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("user2")])
                        .execute()
                )?;

                Ok(())
            })
        })
        .await;

    assert!(result.is_ok());

    // Verify both records were inserted
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "user1");
    assert_eq!(users[1].name, "user2");
}

#[tokio::test]
async fn test_transaction_rollback() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert initial record outside transaction
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("initial_user")])
            .execute()
    );

    // Test transaction that should rollback
    #[cfg(feature = "rusqlite")]
    let result: Result<(), DrizzleError> = db.transaction(SQLiteTransactionType::Immediate, |tx| {
        // Insert a record
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("temp_user")])
                .execute()
        )?;

        // Simulate an error to trigger rollback
        Err(DrizzleError::Other("Intentional rollback".to_string()))
    });

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result: Result<(), DrizzleError> = db
        .transaction(SQLiteTransactionType::Immediate, |tx| {
            Box::pin(async move {
                // Insert a record
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("temp_user")])
                        .execute()
                )?;

                // Simulate an error to trigger rollback
                Err(DrizzleError::Other("Intentional rollback".to_string()))
            })
        })
        .await;

    assert!(result.is_err());

    // Verify only the initial record exists (transaction was rolled back)
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial_user");
}

#[tokio::test]
async fn test_transaction_types() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Test different transaction types
    for tx_type in [
        SQLiteTransactionType::Deferred,
        SQLiteTransactionType::Immediate,
        SQLiteTransactionType::Exclusive,
    ] {
        #[cfg(feature = "rusqlite")]
        let result = db.transaction(tx_type, |tx| {
            let user_name = format!("user_{:?}", tx_type);
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new(user_name.as_str())])
                    .execute()
            )?;
            Ok(())
        });

        #[cfg(any(feature = "turso", feature = "libsql"))]
        let result = db
            .transaction(tx_type, |tx| {
                Box::pin(async move {
                    let user_name = format!("user_{:?}", tx_type);
                    drizzle_try!(
                        tx.insert(simple)
                            .values([InsertSimple::new(user_name.as_str())])
                            .execute()
                    )?;
                    Ok(())
                })
            })
            .await;

        assert!(result.is_ok(), "Transaction failed for type {:?}", tx_type);
    }

    // Verify all records were inserted
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 3);
}

#[tokio::test]
async fn test_transaction_query_builders() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert test data
    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("alice"),
                InsertSimple::new("bob"),
                InsertSimple::new("charlie"),
            ])
            .execute()
    );

    // Test all query builders work in transaction
    #[cfg(feature = "rusqlite")]
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Test SELECT
        let users: Vec<SelectSimple> = drizzle_try!(
            tx.select(())
                .from(simple)
                .r#where(eq(simple.name, "alice"))
                .all()
        )?;
        assert_eq!(users.len(), 1);

        // Test INSERT
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("dave")])
                .execute()
        )?;

        // Test UPDATE
        drizzle_try!(
            tx.update(simple)
                .set(UpdateSimple::default().with_name("updated_bob"))
                .r#where(eq(simple.name, "bob"))
                .execute()
        )?;

        // Test DELETE
        drizzle_try!(
            tx.delete(simple)
                .r#where(eq(simple.name, "charlie"))
                .execute()
        )?;

        Ok(())
    });
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .transaction(SQLiteTransactionType::Deferred, |tx| {
            Box::pin(async move {
                // Test SELECT
                let users: Vec<SelectSimple> = drizzle_try!(
                    tx.select(())
                        .from(simple)
                        .r#where(eq(simple.name, "alice"))
                        .all()
                )?;
                assert_eq!(users.len(), 1);

                // Test INSERT
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("dave")])
                        .execute()
                )?;

                // Test UPDATE
                drizzle_try!(
                    tx.update(simple)
                        .set(UpdateSimple::default().with_name("updated_bob"))
                        .r#where(eq(simple.name, "bob"))
                        .execute()
                )?;

                // Test DELETE
                drizzle_try!(
                    tx.delete(simple)
                        .r#where(eq(simple.name, "charlie"))
                        .execute()
                )?;

                Ok(())
            })
        })
        .await;

    assert!(result.is_ok());

    // Verify final state
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 3); // alice, dave, updated_bob

    let names: Vec<String> = users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"alice".to_string()));
    assert!(names.contains(&"dave".to_string()));
    assert!(names.contains(&"updated_bob".to_string()));
    assert!(!names.contains(&"bob".to_string()));
    assert!(!names.contains(&"charlie".to_string()));
}

#[tokio::test]
async fn test_transaction_database_error_rollback() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("initial")])
            .execute()
    );

    // Test transaction that fails due to database constraint
    #[cfg(feature = "rusqlite")]
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Insert valid record
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("valid_insert")])
                .execute()
        )?;

        // Try to insert duplicate primary key (should cause DB error)
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("duplicate").with_id(1)]) // Same ID as "initial"
                .execute()
        )?;

        Ok(())
    });
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .transaction(SQLiteTransactionType::Deferred, |tx| {
            Box::pin(async move {
                // Insert valid record
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("valid_insert")])
                        .execute()
                )?;

                // Try to insert duplicate primary key (should cause DB error)
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("duplicate").with_id(1)]) // Same ID as "initial"
                        .execute()
                )?;

                Ok(())
            })
        })
        .await;

    assert!(result.is_err());

    // Verify rollback - only initial record should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial");
}

#[tokio::test]
async fn test_transaction_panic_rollback() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("before_panic")])
            .execute()
    );

    // Test transaction that panics
    #[cfg(feature = "rusqlite")]
    let result: Result<Result<(), DrizzleError>, _> =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            db.transaction(SQLiteTransactionType::Deferred, |tx| {
                // Insert a record
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("should_rollback")])
                        .execute()
                )?;

                // Panic!
                panic!("Simulated panic in transaction");
            })
        }));
    // For async drivers (turso/libsql), use FutureExt::catch_unwind for panic testing
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        use futures_util::future::FutureExt;
        use std::panic::AssertUnwindSafe;

        let panic_future = db.transaction(SQLiteTransactionType::Deferred, |tx| {
            Box::pin(async move {
                // Insert a record
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("should_rollback")])
                        .execute()
                )?;

                // Panic!
                panic!("Simulated panic in transaction");
            })
        });

        let result: Result<Result<(), DrizzleError>, _> =
            AssertUnwindSafe(panic_future).catch_unwind().await;
        assert!(result.is_err()); // Panic occurred
    }

    #[cfg(feature = "rusqlite")]
    assert!(result.is_err()); // Panic occurred

    // Verify rollback - panic should have triggered rollback
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "before_panic");
}

#[tokio::test]
async fn test_nested_transaction_operations() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(feature = "rusqlite")]
    // Test complex transaction with multiple interdependent operations
    let result = db.transaction(SQLiteTransactionType::Immediate, |tx| {
        // Insert users
        drizzle_try!(
            tx.insert(simple)
                .values([
                    InsertSimple::new("user1"),
                    InsertSimple::new("user2"),
                    InsertSimple::new("user3"),
                ])
                .execute()
        )?;

        // Verify inserts worked
        let count: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
        assert_eq!(count.len(), 3);

        // Update one user
        drizzle_try!(
            tx.update(simple)
                .set(UpdateSimple::default().with_name("updated_user1"))
                .r#where(eq(simple.name, "user1"))
                .execute()
        )?;

        // Delete one user
        drizzle_try!(
            tx.delete(simple)
                .r#where(eq(simple.name, "user3"))
                .execute()
        )?;

        // Verify intermediate state
        let remaining: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
        assert_eq!(remaining.len(), 2);

        // If we got this far, everything should commit
        Ok(())
    });

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .transaction(SQLiteTransactionType::Immediate, |tx| {
            Box::pin(async move {
                // Insert users
                drizzle_try!(
                    tx.insert(simple)
                        .values([
                            InsertSimple::new("user1"),
                            InsertSimple::new("user2"),
                            InsertSimple::new("user3"),
                        ])
                        .execute()
                )?;

                // Verify inserts worked
                let count: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(count.len(), 3);

                // Update one user
                drizzle_try!(
                    tx.update(simple)
                        .set(UpdateSimple::default().with_name("updated_user1"))
                        .r#where(eq(simple.name, "user1"))
                        .execute()
                )?;

                // Delete one user
                drizzle_try!(
                    tx.delete(simple)
                        .r#where(eq(simple.name, "user3"))
                        .execute()
                )?;

                // Verify intermediate state
                let remaining: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(remaining.len(), 2);

                // If we got this far, everything should commit
                Ok(())
            })
        })
        .await;

    assert!(result.is_ok());

    // Verify final committed state
    let final_users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(final_users.len(), 2);

    let names: Vec<String> = final_users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"updated_user1".to_string()));
    assert!(names.contains(&"user2".to_string()));
    assert!(!names.contains(&"user1".to_string()));
    assert!(!names.contains(&"user3".to_string()));
}

#[tokio::test]
async fn test_transaction_with_failed_query_in_middle() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(feature = "rusqlite")]
    // Test transaction where a query fails in the middle
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Insert first record (should succeed)
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("first")])
                .execute()
        )?;

        // Insert second record (should succeed)
        drizzle_try!(
            tx.insert(simple)
                .values([InsertSimple::new("second")])
                .execute()
        )?;

        // Try invalid operation that should fail
        // Attempt to update non-existent record and verify it returns 0 affected rows
        let affected = drizzle_try!(
            tx.update(simple)
                .set(UpdateSimple::default().with_name("wont_work"))
                .r#where(eq(simple.name, "nonexistent_user"))
                .execute()
        )?;

        if affected == 0 {
            return Err(DrizzleError::Other(
                "No rows affected by update".to_string(),
            ));
        }

        Ok(())
    });

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .transaction(SQLiteTransactionType::Deferred, |tx| {
            Box::pin(async move {
                // Insert first record (should succeed)
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("first")])
                        .execute()
                )?;

                // Insert second record (should succeed)
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("second")])
                        .execute()
                )?;

                // Try invalid operation that should fail
                // Attempt to update non-existent record and verify it returns 0 affected rows
                let affected = drizzle_try!(
                    tx.update(simple)
                        .set(UpdateSimple::default().with_name("wont_work"))
                        .r#where(eq(simple.name, "nonexistent_user"))
                        .execute()
                )?;

                if affected == 0 {
                    return Err(DrizzleError::Other(
                        "No rows affected by update".to_string(),
                    ));
                }

                Ok(())
            })
        })
        .await;

    assert!(result.is_err());

    // Verify complete rollback - no records should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 0);
}

#[tokio::test]
async fn test_large_transaction_rollback() {
    let conn = setup_test_db!();
    #[cfg(feature = "rusqlite")]
    let (mut db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Test rollback of transaction with many operations
    #[cfg(feature = "rusqlite")]
    let result: Result<(), DrizzleError> = db.transaction(SQLiteTransactionType::Exclusive, |tx| {
        // Insert many records
        for i in 0..100 {
            let user_name = format!("user_{}", i);
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new(user_name.as_str())])
                    .execute()
            )?;
        }

        // Verify all were inserted
        let count: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
        assert_eq!(count.len(), 100);

        // Force rollback
        Err(DrizzleError::Other(
            "Intentional rollback of large transaction".to_string(),
        ))
    });
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result: Result<(), DrizzleError> = db
        .transaction(SQLiteTransactionType::Exclusive, |tx| {
            Box::pin(async move {
                // Insert many records
                for i in 0..100 {
                    let user_name = format!("user_{}", i);
                    drizzle_try!(
                        tx.insert(simple)
                            .values([InsertSimple::new(user_name.as_str())])
                            .execute()
                    )?;
                }

                // Verify all were inserted
                let count: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(count.len(), 100);

                // Force rollback
                Err(DrizzleError::Other(
                    "Intentional rollback of large transaction".to_string(),
                ))
            })
        })
        .await;

    assert!(result.is_err());

    // Verify complete rollback - no records should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 0);
}
