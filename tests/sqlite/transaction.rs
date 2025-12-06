#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use crate::common::{InsertSimple, SelectSimple, SimpleSchema, UpdateSimple};
use drizzle::error::DrizzleError;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::SQLiteTransactionType;
use drizzle_macros::sqlite_test;

sqlite_test!(test_transaction_commit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(SQLiteTransactionType::Deferred, |tx| {
        drizzle_tx!(tx, {
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
    }));

    assert!(result.is_ok());

    // Verify both records were inserted
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "user1");
    assert_eq!(users[1].name, "user2");
});

sqlite_test!(test_transaction_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial record outside transaction
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("initial_user")])
            .execute()
    );

    let result: Result<(), DrizzleError> = drizzle_try!(db.transaction(
        SQLiteTransactionType::Immediate,
        |tx| drizzle_tx!(tx, {
            // Insert a record
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("temp_user")])
                    .execute()
            )?;

            // Simulate an error to trigger rollback
            Err(DrizzleError::Other(
                "Intentional rollback".to_string().into(),
            ))
        })
    ));

    assert!(result.is_err());

    // Verify only the initial record exists (transaction was rolled back)
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial_user");
});

sqlite_test!(test_transaction_types, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Test different transaction types
    for tx_type in [
        SQLiteTransactionType::Deferred,
        SQLiteTransactionType::Immediate,
        SQLiteTransactionType::Exclusive,
    ] {
        let result = drizzle_try!(db.transaction(tx_type, |tx| drizzle_tx!(tx, {
            let user_name = format!("user_{:?}", tx_type);
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new(user_name.as_str())])
                    .execute()
            )?;
            Ok(())
        })));

        assert!(result.is_ok(), "Transaction failed for type {:?}", tx_type);
    }

    // Verify all records were inserted
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 3);
});

sqlite_test!(test_transaction_query_builders, SimpleSchema, {
    let SimpleSchema { simple } = schema;

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

    let result = drizzle_try!(
        db.transaction(SQLiteTransactionType::Deferred, |tx| drizzle_tx!(tx, {
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
        }))
    );

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
});

sqlite_test!(test_transaction_database_error_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("initial")])
            .execute()
    );

    let result = drizzle_try!(
        db.transaction(SQLiteTransactionType::Deferred, |tx| drizzle_tx!(tx, {
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
        }))
    );

    assert!(result.is_err());

    // Verify rollback - only initial record should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial");
});

sqlite_test!(test_transaction_panic_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("before_panic")])
            .execute()
    );

    // Attempt transaction that will panic
    let result: Result<Result<(), DrizzleError>, _> = drizzle_catch_unwind!(db.transaction(
        SQLiteTransactionType::Deferred,
        |tx| drizzle_tx!(tx, {
            // Insert a record
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("should_rollback")])
                    .execute()
            )?;

            // Panic!
            panic!("Simulated panic in transaction");
        })
    ));

    assert!(result.is_err()); // Panic occurred

    // Verify rollback - panic should have triggered rollback
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "before_panic");
});

sqlite_test!(test_nested_transaction_operations, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(
        db.transaction(SQLiteTransactionType::Immediate, |tx| drizzle_tx!(tx, {
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
        }))
    );

    assert!(result.is_ok());

    // Verify final committed state
    let final_users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(final_users.len(), 2);

    let names: Vec<String> = final_users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"updated_user1".to_string()));
    assert!(names.contains(&"user2".to_string()));
    assert!(!names.contains(&"user1".to_string()));
    assert!(!names.contains(&"user3".to_string()));
});

sqlite_test!(
    test_transaction_with_failed_query_in_middle,
    SimpleSchema,
    {
        let SimpleSchema { simple } = schema;

        // Test transaction where a query fails in the middle
        let result = drizzle_try!(db.transaction(
            SQLiteTransactionType::Deferred,
            |tx| drizzle_tx!(tx, {
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
                        "No rows affected by update".to_string().into(),
                    ));
                }

                Ok(())
            })
        ));

        assert!(result.is_err());

        // Verify complete rollback - no records should exist
        let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
        assert_eq!(users.len(), 0);
    }
);

sqlite_test!(test_large_transaction_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Test rollback of transaction with many operations
    let result: Result<(), DrizzleError> = drizzle_try!(db.transaction(
        SQLiteTransactionType::Exclusive,
        |tx| drizzle_tx!(tx, {
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
                "Intentional rollback of large transaction"
                    .to_string()
                    .into(),
            ))
        })
    ));

    assert!(result.is_err());

    // Verify complete rollback - no records should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert_eq!(users.len(), 0);
});
