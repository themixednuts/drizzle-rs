//! PostgreSQL DELETE statement tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[allow(dead_code)]
#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    active: bool,
}

postgres_test!(delete_single_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert data
    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")]);
    drizzle_exec!(stmt => execute);

    // Delete one row
    let stmt = db.delete(simple).r#where(eq(simple.name, "Alice"));
    drizzle_exec!(stmt => execute);

    // Verify deletion
    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
});

postgres_test!(delete_multiple_rows, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert data
    let stmt = db.insert(simple).values([
        InsertSimple::new("test_one"),
        InsertSimple::new("test_two"),
        InsertSimple::new("other"),
    ]);
    drizzle_exec!(stmt => execute);

    // Delete rows matching pattern
    let stmt = db.delete(simple).r#where(like(simple.name, "test%"));
    drizzle_exec!(stmt => execute);

    // Verify only "other" remains
    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "other");
});

postgres_test!(delete_with_in_condition, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert data
    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
        InsertSimple::new("David"),
    ]);
    drizzle_exec!(stmt => execute);

    // Delete specific names
    let stmt = db
        .delete(simple)
        .r#where(in_array(simple.name, ["Alice", "Charlie"]));
    drizzle_exec!(stmt => execute);

    // Verify correct rows deleted
    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Bob"));
    assert!(names.contains(&"David"));
});

#[cfg(feature = "uuid")]
postgres_test!(delete_with_complex_where, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Insert data
    let stmt = db.insert(complex).values([
        InsertComplex::new("Active User", true, Role::User),
        InsertComplex::new("Inactive User", false, Role::User),
        InsertComplex::new("Active Admin", true, Role::Admin),
        InsertComplex::new("Inactive Admin", false, Role::Admin),
    ]);
    drizzle_exec!(stmt => execute);

    // Delete inactive users (not admins)
    let stmt = db.delete(complex).r#where(and([
        eq(complex.active, false),
        eq(complex.role, Role::User),
    ]));
    drizzle_exec!(stmt => execute);

    // Verify correct deletion
    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 3);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Active User"));
    assert!(names.contains(&"Active Admin"));
    assert!(names.contains(&"Inactive Admin"));
    assert!(!names.contains(&"Inactive User"));
});

#[cfg(feature = "uuid")]
postgres_test!(delete_with_null_check, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Insert data with email
    let stmt = db.insert(complex).values([
        InsertComplex::new("With Email", true, Role::User).with_email("test@example.com")
    ]);
    drizzle_exec!(stmt => execute);

    // Insert data without email (separate insert due to type state)
    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("No Email", true, Role::User)]);
    drizzle_exec!(stmt => execute);

    // Delete rows with NULL email
    let stmt = db.delete(complex).r#where(is_null(complex.email));
    drizzle_exec!(stmt => execute);

    // Verify only row with email remains
    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "With Email");
    assert!(results[0].email.is_some());
});

#[cfg(feature = "uuid")]
postgres_test!(delete_with_comparison, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Insert data with ages
    let stmt = db.insert(complex).values([
        InsertComplex::new("Young", true, Role::User).with_age(20),
        InsertComplex::new("Middle", true, Role::User).with_age(40),
        InsertComplex::new("Senior", true, Role::User).with_age(70),
    ]);
    drizzle_exec!(stmt => execute);

    // Delete users over 65
    let stmt = db.delete(complex).r#where(gt(complex.age, 65));
    drizzle_exec!(stmt => execute);

    // Verify deletion
    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Young"));
    assert!(names.contains(&"Middle"));
});

#[cfg(feature = "uuid")]
postgres_test!(delete_with_between, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Insert data
    let stmt = db.insert(complex).values([
        InsertComplex::new("Teen", true, Role::User).with_age(15),
        InsertComplex::new("Young Adult", true, Role::User).with_age(25),
        InsertComplex::new("Adult", true, Role::User).with_age(45),
        InsertComplex::new("Senior", true, Role::User).with_age(75),
    ]);
    drizzle_exec!(stmt => execute);

    // Delete ages between 20 and 50
    let stmt = db.delete(complex).r#where(between(complex.age, 20, 50));
    drizzle_exec!(stmt => execute);

    // Verify deletion
    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Teen"));
    assert!(names.contains(&"Senior"));
});

postgres_test!(delete_no_matching_rows, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert data
    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    // Delete non-existent row
    let stmt = db.delete(simple).r#where(eq(simple.name, "NonExistent"));
    drizzle_exec!(stmt => execute);

    // Verify data unchanged
    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

postgres_test!(delete_all_rows, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert data
    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
    ]);
    drizzle_exec!(stmt => execute);

    // Delete all rows (where 1=1 equivalent using LIKE '%')
    let stmt = db.delete(simple).r#where(like(simple.name, "%"));
    drizzle_exec!(stmt => execute);

    // Verify all deleted
    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 0);
});
