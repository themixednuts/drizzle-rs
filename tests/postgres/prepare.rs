//! PostgreSQL Prepared Statement tests
//!
//! Integration tests that verify prepared statement execution with PostgreSQL databases.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

postgres_test!(test_prepare_with_placeholder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
            => execute
    );

    // Create a prepared statement with typed placeholder and convert to owned to release borrow
    let name = simple.name.placeholder("name");
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    // Execute the prepared statement with bound parameter
    let result: Vec<SelectSimple> =
        drizzle_exec!(prepared.all(drizzle_client!(), [name.bind("Alice")]));

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Alice");
});

postgres_test!(test_prepare_reuse_with_different_params, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("Alice"),
                InsertSimple::new("Bob"),
                InsertSimple::new("Charlie")
            ])
            => execute
    );

    // Create a prepared statement once
    let name = simple.name.placeholder("name");
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    // Execute with different parameter values
    let alice: Vec<SelectSimple> =
        drizzle_exec!(prepared.all(drizzle_client!(), [name.bind("Alice")]));
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].name, "Alice");

    let bob: Vec<SelectSimple> = drizzle_exec!(prepared.all(drizzle_client!(), [name.bind("Bob")]));
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].name, "Bob");

    let charlie: Vec<SelectSimple> =
        drizzle_exec!(prepared.all(drizzle_client!(), [name.bind("Charlie")]));
    assert_eq!(charlie.len(), 1);
    assert_eq!(charlie[0].name, "Charlie");
});

postgres_test!(test_prepared_get_single_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("UniqueUser")])
            => execute
    );

    let name = simple.name.placeholder("name");
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    // Use get to retrieve a single row
    let result: SelectSimple =
        drizzle_exec!(prepared.get(drizzle_client!(), [name.bind("UniqueUser")]));

    assert_eq!(result.name, "UniqueUser");
});

postgres_test!(test_prepared_missing_named_param_fails, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice")])
            => execute
    );

    let name = simple.name.placeholder("name");
    // Passing 0 params to a query with 1 placeholder should fail:
    // - In debug builds: debug_assert panics on param count mismatch
    // - In release builds: bind() returns ParameterError
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepared.all::<SelectSimple, 0>(drizzle_client!(), [])
    }));
    match result {
        Err(_) => {} // debug_assert panic — expected in debug builds
        Ok(Err(drizzle::error::DrizzleError::ParameterError(_))) => {} // bind error — expected in release builds
        other => panic!("expected param mismatch failure, got: {other:?}"),
    }
});

postgres_test!(test_prepared_extra_named_param_fails, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice")])
            => execute
    );

    let name = simple.name.placeholder("name");
    let extra = simple.name.placeholder("extra");

    // Passing 2 params to a query with 1 placeholder should fail:
    // - In debug builds: debug_assert panics on param count mismatch
    // - In release builds: bind() returns ParameterError
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepared.all::<SelectSimple, 2>(
            drizzle_client!(),
            [name.bind("Alice"), extra.bind("ignored")],
        )
    }));
    match result {
        Err(_) => {} // debug_assert panic — expected in debug builds
        Ok(Err(drizzle::error::DrizzleError::ParameterError(_))) => {} // bind error — expected in release builds
        other => panic!("expected param mismatch failure, got: {other:?}"),
    }
});

postgres_test!(test_prepared_execute_insert, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Prepare an insert with values baked in and convert to owned
    let insert_data = InsertSimple::new("PreparedInsert");
    let prepared = db
        .insert(simple)
        .values([insert_data])
        .prepare()
        .into_owned();

    // Execute the prepared insert
    drizzle_exec!(prepared.execute(drizzle_client!(), []));

    // Verify the insert worked
    let results: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "PreparedInsert"))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "PreparedInsert");
});

postgres_test!(test_prepared_select_all_no_params, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("User1"),
                InsertSimple::new("User2"),
                InsertSimple::new("User3")
            ])
            => execute
    );

    // Prepared statement without placeholders - convert to owned
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .prepare()
        .into_owned();

    let results: Vec<SelectSimple> = drizzle_exec!(prepared.all(drizzle_client!(), []));

    assert_eq!(results.len(), 3);
});

postgres_test!(test_prepared_owned_conversion, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("OwnedTest")])
            => execute
    );

    // Create a prepared statement and convert to owned
    let name = simple.name.placeholder("name");
    let owned = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    // Owned statement can be stored and reused
    let result: Vec<SelectSimple> =
        drizzle_exec!(owned.all(drizzle_client!(), [name.bind("OwnedTest")]));

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "OwnedTest");
});

postgres_test!(test_prepared_performance_comparison, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data: Vec<_> = (0..100)
        .map(|i| InsertSimple::new(format!("User{}", i)))
        .collect();
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test regular query performance
    let start = std::time::Instant::now();
    for i in 0..10 {
        let _results: Vec<SelectSimple> = drizzle_exec!(
            db.select(())
                .from(simple)
                .r#where(eq(simple.name, format!("User{}", i)))
                => all
        );
    }
    let regular_duration = start.elapsed();

    // Test prepared statement performance
    let name = simple.name.placeholder("name");
    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    let start = std::time::Instant::now();
    for i in 0..10 {
        let _results: Vec<SelectSimple> =
            drizzle_exec!(prepared.all(drizzle_client!(), [name.bind(format!("User{}", i))]));
    }
    let prepared_duration = start.elapsed();

    // Prepared statements shouldn't be significantly slower
    assert!(
        prepared_duration <= regular_duration * 3,
        "Prepared statements shouldn't be significantly slower"
    );
});

postgres_test!(test_prepared_insert_multiple_times, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Create prepared inserts with different values
    for i in 0..5 {
        let insert_data = InsertSimple::new(format!("BatchUser{}", i));
        let prepared = db
            .insert(simple)
            .values([insert_data])
            .prepare()
            .into_owned();
        drizzle_exec!(prepared.execute(drizzle_client!(), []));
    }

    // Verify all inserts worked
    let results: Vec<SelectSimple> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple) => all);

    assert_eq!(results.len(), 5);
    for i in 0..5 {
        assert!(results.iter().any(|r| r.name == format!("BatchUser{}", i)));
    }
});
