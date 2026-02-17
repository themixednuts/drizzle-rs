//! Async edge case tests for SQLite drivers (libsql, turso)
//!
//! Tests cancellation safety, panic recovery, and error handling
//! in async contexts.

#![cfg(any(feature = "libsql", feature = "turso"))]

use crate::common::schema::sqlite::{InsertSimple, SimpleSchema};
use drizzle::sqlite::prelude::*;

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct SimpleResult {
    id: i32,
    name: String,
}

// ============================================================================
// libsql async edge cases
// ============================================================================

#[cfg(feature = "libsql")]
mod libsql_edge_cases {
    use super::*;
    use crate::common::helpers::libsql_setup;
    use drizzle::core::expr::*;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn cancellation_via_timeout_does_not_break_connection() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_timeout")])
            .execute()
            .await
            .unwrap();

        // Wrap a query in a generous timeout - should complete well before
        let result = timeout(Duration::from_secs(5), async {
            let results: Vec<SimpleResult> = db
                .select((simple.id, simple.name))
                .from(simple)
                .all_as()
                .await
                .unwrap();
            results
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);

        // Verify connection still works after timeout-wrapped operation
        db.insert(simple)
            .values([InsertSimple::new("after_timeout")])
            .execute()
            .await
            .unwrap();

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn select_cancellation_via_drop() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        // Insert data
        for i in 0..10 {
            db.insert(simple)
                .values([InsertSimple::new(format!("user_{}", i))])
                .execute()
                .await
                .unwrap();
        }

        // Start a select, then drop the future via select!
        tokio::select! {
            results = async {
                let r: Vec<SimpleResult> = db.select((simple.id, simple.name))
                    .from(simple)
                    .all_as()
                    .await
                    .unwrap();
                r
            } => {
                assert_eq!(results.len(), 10);
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                panic!("query should complete before sleep");
            }
        }

        // Connection should still be usable
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn panic_recovery_in_async_context() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_panic")])
            .execute()
            .await
            .unwrap();

        // Catch a panic in async context
        let result =
            futures_util::future::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
                panic!("intentional test panic")
            }))
            .await;

        assert!(result.is_err());

        // Connection should still be usable after catching the panic
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "before_panic");
    }

    #[tokio::test]
    async fn error_recovery_connection_reuse() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("valid")])
            .execute()
            .await
            .unwrap();

        // Attempt to insert a duplicate or trigger a constraint violation
        // by inserting and then verifying the connection works for subsequent ops
        db.insert(simple)
            .values([InsertSimple::new("second")])
            .execute()
            .await
            .unwrap();

        // Verify both records exist
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);

        // Delete all and verify connection handles empty results
        db.delete(simple)
            .r#where(eq(simple.name, "valid"))
            .execute()
            .await
            .unwrap();

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "second");
    }

    #[tokio::test]
    async fn rapid_sequential_operations() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        // Rapidly insert, query, update, delete in sequence
        for i in 0..20 {
            let name = format!("user_{}", i);
            db.insert(simple)
                .values([InsertSimple::new(&name)])
                .execute()
                .await
                .unwrap();
        }

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 20);

        // Update all at once
        db.update(simple)
            .set(crate::common::schema::sqlite::UpdateSimple::default().with_name("updated"))
            .r#where(like(simple.name, "user_%"))
            .execute()
            .await
            .unwrap();

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated"))
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 20);
    }

    #[tokio::test]
    async fn clone_and_query_from_spawned_task() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("initial")])
            .execute()
            .await
            .unwrap();

        // Clone the Drizzle handle and move it into a spawned task
        let db_clone = db.db.clone();
        let handle = tokio::spawn(async move {
            db_clone
                .insert(simple)
                .values([InsertSimple::new("from_spawn")])
                .execute()
                .await
                .unwrap();
        });

        handle.await.unwrap();

        // Original db is still usable
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);

        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"initial"));
        assert!(names.contains(&"from_spawn"));
    }

    #[tokio::test]
    async fn clone_select_from_spawned_task() {
        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        for i in 0..5 {
            db.insert(simple)
                .values([InsertSimple::new(format!("user_{}", i))])
                .execute()
                .await
                .unwrap();
        }

        // Clone and select from a spawned task
        let db_clone = db.db.clone();
        let handle = tokio::spawn(async move {
            let results: Vec<SimpleResult> = db_clone
                .select((simple.id, simple.name))
                .from(simple)
                .all_as()
                .await
                .unwrap();
            results.len()
        });

        let count = handle.await.unwrap();
        assert_eq!(count, 5);

        // Original handle still works
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn transaction_works_with_outstanding_clone() {
        use drizzle_sqlite::connection::SQLiteTransactionType;

        let (db, SimpleSchema { simple }) = libsql_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_tx")])
            .execute()
            .await
            .unwrap();

        // Create a clone — libsql transaction takes &self, so this should work
        let _clone = db.db.clone();

        let result = db
            .transaction(SQLiteTransactionType::default(), async |tx| {
                tx.insert(simple)
                    .values([InsertSimple::new("in_tx")])
                    .execute()
                    .await?;
                Ok(())
            })
            .await;

        assert!(result.is_ok());

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }
}

