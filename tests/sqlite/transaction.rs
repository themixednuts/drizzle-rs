#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema, UpdateSimple};
use drizzle::core::expr::*;
use drizzle::error::DrizzleError;
use drizzle::sqlite::connection::SQLiteTransactionType;
use drizzle_core::SQL;
use drizzle_macros::sqlite_test;
use drizzle_sqlite::params;

sqlite_test!(test_transaction_commit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
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
    ));

    assert!(result.is_ok());

    // Verify both records were inserted
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
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
            => execute
    );

    let result: Result<(), DrizzleError> = drizzle_try!(db.transaction(
        SQLiteTransactionType::Immediate,
        drizzle_tx!(tx, {
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
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
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
        let result = drizzle_try!(db.transaction(
            tx_type,
            drizzle_tx!(tx, {
                let user_name = format!("user_{:?}", tx_type);
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new(user_name.as_str())])
                        .execute()
                )?;
                Ok(())
            })
        ));

        assert!(result.is_ok(), "Transaction failed for type {:?}", tx_type);
    }

    // Verify all records were inserted
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
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
            => execute
    );

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
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
    ));

    assert!(result.is_ok());

    // Verify final state
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
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
            => execute
    );

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
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
    ));

    assert!(result.is_err());

    // Verify rollback - only initial record should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "initial");
});

sqlite_test!(test_transaction_panic_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("before_panic")])
            => execute
    );

    // Attempt transaction that will panic
    let result: Result<Result<(), DrizzleError>, _> = drizzle_catch_unwind!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
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
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "before_panic");
});

sqlite_test!(test_nested_transaction_operations, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Immediate,
        drizzle_tx!(tx, {
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
    ));

    assert!(result.is_ok());

    // Verify final committed state
    let final_users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
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
            drizzle_tx!(tx, {
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
        let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
        assert_eq!(users.len(), 0);
    }
);

sqlite_test!(test_large_transaction_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Test rollback of transaction with many operations
    let result: Result<(), DrizzleError> = drizzle_try!(db.transaction(
        SQLiteTransactionType::Exclusive,
        drizzle_tx!(tx, {
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
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 0);
});

sqlite_test!(test_savepoint_commit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
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

    assert!(result.is_ok());

    // Both records should exist
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 2);
    let names: Vec<String> = users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"outer".to_string()));
    assert!(names.contains(&"inner".to_string()));
});

sqlite_test!(test_savepoint_rollback_preserves_outer, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
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
                Err(DrizzleError::Other("rollback inner".to_string().into()))
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

    assert!(result.is_ok());

    // Only outer + after_sp should exist, inner_rollback should be gone
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 2);
    let names: Vec<String> = users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"outer".to_string()));
    assert!(names.contains(&"after_sp".to_string()));
    assert!(!names.contains(&"inner_rollback".to_string()));
});

sqlite_test!(test_savepoint_outer_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result: Result<(), DrizzleError> = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
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

            // Rollback the entire outer transaction
            Err(DrizzleError::Other("rollback outer".to_string().into()))
        })
    ));

    assert!(result.is_err());

    // Everything should be rolled back
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 0);
});

sqlite_test!(test_nested_savepoints, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("level_0")])
                    .execute()
            )?;

            // First level savepoint
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("level_1")])
                        .execute()
                )?;

                // Second level savepoint (nested within first)
                drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                    drizzle_try!(
                        tx.insert(simple)
                            .values([InsertSimple::new("level_2")])
                            .execute()
                    )?;
                    Ok(())
                })))?;

                Ok(())
            })))?;

            Ok(())
        })
    ));

    assert!(result.is_ok());

    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 3);
    let names: Vec<String> = users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"level_0".to_string()));
    assert!(names.contains(&"level_1".to_string()));
    assert!(names.contains(&"level_2".to_string()));
});

