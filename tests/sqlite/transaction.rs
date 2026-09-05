#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::connection::SQLiteTransactionType;
use drizzle::sqlite::prelude::*;

#[drizzle::test]
fn test_transaction_types(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Test different transaction types
    for tx_type in [
        SQLiteTransactionType::Deferred,
        SQLiteTransactionType::Immediate,
        SQLiteTransactionType::Exclusive,
    ] {
        let result = result!(db.transaction(tx_type, |tx| {
            let user_name = format!("user_{:?}", tx_type);
            result!(
                tx.insert(simple)
                    .values([InsertSimple::new(user_name.as_str())])
                    .execute()
            )?;
            Ok(())
        }));

        assert!(result.is_ok(), "Transaction failed for type {:?}", tx_type);
    }

    // Verify all records were inserted
    let users: Vec<SelectSimple> = db.select(()).from(simple).all();
    assert_eq!(users.len(), 3);
}

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

#[drizzle::test]
fn test_owned_statement_binds_closure_local_by_reference(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
        .execute();

    #[derive(SQLiteFromRow, Default)]
    struct PartialSimple {
        name: String,
    }

    let name = simple.name.placeholder("name");

    // A borrowed statement pins its bindings to the statement's own lifetime,
    // so a value built inside the closure cannot be bound by reference.
    // `into_owned` gives the executors a fresh per-call lifetime instead, so
    // closure-local values bind borrowed — no clone of the bound value.
    let by_name = db
        .select(simple.name)
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    let found = result!(db.transaction(SQLiteTransactionType::Deferred, |tx| {
        let wanted = String::from("Ali") + "ce";
        let rows: Vec<PartialSimple> =
            result!(by_name.all(tx.inner(), [name.bind(wanted.as_str())]))?;
        Ok(rows.into_iter().map(|row| row.name).collect::<Vec<_>>())
    }));

    assert_eq!(found.unwrap(), vec!["Alice".to_string()]);
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_owned_prepared_query_survives_transaction_rollback() -> drizzle::Result<()> {
    let (mut db, SimpleSchema { simple }) =
        crate::common::helpers::turso_setup::setup_db::<SimpleSchema>().await;
    let prepared = db.select(()).from(simple).prepare().into_owned();

    let rolled_back: drizzle::Result<()> = db
        .transaction(TransactionConfig::Deferred, async |tx| {
            tx.insert(simple)
                .value(InsertSimple::new("rolled back"))
                .execute()
                .await?;
            Err(drizzle::error::DrizzleError::Other("rollback".into()))
        })
        .await;
    assert!(rolled_back.is_err());

    let rows: Vec<SelectSimple> = prepared.all(db.conn(), []).await?;
    assert!(rows.is_empty());
    Ok(())
}

// Static assertion: OwnedPreparedStatement is Send + Sync
#[cfg(feature = "rusqlite")]
#[test]
fn test_sqlite_owned_prepared_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<drizzle_sqlite::builder::prepared::OwnedPreparedStatement>();
}
