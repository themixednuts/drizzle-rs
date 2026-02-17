//! Async edge case tests for PostgreSQL (tokio-postgres)
//!
//! Tests cancellation safety, panic recovery, and error handling
//! in async contexts.

#![cfg(feature = "tokio-postgres")]

use crate::common::schema::postgres::{InsertSimple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "tokio-postgres")]
mod tokio_postgres_edge_cases {
    use super::*;
    use crate::common::helpers::tokio_postgres_setup;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn cancellation_via_timeout_does_not_break_connection() {
        let (db, SimpleSchema { simple }) = tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_timeout")])
            .execute()
            .await
            .unwrap();

        let result = timeout(Duration::from_secs(5), async {
            let results: Vec<PgSimpleResult> = db
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

        let results: Vec<PgSimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn select_cancellation_via_drop() {
        let (db, SimpleSchema { simple }) = tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        for i in 0..10 {
            db.insert(simple)
                .values([InsertSimple::new(format!("user_{}", i))])
                .execute()
                .await
                .unwrap();
        }

        tokio::select! {
            results = async {
                let r: Vec<PgSimpleResult> = db.select((simple.id, simple.name))
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
        let results: Vec<PgSimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn panic_recovery_in_async_context() {
        let (db, SimpleSchema { simple }) = tokio_postgres_setup::setup_db::<SimpleSchema>().await;

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
        let results: Vec<PgSimpleResult> = db
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
        let (db, SimpleSchema { simple }) = tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        // Rapidly insert many records
        for i in 0..20 {
            let name = format!("user_{}", i);
            db.insert(simple)
                .values([InsertSimple::new(name.as_str())])
                .execute()
                .await
                .unwrap();
        }

        let results: Vec<PgSimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 20);

        // Update all at once
        db.update(simple)
            .set(crate::common::schema::postgres::UpdateSimple::default().with_name("updated"))
            .r#where(like(simple.name, "user_%"))
            .execute()
            .await
            .unwrap();

        let results: Vec<PgSimpleResult> = db
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
        let (db, SimpleSchema { simple }) = tokio_postgres_setup::setup_db::<SimpleSchema>().await;

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
        let results: Vec<PgSimpleResult> = db
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
        let (db, SimpleSchema { simple }) = tokio_postgres_setup::setup_db::<SimpleSchema>().await;

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
            let results: Vec<PgSimpleResult> = db_clone
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
        let results: Vec<PgSimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn transaction_fails_with_outstanding_clones() {
        use drizzle_postgres::common::PostgresTransactionType;

        let (mut db, SimpleSchema { simple }) =
            tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_tx")])
            .execute()
            .await
            .unwrap();

        // Create an outstanding clone
        let _clone = db.db.clone();

        // Transaction should fail because a clone exists
        let result = db
            .transaction(PostgresTransactionType::default(), async |tx| {
                tx.insert(simple)
                    .values([InsertSimple::new("in_tx")])
                    .execute()
                    .await?;
                Ok(())
            })
            .await;

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("outstanding"),
            "error should mention outstanding clones: {}",
            err_msg
        );

        // Original data should still be intact
        let results: Vec<PgSimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "before_tx");
    }

    #[tokio::test]
    async fn transaction_succeeds_after_clone_dropped() {
        use drizzle_postgres::common::PostgresTransactionType;

        let (mut db, SimpleSchema { simple }) =
            tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        db.insert(simple)
            .values([InsertSimple::new("before_tx")])
            .execute()
            .await
            .unwrap();

        // Clone, use in spawn, await completion (clone is dropped)
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

        // Now transaction should succeed — clone was dropped
        let result = db
            .transaction(PostgresTransactionType::default(), async |tx| {
                tx.insert(simple)
                    .values([InsertSimple::new("in_tx")])
                    .execute()
                    .await?;
                Ok(())
            })
            .await;

        assert!(result.is_ok());

        let results: Vec<PgSimpleResult> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all_as()
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn conn_mut_returns_none_when_shared() {
        let (mut db, SimpleSchema { simple: _ }) =
            tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        let _clone = db.db.clone();

        // conn_mut should return None while clone exists
        assert!(db.conn_mut().is_none());
    }

    #[tokio::test]
    async fn conn_mut_returns_some_when_unique() {
        let (mut db, SimpleSchema { simple: _ }) =
            tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        // No clones — conn_mut should return Some
        assert!(db.conn_mut().is_some());
    }

    #[tokio::test]
    async fn conn_mut_available_after_clone_dropped() {
        let (mut db, SimpleSchema { simple: _ }) =
            tokio_postgres_setup::setup_db::<SimpleSchema>().await;

        {
            let _clone = db.db.clone();
            assert!(db.conn_mut().is_none());
        }
        // Clone dropped — conn_mut should work again
        assert!(db.conn_mut().is_some());
    }
}