// Standalone rusqlite-only stress test for deeply nested savepoints.
// Tests 50 levels of recursive savepoint nesting via our Transaction::savepoint() method.
// Uses tx.inner() for raw SQL inserts to avoid complex type-state builder types in recursion.
#[cfg(feature = "rusqlite")]
mod test_deep_savepoint_nesting_rusqlite {
    use crate::common::schema::sqlite::{SelectSimple, SimpleSchema};
    use drizzle::sqlite::connection::SQLiteTransactionType;

    fn nest<S>(
        tx: &drizzle::sqlite::rusqlite::Transaction<'_, S>,
        depth: usize,
        max: usize,
    ) -> drizzle_core::error::Result<()> {
        if depth >= max {
            return Ok(());
        }
        tx.savepoint(|tx| {
            let name = format!("depth_{}", depth);
            tx.inner().execute(
                "INSERT INTO simple (name) VALUES (?1)",
                rusqlite::params![&name],
            )?;
            nest(tx, depth + 1, max)
        })
    }

    #[test]
    fn run() -> Result<(), drizzle::error::DrizzleError> {
        use crate::common::helpers::rusqlite_setup;

        let (mut db, schema) = rusqlite_setup::setup_db::<SimpleSchema>();
        let SimpleSchema { simple } = schema;

        const MAX_DEPTH: usize = 50;

        let result = db.transaction(SQLiteTransactionType::Deferred, |tx| nest(tx, 0, MAX_DEPTH));

        assert!(
            result.is_ok(),
            "50-level nested savepoint failed: {:?}",
            result.err()
        );

        let users: Vec<SelectSimple> = db.select(()).from(simple).all()?;
        assert_eq!(
            users.len(),
            MAX_DEPTH,
            "Expected {} rows, got {}",
            MAX_DEPTH,
            users.len()
        );

        // Verify each depth level inserted exactly one row
        for i in 0..MAX_DEPTH {
            let expected = format!("depth_{}", i);
            assert!(
                users.iter().any(|u| u.name == expected),
                "Missing row for depth {}",
                i
            );
        }

        Ok(())
    }
}

// Test deep nesting with partial rollback at a specific depth.
// Ensures that rolling back an inner savepoint doesn't affect outer levels,
// and that the transaction can continue inserting after a rollback.
#[cfg(feature = "rusqlite")]
mod test_deep_savepoint_partial_rollback_rusqlite {
    use crate::common::schema::sqlite::{SelectSimple, SimpleSchema};
    use drizzle::error::DrizzleError;
    use drizzle::sqlite::connection::SQLiteTransactionType;

    fn nest_with_rollback_at<S>(
        tx: &drizzle::sqlite::rusqlite::Transaction<'_, S>,
        depth: usize,
        max: usize,
        rollback_at: usize,
    ) -> drizzle_core::error::Result<()> {
        if depth >= max {
            return Ok(());
        }
        let sp_result: drizzle_core::error::Result<()> = tx.savepoint(|tx| {
            let name = format!("depth_{}", depth);
            tx.inner().execute(
                "INSERT INTO simple (name) VALUES (?1)",
                rusqlite::params![&name],
            )?;

            if depth == rollback_at {
                return Err(DrizzleError::Other(
                    format!("rollback at depth {}", depth).into(),
                ));
            }

            nest_with_rollback_at(tx, depth + 1, max, rollback_at)
        });

        // If this savepoint rolled back, continue — insert a recovery row outside the savepoint
        if sp_result.is_err() && depth == rollback_at {
            let recovery = format!("recovered_{}", depth);
            tx.inner().execute(
                "INSERT INTO simple (name) VALUES (?1)",
                rusqlite::params![&recovery],
            )?;
        } else {
            sp_result?;
        }

        Ok(())
    }

