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
