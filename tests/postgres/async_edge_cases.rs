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
                .all()
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
            .all()
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
                    .all()
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
            .all()
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
            .all()
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
            .all()
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
            .all()
            .await
            .unwrap();
        assert_eq!(results.len(), 20);
    }
}