    #[test]
    fn run() -> Result<(), drizzle::error::DrizzleError> {
        use crate::common::helpers::rusqlite_setup;

        let (mut db, schema) = rusqlite_setup::setup_db::<SimpleSchema>();
        let SimpleSchema { simple } = schema;

        const MAX_DEPTH: usize = 20;
        const ROLLBACK_AT: usize = 15;

        let result = db.transaction(SQLiteTransactionType::Deferred, |tx| {
            nest_with_rollback_at(tx, 0, MAX_DEPTH, ROLLBACK_AT)
        });

        assert!(
            result.is_ok(),
            "Partial rollback test failed: {:?}",
            result.err()
        );

        let users: Vec<SelectSimple> = db.select(()).from(simple).all()?;

        // Depths 0..15 committed successfully
        for i in 0..ROLLBACK_AT {
            let expected = format!("depth_{}", i);
            assert!(
                users.iter().any(|u| u.name == expected),
                "Missing row for depth {}",
                i
            );
        }

        // Depth 15 was rolled back, but a recovery row was inserted at the parent level
        assert!(
            !users
                .iter()
                .any(|u| u.name == format!("depth_{}", ROLLBACK_AT)),
            "depth_{} should have been rolled back",
            ROLLBACK_AT
        );
        assert!(
            users
                .iter()
                .any(|u| u.name == format!("recovered_{}", ROLLBACK_AT)),
            "recovered_{} should exist",
            ROLLBACK_AT
        );

        // Depths 16..20 never ran (they were inside the rolled-back savepoint)
        for i in (ROLLBACK_AT + 1)..MAX_DEPTH {
            assert!(
                !users.iter().any(|u| u.name == format!("depth_{}", i)),
                "depth_{} should not exist (after rollback point)",
                i
            );
        }

        Ok(())
    }
}

sqlite_test!(test_sequential_sibling_savepoints, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            // Insert before any savepoints
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("before_sp")])
                    .execute()
            )?;

            // First savepoint — commits
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp1")])
                        .execute()
                )?;
                Ok(())
            })))?;

            // Second savepoint — commits
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp2")])
                        .execute()
                )?;
                Ok(())
            })))?;

            // Third savepoint — commits
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp3")])
                        .execute()
                )?;
                Ok(())
            })))?;

            // Insert after all savepoints
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("after_sp")])
                    .execute()
            )?;

            Ok(())
        })
    ));

    assert!(result.is_ok());

    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 5);
    let names: Vec<String> = users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"before_sp".to_string()));
    assert!(names.contains(&"sp1".to_string()));
    assert!(names.contains(&"sp2".to_string()));
    assert!(names.contains(&"sp3".to_string()));
    assert!(names.contains(&"after_sp".to_string()));
});

sqlite_test!(test_sequential_savepoints_mixed_outcomes, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            // sp1: commits successfully
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp1_committed")])
                        .execute()
                )?;
                Ok(())
            })))?;

            // sp2: rolls back
            let sp2_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp2_rolled_back")])
                        .execute()
                )?;
                Err(DrizzleError::Other("sp2 fail".to_string().into()))
            })));
            assert!(sp2_result.is_err());

            // sp3: commits successfully — the transaction recovered from sp2 failure
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp3_committed")])
                        .execute()
                )?;
                Ok(())
            })))?;

            // sp4: rolls back
            let sp4_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp4_rolled_back")])
                        .execute()
                )?;
                Err(DrizzleError::Other("sp4 fail".to_string().into()))
            })));
            assert!(sp4_result.is_err());

            // sp5: commits — can still recover after multiple failures
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("sp5_committed")])
                        .execute()
                )?;
                Ok(())
            })))?;

            Ok(())
        })
    ));

    assert!(result.is_ok());

    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();

    // Committed savepoints should be present
    assert!(names.contains(&"sp1_committed".to_string()));
    assert!(names.contains(&"sp3_committed".to_string()));
    assert!(names.contains(&"sp5_committed".to_string()));

    // Rolled-back savepoints should NOT be present
    assert!(!names.contains(&"sp2_rolled_back".to_string()));
    assert!(!names.contains(&"sp4_rolled_back".to_string()));

    assert_eq!(users.len(), 3, "exactly 3 committed savepoints");
});

