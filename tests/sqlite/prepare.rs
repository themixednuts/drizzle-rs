//! SQLite Prepared Statement Integration Tests
//!
//! Integration tests that verify prepared statement execution with SQLite databases.
//! Unit tests for SQL structure verification are in drizzle_sqlite::builder::prepared.

#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};

sqlite_test!(test_prepare_with_placeholder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
            => execute
    );

    // Create a typed placeholder from the column
    let name = simple.name.placeholder("name");

    // Create a prepared statement with placeholder
    let prepared = db
        .select(simple.name)
        .from(simple)
        .r#where(and([eq(simple.name, name)]))
        .prepare();

    #[derive(SQLiteFromRow, Default)]
    struct PartialSimple {
        name: String,
    }

    // Execute the prepared statement with bound parameter
    let result: Vec<PartialSimple> = drizzle_exec!(prepared.all(db.conn(), [name.bind("Alice")]));

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Alice");
});

sqlite_test!(test_prepare_reuse_with_different_params, SimpleSchema, {
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

    // Create a typed placeholder from the column
    let name = simple.name.placeholder("name");

    // Create a prepared statement once
    let prepared = db
        .select(simple.name)
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    #[derive(SQLiteFromRow, Default)]
    struct NameOnly {
        name: String,
    }

    // Execute with different parameter values
    let alice: Vec<NameOnly> = drizzle_exec!(prepared.all(db.conn(), [name.bind("Alice")]));
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].name, "Alice");

    let bob: Vec<NameOnly> = drizzle_exec!(prepared.all(db.conn(), [name.bind("Bob")]));
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].name, "Bob");

    let charlie: Vec<NameOnly> = drizzle_exec!(prepared.all(db.conn(), [name.bind("Charlie")]));
    assert_eq!(charlie.len(), 1);
    assert_eq!(charlie[0].name, "Charlie");
});

sqlite_test!(test_prepared_get_single_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("UniqueUser")])
            => execute
    );

    // Create a typed placeholder from the column
    let name = simple.name.placeholder("name");

    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare();

    // Use get to retrieve a single row
    let result: SelectSimple = drizzle_exec!(prepared.get(db.conn(), [name.bind("UniqueUser")]));

    assert_eq!(result.name, "UniqueUser");
});

sqlite_test!(test_prepared_missing_named_param_fails, SimpleSchema, {
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
        .select(())
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare();

    let result = drizzle_catch_unwind!(prepared.all::<SelectSimple, 0>(db.conn(), []));
    match result {
        Err(_) => {} // debug_assert panic — expected in debug builds
        Ok(Err(drizzle::error::DrizzleError::ParameterError(_))) => {} // bind error — expected in release builds
        other => panic!("expected param mismatch failure, got: {other:?}"),
    }
});

sqlite_test!(test_prepared_extra_named_param_fails, SimpleSchema, {
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
        .select(())
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare();

    let result = drizzle_catch_unwind!(
        prepared.all::<SelectSimple, 2>(db.conn(), [name.bind("Alice"), extra.bind("ignored")],)
    );
    match result {
        Err(_) => {} // debug_assert panic — expected in debug builds
        Ok(Err(drizzle::error::DrizzleError::ParameterError(_))) => {} // bind error — expected in release builds
        other => panic!("expected param mismatch failure, got: {other:?}"),
    }
});

sqlite_test!(test_prepared_execute_insert, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Prepare an insert with values baked in
    let insert_data = InsertSimple::new("PreparedInsert");
    let prepared = db.insert(simple).values([insert_data]).prepare();

    // Execute the prepared insert
    drizzle_exec!(prepared.execute(db.conn(), []));

    // Verify the insert worked
    let results: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(eq(simple.name, "PreparedInsert"))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "PreparedInsert");
});

sqlite_test!(test_prepared_select_all_no_params, SimpleSchema, {
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

    // Prepared statement without placeholders
    let prepared = db.select(()).from(simple).prepare();

    let results: Vec<SelectSimple> = drizzle_exec!(prepared.all(db.conn(), []));

    assert_eq!(results.len(), 3);
});

sqlite_test!(test_prepared_owned_conversion, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("OwnedTest")])
            => execute
    );

    // Create a typed placeholder from the column
    let name = simple.name.placeholder("name");

    // Create a prepared statement and convert to owned
    let owned = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    // Owned statement can be stored and reused
    let result: Vec<SelectSimple> = drizzle_exec!(owned.all(db.conn(), [name.bind("OwnedTest")]));

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "OwnedTest");
});

sqlite_test!(test_prepared_performance_comparison, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data: Vec<_> = (0..1000)
        .map(|i| InsertSimple::new(format!("User{}", i)))
        .collect();
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test regular query performance
    let start = std::time::Instant::now();
    for i in 0..100 {
        let _results: Vec<SelectSimple> = drizzle_exec!(
            db.select(())
                .from(simple)
                .r#where(eq(simple.name, format!("User{}", i)))
                => all
        );
    }
    let regular_duration = start.elapsed();

    // Create a typed placeholder from the column
    let name = simple.name.placeholder("name");

    // Test prepared statement performance
    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, name))
        .prepare()
        .into_owned();

    let start = std::time::Instant::now();
    for i in 0..100 {
        let _results: Vec<SelectSimple> =
            drizzle_exec!(prepared.all(db.conn(), [name.bind(format!("User{}", i))]));
    }
    let prepared_duration = start.elapsed();

    // Prepared statements should generally be faster for repeated queries
    assert!(
        prepared_duration <= regular_duration * 2,
        "Prepared statements shouldn't be significantly slower"
    );
});

sqlite_test!(test_prepared_insert_multiple_times, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Create prepared inserts with different values
    for i in 0..5 {
        let insert_data = InsertSimple::new(format!("BatchUser{}", i));
        let prepared = db.insert(simple).values([insert_data]).prepare();
        drizzle_exec!(prepared.execute(db.conn(), []));
    }

    // Verify all inserts worked
    let results: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);

    assert_eq!(results.len(), 5);
    for i in 0..5 {
        assert!(results.iter().any(|r| r.name == format!("BatchUser{}", i)));
    }
});
