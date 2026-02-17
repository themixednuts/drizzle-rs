//! PostgreSQL Prepared Statement tests
//!
//! Integration tests that verify prepared statement execution with PostgreSQL databases.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_core::SQL;
use drizzle_macros::postgres_test;
use drizzle_postgres::{params, values::PostgresValue};

// =============================================================================
// Placeholder Indexing Tests (verify $1, $2, $3... are correct)
// =============================================================================

/// Test that multiple positional placeholders get correct sequential indices
#[test]
fn test_placeholder_indexing_sequential() {
    // Build a SQL with multiple positional placeholders
    let sql = SQL::<PostgresValue>::raw("SELECT * FROM users WHERE name = ")
        .append(SQL::param(PostgresValue::from("Alice")))
        .append(SQL::raw(" AND age > "))
        .append(SQL::param(PostgresValue::from(25i32)))
        .append(SQL::raw(" AND active = "))
        .append(SQL::param(PostgresValue::from(true)));

    let sql_string = sql.sql();

    // PostgreSQL should use $1, $2, $3 for positional placeholders
    assert!(
        sql_string.contains("$1"),
        "SQL should contain $1, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$2"),
        "SQL should contain $2, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$3"),
        "SQL should contain $3, got: {}",
        sql_string
    );

    // Verify the order is correct
    let idx1 = sql_string.find("$1").unwrap();
    let idx2 = sql_string.find("$2").unwrap();
    let idx3 = sql_string.find("$3").unwrap();
    assert!(
        idx1 < idx2 && idx2 < idx3,
        "Placeholders should appear in order: $1 at {}, $2 at {}, $3 at {}",
        idx1,
        idx2,
        idx3
    );
}

/// Test that named placeholders in PostgreSQL are rendered as $N (not :name)
/// PostgreSQL only supports $N style placeholders
#[test]
fn test_named_placeholder_rendering() {
    let sql = SQL::<PostgresValue>::raw("SELECT * FROM users WHERE name = ")
        .append(SQL::placeholder("user_name"))
        .append(SQL::raw(" AND id = "))
        .append(SQL::placeholder("user_id"));

    let sql_string = sql.sql();

    // PostgreSQL uses $N style even for named placeholders
    // The name is used for binding, not for SQL syntax
    assert!(
        sql_string.contains("$1"),
        "SQL should contain $1, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$2"),
        "SQL should contain $2, got: {}",
        sql_string
    );

    // PostgreSQL should NOT use :name style
    assert!(
        !sql_string.contains(":user_name"),
        "PostgreSQL should not use :name style placeholders"
    );
}

/// Test mixing named and positional placeholders - all become $N in PostgreSQL
#[test]
fn test_mixed_placeholder_styles() {
    let sql = SQL::<PostgresValue>::raw("SELECT * FROM users WHERE name = ")
        .append(SQL::placeholder("user_name")) // Named - becomes $1
        .append(SQL::raw(" AND age > "))
        .append(SQL::param(PostgresValue::from(25i32))) // Positional - becomes $2
        .append(SQL::raw(" AND status = "))
        .append(SQL::placeholder("status")); // Named - becomes $3

    let sql_string = sql.sql();

    // All placeholders in PostgreSQL use $N style
    assert!(
        sql_string.contains("$1"),
        "SQL should contain $1, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$2"),
        "SQL should contain $2, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$3"),
        "SQL should contain $3, got: {}",
        sql_string
    );

    // PostgreSQL should NOT use :name style
    assert!(
        !sql_string.contains(":user_name"),
        "PostgreSQL should not use :name style"
    );
    assert!(
        !sql_string.contains(":status"),
        "PostgreSQL should not use :name style"
    );
}

/// Test that many positional placeholders maintain correct indexing
#[test]
fn test_many_positional_placeholders() {
    let mut sql = SQL::<PostgresValue>::raw("SELECT * FROM data WHERE ");

    // Add 10 conditions with positional placeholders
    for i in 0..10 {
        if i > 0 {
            sql = sql.append(SQL::raw(" AND "));
        }
        sql = sql.append(SQL::raw(format!("col{} = ", i)));
        sql = sql.append(SQL::param(PostgresValue::from(i)));
    }

    let sql_string = sql.sql();

    // Verify all placeholders from $1 to $10 are present
    for i in 1..=10 {
        let placeholder = format!("${}", i);
        assert!(
            sql_string.contains(&placeholder),
            "SQL should contain {}, got: {}",
            placeholder,
            sql_string
        );
    }

    // Verify order by finding indices
    let mut last_idx = 0;
    for i in 1..=10 {
        let placeholder = format!("${}", i);
        let idx = sql_string.find(&placeholder).unwrap();
        assert!(
            idx > last_idx || i == 1,
            "${} should appear after ${}, found at {} vs {}",
            i,
            i - 1,
            idx,
            last_idx
        );
        last_idx = idx;
    }
}

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow, Default)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

postgres_test!(test_prepare_with_placeholder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")])
            => execute
    );

    // Create a prepared statement with placeholder and convert to owned to release borrow
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    // Execute the prepared statement with bound parameter
    let result: Vec<PgSimpleResult> =
        drizzle_exec!(prepared.all(drizzle_client!(), params![{name: "Alice"}]));

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
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    // Execute with different parameter values
    let alice: Vec<PgSimpleResult> =
        drizzle_exec!(prepared.all(drizzle_client!(), params![{name: "Alice"}]));
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].name, "Alice");

    let bob: Vec<PgSimpleResult> =
        drizzle_exec!(prepared.all(drizzle_client!(), params![{name: "Bob"}]));
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].name, "Bob");

    let charlie: Vec<PgSimpleResult> =
        drizzle_exec!(prepared.all(drizzle_client!(), params![{name: "Charlie"}]));
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

    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    // Use get to retrieve a single row
    let result: PgSimpleResult =
        drizzle_exec!(prepared.get(drizzle_client!(), params![{name: "UniqueUser"}]));

    assert_eq!(result.name, "UniqueUser");
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
    let results: Vec<PgSimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "PreparedInsert"))
            => all_as
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

    let results: Vec<PgSimpleResult> = drizzle_exec!(prepared.all(drizzle_client!(), []));

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
    let owned = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    // Owned statement can be stored and reused
    let result: Vec<PgSimpleResult> =
        drizzle_exec!(owned.all(drizzle_client!(), params![{name: "OwnedTest"}]));

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
                => all_as
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
    for i in 0..10 {
        let _results: Vec<SelectSimple> =
            drizzle_exec!(prepared.all(drizzle_client!(), params![{name: format!("User{}", i)}]));
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
    let results: Vec<PgSimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple) => all_as);

    assert_eq!(results.len(), 5);
    for i in 0..5 {
        assert!(results.iter().any(|r| r.name == format!("BatchUser{}", i)));
    }
});