sqlite_test!(test_savepoint_data_visibility, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            // Insert outside any savepoint
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("outer_data")])
                    .execute()
            )?;

            // Inside a savepoint, verify we can see the outer data
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(rows.len(), 1, "savepoint should see outer data");
                assert_eq!(rows[0].name, "outer_data");

                // Insert inside the savepoint
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("inner_data")])
                        .execute()
                )?;

                // Verify we can see both
                let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(rows.len(), 2, "should see both outer and inner");
                Ok(())
            })))?;

            // After savepoint commits, verify outer can see savepoint data
            let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
            assert_eq!(rows.len(), 2, "outer should see committed savepoint data");

            let names: Vec<String> = rows.into_iter().map(|u| u.name).collect();
            assert!(names.contains(&"outer_data".to_string()));
            assert!(names.contains(&"inner_data".to_string()));

            Ok(())
        })
    ));

    assert!(result.is_ok());
});

sqlite_test!(test_savepoint_rolled_back_data_not_visible, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("outer_data")])
                    .execute()
            )?;

            // Savepoint that inserts then rolls back
            let sp_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("ghost_data")])
                        .execute()
                )?;

                // Verify data is visible inside the savepoint before rollback
                let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(rows.len(), 2, "should see both before rollback");

                Err(DrizzleError::Other("rollback".to_string().into()))
            })));
            assert!(sp_result.is_err());

            // After rollback, ghost_data should be gone
            let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
            assert_eq!(
                rows.len(),
                1,
                "only outer_data should remain after rollback"
            );
            assert_eq!(rows[0].name, "outer_data");

            Ok(())
        })
    ));

    assert!(result.is_ok());
});

sqlite_test!(test_savepoint_update_and_delete_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            // Set up initial data
            drizzle_try!(
                tx.insert(simple)
                    .values([
                        InsertSimple::new("alice"),
                        InsertSimple::new("bob"),
                        InsertSimple::new("charlie"),
                    ])
                    .execute()
            )?;

            // Savepoint: update + delete, then rollback
            let sp_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                // Update alice → alicia
                drizzle_try!(
                    tx.update(simple)
                        .set(UpdateSimple::default().with_name("alicia"))
                        .r#where(eq(simple.name, "alice"))
                        .execute()
                )?;

                // Delete bob
                drizzle_try!(tx.delete(simple).r#where(eq(simple.name, "bob")).execute())?;

                // Verify the changes took effect inside the savepoint
                let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
                assert_eq!(rows.len(), 2);
                let names: Vec<String> = rows.into_iter().map(|u| u.name).collect();
                assert!(names.contains(&"alicia".to_string()));
                assert!(names.contains(&"charlie".to_string()));
                assert!(!names.contains(&"alice".to_string()));
                assert!(!names.contains(&"bob".to_string()));

                // Now rollback everything
                Err(DrizzleError::Other("undo changes".to_string().into()))
            })));
            assert!(sp_result.is_err());

            // After rollback, original data should be restored
            let rows: Vec<SelectSimple> = drizzle_try!(tx.select(()).from(simple).all())?;
            assert_eq!(rows.len(), 3, "all 3 original rows should be back");
            let names: Vec<String> = rows.into_iter().map(|u| u.name).collect();
            assert!(
                names.contains(&"alice".to_string()),
                "alice should be back (update rolled back)"
            );
            assert!(
                names.contains(&"bob".to_string()),
                "bob should be back (delete rolled back)"
            );
            assert!(names.contains(&"charlie".to_string()));

            Ok(())
        })
    ));

    assert!(result.is_ok());

    // Verify final committed state matches
    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 3);
});