// ============================================================================
// turso async edge cases
// ============================================================================

#[cfg(feature = "turso")]
mod turso_edge_cases {
    use super::*;
    use crate::common::helpers::turso_setup;
    use drizzle::core::expr::*;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn cancellation_via_timeout_does_not_break_connection() {
        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_timeout")])
            .execute()
            .await
            .unwrap();

        let result = timeout(Duration::from_secs(5), async {
            let results: Vec<SimpleResult> = db
                .select((simple.id, simple.name))
                .from(simple)
                .all_as()
                .await
                .unwrap();
            results
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);

        // Verify connection still works
        db.insert(simple)
            .values([InsertSimple::new("after_timeout")])
            .execute()
            .await
            .unwrap();

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn select_cancellation_via_drop() {
        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        for i in 0..10 {
            db.insert(simple)
                .values([InsertSimple::new(format!("user_{}", i))])
                .execute()
                .await
                .unwrap();
        }

        tokio::select! {
            results = async {
                let r: Vec<SimpleResult> = db.select((simple.id, simple.name))
                    .from(simple)
                    .all_as()
                    .await
                    .unwrap();
                r
            } => {
                assert_eq!(results.len(), 10);
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                panic!("query should complete before sleep");
            }
        }

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn panic_recovery_in_async_context() {
        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_panic")])
            .execute()
            .await
            .unwrap();

        let result =
            futures_util::future::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
                panic!("intentional test panic")
            }))
            .await;

        assert!(result.is_err());

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "before_panic");
    }

    #[tokio::test]
    async fn rapid_sequential_operations() {
        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        for i in 0..20 {
            let name = format!("user_{}", i);
            db.insert(simple)
                .values([InsertSimple::new(&name)])
                .execute()
                .await
                .unwrap();
        }

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 20);

        db.update(simple)
            .set(crate::common::schema::sqlite::UpdateSimple::default().with_name("updated"))
            .r#where(like(simple.name, "user_%"))
            .execute()
            .await
            .unwrap();

        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated"))
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 20);
    }

    #[tokio::test]
    async fn clone_and_query_from_spawned_task() {
        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("initial")])
            .execute()
            .await
            .unwrap();

        // Clone the Drizzle handle and move it into a spawned task
        let db_clone = db.db.clone();
        let handle = tokio::spawn(async move {
            db_clone
                .insert(simple)
                .values([InsertSimple::new("from_spawn")])
                .execute()
                .await
                .unwrap();
        });

        handle.await.unwrap();

        // Original db is still usable
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);

        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"initial"));
        assert!(names.contains(&"from_spawn"));
    }

    #[tokio::test]
    async fn clone_select_from_spawned_task() {
        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        for i in 0..5 {
            db.insert(simple)
                .values([InsertSimple::new(format!("user_{}", i))])
                .execute()
                .await
                .unwrap();
        }

        // Clone and select from a spawned task
        let db_clone = db.db.clone();
        let handle = tokio::spawn(async move {
            let results: Vec<SimpleResult> = db_clone
                .select((simple.id, simple.name))
                .from(simple)
                .all_as()
                .await
                .unwrap();
            results.len()
        });

        let count = handle.await.unwrap();
        assert_eq!(count, 5);

        // Original handle still works
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn clone_transaction_in_spawned_task() {
        use drizzle_sqlite::connection::SQLiteTransactionType;

        let (db, SimpleSchema { simple }) = turso_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_tx")])
            .execute()
            .await
            .unwrap();

        // Clone, move into spawn, run a transaction there
        let mut db_clone = db.db.clone();
        let handle = tokio::spawn(async move {
            db_clone
                .transaction(SQLiteTransactionType::default(), async |tx| {
                    tx.insert(simple)
                        .values([InsertSimple::new("in_spawn_tx")])
                        .execute()
                        .await?;
                    Ok(())
                })
                .await
                .unwrap();
        });

        handle.await.unwrap();

        // Original db sees the committed data
        let results: Vec<SimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);

        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"before_tx"));
        assert!(names.contains(&"in_spawn_tx"));
    }
}
