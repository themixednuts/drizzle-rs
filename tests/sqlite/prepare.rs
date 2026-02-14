//! SQLite Prepared Statement Integration Tests
//!
//! Integration tests that verify prepared statement execution with SQLite databases.
//! Unit tests for SQL structure verification are in drizzle_sqlite::builder::prepared.

#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_core::SQL;
use drizzle_macros::sqlite_test;

use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};

sqlite_test!(test_prepare_with_placeholder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
            .execute()
    );

    // Create a prepared statement with placeholder
    let prepared = db
        .select(simple.name)
        .from(simple)
        .r#where(and([eq(simple.name, SQL::placeholder("name"))]))
        .prepare();

    #[derive(SQLiteFromRow, Default)]
    struct PartialSimple {
        name: String,
    }

    // Execute the prepared statement with bound parameter
    let result: Vec<PartialSimple> =
        drizzle_exec!(prepared.all(db.conn(), params![{name: "Alice"}]));

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
            .execute()
    );

    // Create a prepared statement once
    let prepared = db
        .select(simple.name)
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    #[derive(SQLiteFromRow, Default)]
    struct NameOnly {
        name: String,
    }

    // Execute with different parameter values
    let alice: Vec<NameOnly> = drizzle_exec!(prepared.all(db.conn(), params![{name: "Alice"}]));
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].name, "Alice");

    let bob: Vec<NameOnly> = drizzle_exec!(prepared.all(db.conn(), params![{name: "Bob"}]));
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].name, "Bob");

    let charlie: Vec<NameOnly> = drizzle_exec!(prepared.all(db.conn(), params![{name: "Charlie"}]));
    assert_eq!(charlie.len(), 1);
    assert_eq!(charlie[0].name, "Charlie");
});

sqlite_test!(test_prepared_get_single_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("UniqueUser")])
            .execute()
    );

    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare();

    // Use get to retrieve a single row
    let result: SelectSimple =
        drizzle_exec!(prepared.get(db.conn(), params![{name: "UniqueUser"}]));

    assert_eq!(result.name, "UniqueUser");
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
            .all()
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
            .execute()
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
            .execute()
    );

    // Create a prepared statement and convert to owned
    let owned = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    // Owned statement can be stored and reused
    let result: Vec<SelectSimple> =
        drizzle_exec!(owned.all(db.conn(), params![{name: "OwnedTest"}]));

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "OwnedTest");
});

sqlite_test!(test_prepared_performance_comparison, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data: Vec<_> = (0..1000)
        .map(|i| InsertSimple::new(format!("User{}", i)))
        .collect();
    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test regular query performance
    let start = std::time::Instant::now();
    for i in 0..100 {
        let _results: Vec<SelectSimple> = drizzle_exec!(
            db.select(())
                .from(simple)
                .r#where(eq(simple.name, format!("User{}", i)))
                .all()
        );
    }
    let regular_duration = start.elapsed();

    // Test prepared statement performance
    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    let start = std::time::Instant::now();
    for i in 0..100 {
        let _results: Vec<SelectSimple> =
            drizzle_exec!(prepared.all(db.conn(), params![{name: format!("User{}", i)}]));
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
    let results: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());

    assert_eq!(results.len(), 5);
    for i in 0..5 {
        assert!(results.iter().any(|r| r.name == format!("BatchUser{}", i)));
    }
});