sqlite_test!(test_nested_savepoint_inner_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("level_0")])
                    .execute()
            )?;

            // First level savepoint
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("level_1")])
                        .execute()
                )?;

                // Second level savepoint that rolls back
                let inner_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                    drizzle_try!(
                        tx.insert(simple)
                            .values([InsertSimple::new("level_2_rollback")])
                            .execute()
                    )?;
                    Err(DrizzleError::Other("rollback level 2".to_string().into()))
                })));

                assert!(inner_result.is_err());

                // level_1 should still work after inner rollback
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("after_inner_rollback")])
                        .execute()
                )?;

                Ok(())
            })))?;

            Ok(())
        })
    ));

    assert!(result.is_ok());

    let users: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(users.len(), 3);
    let names: Vec<String> = users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"level_0".to_string()));
    assert!(names.contains(&"level_1".to_string()));
    assert!(names.contains(&"after_inner_rollback".to_string()));
    assert!(!names.contains(&"level_2_rollback".to_string()));
});

// --- Prepared statement + transaction tests ---

sqlite_test!(test_prepared_outside_transaction, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
            => execute
    );

    // Create an owned prepared statement OUTSIDE the transaction
    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    // Use it INSIDE the transaction
    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            let alice: Vec<SelectSimple> =
                drizzle_try!(prepared.all(tx.inner(), params![{name: "Alice"}]))?;
            assert_eq!(alice.len(), 1);
            assert_eq!(alice[0].name, "Alice");

            // Reuse with different params in the same transaction
            let bob: Vec<SelectSimple> =
                drizzle_try!(prepared.all(tx.inner(), params![{name: "Bob"}]))?;
            assert_eq!(bob.len(), 1);
            assert_eq!(bob[0].name, "Bob");

            // No match
            let nobody: Vec<SelectSimple> =
                drizzle_try!(prepared.all(tx.inner(), params![{name: "Nobody"}]))?;
            assert_eq!(nobody.len(), 0);

            Ok(())
        })
    ));

    assert!(result.is_ok());
});

sqlite_test!(test_prepared_in_savepoint, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice")])
            => execute
    );

    // Prepared statement created outside everything
    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            // Insert inside transaction
            drizzle_try!(
                tx.insert(simple)
                    .values([InsertSimple::new("Bob")])
                    .execute()
            )?;

            // Use prepared statement inside a savepoint
            drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                let both: Vec<SelectSimple> =
                    drizzle_try!(prepared.all(tx.inner(), params![{name: "Alice"}]))?;
                assert_eq!(both.len(), 1);

                let bob: Vec<SelectSimple> =
                    drizzle_try!(prepared.all(tx.inner(), params![{name: "Bob"}]))?;
                assert_eq!(bob.len(), 1);

                Ok(())
            })))?;

            Ok(())
        })
    ));

    assert!(result.is_ok());
});

sqlite_test!(test_prepared_survives_savepoint_rollback, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice")])
            => execute
    );

    let prepared = db.select(()).from(simple).prepare().into_owned();

    let result = drizzle_try!(db.transaction(
        SQLiteTransactionType::Deferred,
        drizzle_tx!(tx, {
            // Savepoint that inserts then rolls back
            let sp_result: Result<(), _> = drizzle_try!(tx.savepoint(drizzle_tx!(tx, {
                drizzle_try!(
                    tx.insert(simple)
                        .values([InsertSimple::new("Ghost")])
                        .execute()
                )?;

                // Prepared statement sees both rows inside the savepoint
                let rows: Vec<SelectSimple> = drizzle_try!(prepared.all(tx.inner(), []))?;
                assert_eq!(rows.len(), 2);

                Err(DrizzleError::Other("rollback".into()))
            })));
            assert!(sp_result.is_err());

            // After rollback, prepared statement sees only Alice
            let rows: Vec<SelectSimple> = drizzle_try!(prepared.all(tx.inner(), []))?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].name, "Alice");

            Ok(())
        })
    ));

    assert!(result.is_ok());
});

// Static assertion: OwnedPreparedStatement is Send + Sync
#[cfg(feature = "rusqlite")]
#[test]
fn test_sqlite_owned_prepared_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<drizzle_sqlite::builder::prepared::OwnedPreparedStatement>();
}
