#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use common::{InsertSimple, SelectSimple, Simple, UpdateSimple};
use drizzle_rs::error::DrizzleError;
use drizzle_rs::prelude::*;

mod common;

#[tokio::test]
async fn test_transaction_commit() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Test successful transaction
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Insert first record
        tx.insert(simple)
            .values([InsertSimple::new("user1")])
            .execute()?;

        // Insert second record
        tx.insert(simple)
            .values([InsertSimple::new("user2")])
            .execute()?;

        Ok(())
    });

    assert!(result.is_ok());

    // Verify both records were inserted
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "user1");
    assert_eq!(users[1].name, "user2");
}

#[tokio::test]
async fn test_transaction_rollback() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Insert initial record outside transaction
    db.insert(simple)
        .values([InsertSimple::new("initial_user")])
        .execute()
        .unwrap();

    // Test transaction that should rollback
    let result: Result<(), DrizzleError> = db.transaction(SQLiteTransactionType::Immediate, |tx| {
        // Insert a record
        tx.insert(simple)
            .values([InsertSimple::new("temp_user")])
            .execute()?;

        // Simulate an error to trigger rollback
        Err(DrizzleError::Other("Intentional rollback".to_string()))
    });

    assert!(result.is_err());

    // Verify only the initial record exists (transaction was rolled back)
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial_user");
}

#[tokio::test]
async fn test_transaction_types() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Test different transaction types
    for tx_type in [
        SQLiteTransactionType::Deferred,
        SQLiteTransactionType::Immediate,
        SQLiteTransactionType::Exclusive,
    ] {
        let result = db.transaction(tx_type, |tx| {
            let user_name = format!("user_{:?}", tx_type);
            tx.insert(simple)
                .values([InsertSimple::new(user_name.as_str())])
                .execute()?;
            Ok(())
        });

        assert!(result.is_ok(), "Transaction failed for type {:?}", tx_type);
    }

    // Verify all records were inserted
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 3);
}

#[tokio::test]
async fn test_transaction_query_builders() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Insert test data
    db.insert(simple)
        .values([
            InsertSimple::new("alice"),
            InsertSimple::new("bob"),
            InsertSimple::new("charlie"),
        ])
        .execute()
        .unwrap();

    // Test all query builders work in transaction
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Test SELECT
        let users: Vec<SelectSimple> = tx
            .select(())
            .from(simple)
            .r#where(eq(simple.name, "alice"))
            .all()?;
        assert_eq!(users.len(), 1);

        // Test INSERT
        tx.insert(simple)
            .values([InsertSimple::new("dave")])
            .execute()?;

        // Test UPDATE
        tx.update(simple)
            .set(UpdateSimple::default().with_name("updated_bob"))
            .r#where(eq(simple.name, "bob"))
            .execute()?;

        // Test DELETE
        tx.delete(simple)
            .r#where(eq(simple.name, "charlie"))
            .execute()?;

        Ok(())
    });

    assert!(result.is_ok());

    // Verify final state
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
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
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Insert initial data
    db.insert(simple)
        .values([InsertSimple::new("initial")])
        .execute()
        .unwrap();

    // Test transaction that fails due to database constraint
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Insert valid record
        tx.insert(simple)
            .values([InsertSimple::new("valid_insert")])
            .execute()?;

        // Try to insert duplicate primary key (should cause DB error)
        tx.insert(simple)
            .values([InsertSimple::new("duplicate").with_id(1)]) // Same ID as "initial"
            .execute()?;

        Ok(())
    });

    assert!(result.is_err());

    // Verify rollback - only initial record should exist
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial");
}

#[tokio::test]
async fn test_transaction_panic_rollback() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Insert initial data
    db.insert(simple)
        .values([InsertSimple::new("before_panic")])
        .execute()
        .unwrap();

    // Test transaction that panics
    let result: Result<Result<(), DrizzleError>, _> = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        db.transaction(SQLiteTransactionType::Deferred, |tx| {
            // Insert a record
            tx.insert(simple)
                .values([InsertSimple::new("should_rollback")])
                .execute()?;

            // Panic!
            panic!("Simulated panic in transaction");
        })
    }));

    assert!(result.is_err()); // Panic occurred

    // Verify rollback - panic should have triggered rollback
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "before_panic");
}

#[tokio::test]
async fn test_nested_transaction_operations() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Test complex transaction with multiple interdependent operations
    let result = db.transaction(SQLiteTransactionType::Immediate, |tx| {
        // Insert users
        tx.insert(simple)
            .values([
                InsertSimple::new("user1"),
                InsertSimple::new("user2"),
                InsertSimple::new("user3"),
            ])
            .execute()?;

        // Verify inserts worked
        let count: Vec<SelectSimple> = tx.select(()).from(simple).all()?;
        assert_eq!(count.len(), 3);

        // Update one user
        tx.update(simple)
            .set(UpdateSimple::default().with_name("updated_user1"))
            .r#where(eq(simple.name, "user1"))
            .execute()?;

        // Delete one user
        tx.delete(simple)
            .r#where(eq(simple.name, "user3"))
            .execute()?;

        // Verify intermediate state
        let remaining: Vec<SelectSimple> = tx.select(()).from(simple).all()?;
        assert_eq!(remaining.len(), 2);

        // If we got this far, everything should commit
        Ok(())
    });

    assert!(result.is_ok());

    // Verify final committed state
    let final_users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
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
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Test transaction where a query fails in the middle
    let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
        // Insert first record (should succeed)
        tx.insert(simple)
            .values([InsertSimple::new("first")])
            .execute()?;

        // Insert second record (should succeed)  
        tx.insert(simple)
            .values([InsertSimple::new("second")])
            .execute()?;

        // Try invalid operation that should fail
        // Attempt to update non-existent record and verify it returns 0 affected rows
        let affected = tx.update(simple)
            .set(UpdateSimple::default().with_name("wont_work"))
            .r#where(eq(simple.name, "nonexistent_user"))
            .execute()?;

        if affected == 0 {
            return Err(DrizzleError::Other("No rows affected by update".to_string()));
        }

        Ok(())
    });

    assert!(result.is_err());

    // Verify complete rollback - no records should exist
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 0);
}

#[tokio::test]
async fn test_large_transaction_rollback() {
    let conn = setup_test_db!();
    let (mut db, simple) = drizzle!(conn, [Simple]);

    // Test rollback of transaction with many operations
    let result: Result<(), DrizzleError> = db.transaction(SQLiteTransactionType::Exclusive, |tx| {
        // Insert many records
        for i in 0..100 {
            let user_name = format!("user_{}", i);
            tx.insert(simple)
                .values([InsertSimple::new(user_name.as_str())])
                .execute()?;
        }

        // Verify all were inserted
        let count: Vec<SelectSimple> = tx.select(()).from(simple).all()?;
        assert_eq!(count.len(), 100);

        // Force rollback
        Err(DrizzleError::Other("Intentional rollback of large transaction".to_string()))
    });

    assert!(result.is_err());

    // Verify complete rollback - no records should exist
    let users: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();
    assert_eq!(users.len(), 0);
}
