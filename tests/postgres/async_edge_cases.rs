//! Async edge case tests for PostgreSQL (tokio-postgres)
//!
//! Tests cancellation safety, panic recovery, and error handling
//! in async contexts.

#![cfg(feature = "tokio-postgres")]

use crate::common::schema::postgres::{InsertSimple, SelectSimple, SimpleSchema};
use drizzle::core::expr::*;

#[cfg(feature = "tokio-postgres")]
mod tokio_postgres_edge_cases {
    use super::*;
    use drizzle::postgres::prelude::*;

    #[PostgresTable(name = "mismatch_source")]
    struct MismatchSource {
        #[column(primary)]
        id: i32,
        n: i32,
    }

    // `avg(integer)` is NUMERIC, which does not decode as `f64`.
    #[PostgresView(DEFINITION = "SELECT avg(n) AS mean FROM mismatch_source")]
    struct MismatchView {
        mean: Option<f64>,
    }

    #[derive(PostgresSchema)]
    struct MismatchSchema {
        mismatch_source: MismatchSource,
        mismatch_view: MismatchView,
    }

    async fn raw_row(sql: &str) -> tokio_postgres::Row {
        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
        });
        let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
            .await
            .expect("connect");
        tokio::spawn(connection);
        client.query_one(sql, &[]).await.expect("query")
    }

    // `get_column` used to read any value it had no probe for as NULL.
    #[cfg(feature = "chrono")]
    #[tokio::test]
    async fn get_column_reports_undecodable_values_instead_of_null() {
        use drizzle::postgres::traits::DrizzleRowByIndex;

        let row = raw_row("SELECT interval '1 day' AS present, NULL::interval AS absent").await;
        let present = DrizzleRowByIndex::get_column::<Option<chrono::Duration>>(&row, 0);
        assert!(
            present.is_err(),
            "a non-NULL interval must not decode as None: {present:?}"
        );
        let absent = DrizzleRowByIndex::get_column::<Option<chrono::Duration>>(&row, 1);
        assert!(matches!(absent, Ok(None)), "{absent:?}");
    }

    // With chrono and jiff both enabled, the chrono probe reads a timestamptz
    // first; a jiff target must still decode it.
    #[cfg(all(feature = "chrono", feature = "jiff"))]
    #[tokio::test]
    async fn get_column_reads_jiff_timestamps_alongside_chrono() {
        use drizzle::postgres::traits::DrizzleRowByIndex;

        let row = raw_row("SELECT timestamptz '2024-01-02 03:04:05+00' AS at").await;
        let at = DrizzleRowByIndex::get_column::<jiff::Timestamp>(&row, 0).expect("jiff timestamp");
        assert_eq!(at.to_string(), "2024-01-02T03:04:05Z");
    }

    // `[u8; N]` is bytes: the parameter's declared type decides whether they
    // are written as BYTEA or MACADDR, and either cell decodes back.
    #[tokio::test]
    async fn fixed_size_bytes_follow_the_declared_column_type() {
        use drizzle::core::FromDrizzleRow;
        use drizzle::postgres::values::PostgresValue;

        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
        });
        let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
            .await
            .expect("connect");
        tokio::spawn(connection);

        let mac = PostgresValue::from([1_u8, 2, 3, 4, 5, 6]);
        let raw = PostgresValue::from([7_u8, 8]);
        let row = client
            .query_one("SELECT $1::macaddr AS mac, $2::bytea AS raw", &[&mac, &raw])
            .await
            .expect("query");

        let mac = <[u8; 6] as FromDrizzleRow<tokio_postgres::Row>>::from_row_at(&row, 0).unwrap();
        assert_eq!(mac, [1, 2, 3, 4, 5, 6]);
        let raw = <[u8; 2] as FromDrizzleRow<tokio_postgres::Row>>::from_row_at(&row, 1).unwrap();
        assert_eq!(raw, [7, 8]);
        assert!(<[u8; 4] as FromDrizzleRow<tokio_postgres::Row>>::from_row_at(&row, 1).is_err());
    }

    #[tokio::test]
    async fn select_model_type_mismatch_is_an_error_not_a_panic() {
        let (
            db,
            MismatchSchema {
                mismatch_source,
                mismatch_view,
            },
        ) = tokio_postgres_setup::setup_db::<MismatchSchema>().await;

        db.insert(mismatch_source)
            .values([InsertMismatchSource::new(1, 3)])
            .execute()
            .await
            .unwrap();

        let result: drizzle::Result<Vec<SelectMismatchView>> =
            db.select(()).from(mismatch_view).all().await;
        let error = result.expect_err("NUMERIC must not decode as f64");
        assert!(
            error.to_string().contains("error deserializing column 0"),
            "unexpected error: {error}"
        );
    }
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
            let results: Vec<SelectSimple> = db
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

        let results: Vec<SelectSimple> = db
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
                let r: Vec<SelectSimple> = db.select((simple.id, simple.name))
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
        let results: Vec<SelectSimple> = db
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
        let results: Vec<SelectSimple> = db
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

        let results: Vec<SelectSimple> = db
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

        let results: Vec<SelectSimple> = db
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated"))
            .all()
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
        let results: Vec<SelectSimple> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all()
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
            let results: Vec<SelectSimple> = db_clone
                .select((simple.id, simple.name))
                .from(simple)
                .all()
                .await
                .unwrap();
            results.len()
        });

        let count = handle.await.unwrap();
        assert_eq!(count, 5);

        // Original handle still works
        let results: Vec<SelectSimple> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all()
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
        let results: Vec<SelectSimple> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all()
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

        let results: Vec<SelectSimple> = db
            .select((simple.id, simple.name))
            .from(simple)
            .all()
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
