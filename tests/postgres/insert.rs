//! PostgreSQL INSERT statement tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::pg::*;
use drizzle::prelude::*;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    active: bool,
}

postgres_test!(insert_single_row, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertPgSimple::new("Alice")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
    assert!(results[0].id > 0, "ID should be auto-generated");
});

postgres_test!(insert_multiple_rows, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("Alice"),
        InsertPgSimple::new("Bob"),
        InsertPgSimple::new("Charlie"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 3);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Alice"));
    assert!(names.contains(&"Bob"));
    assert!(names.contains(&"Charlie"));
});

#[cfg(feature = "uuid")]
postgres_test!(insert_with_optional_fields, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("Alice", true, PgRole::Admin)
            .with_email("alice@example.com")
            .with_age(30)]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
    assert_eq!(results[0].email, Some("alice@example.com".to_string()));
    assert_eq!(results[0].age, Some(30));
    assert!(results[0].active);
});

#[cfg(feature = "uuid")]
postgres_test!(insert_with_null_fields, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("Bob", false, PgRole::User)]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
    assert_eq!(results[0].email, None);
    assert_eq!(results[0].age, None);
    assert!(!results[0].active);
});

postgres_test!(insert_special_characters, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("O'Brien"),
        InsertPgSimple::new("Hello \"World\""),
        InsertPgSimple::new("Line1\nLine2"),
        InsertPgSimple::new("Tab\there"),
        InsertPgSimple::new("Emoji 🎉"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 5);
    assert!(results.iter().any(|r| r.name == "O'Brien"));
    assert!(results.iter().any(|r| r.name == "Hello \"World\""));
    assert!(results.iter().any(|r| r.name == "Line1\nLine2"));
    assert!(results.iter().any(|r| r.name == "Tab\there"));
    assert!(results.iter().any(|r| r.name == "Emoji 🎉"));
});

#[cfg(feature = "uuid")]
postgres_test!(insert_with_custom_uuid, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let custom_id = uuid::Uuid::new_v4();
    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("CustomID", true, PgRole::User).with_id(custom_id)]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.id, custom_id));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, custom_id);
    assert_eq!(results[0].name, "CustomID");
});

postgres_test!(insert_large_batch, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    // Create a batch of 100 rows
    let names: Vec<String> = (0..100).map(|i| format!("User_{}", i)).collect();
    let rows: Vec<_> = names
        .iter()
        .map(|n| InsertPgSimple::new(n.as_str()))
        .collect();

    let stmt = db.insert(simple).values(rows);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 100);
});
