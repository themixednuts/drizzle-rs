//! PostgreSQL index tests
//!
//! Note: Index creation is tested via schema creation in db.create().
//! These tests verify queries work correctly (indexes improve performance but don't change results).

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::pg::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

// Test queries that would benefit from indexes
postgres_test!(query_by_name_column, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("Alice"),
        InsertPgSimple::new("Bob"),
        InsertPgSimple::new("Charlie"),
    ]);
    drizzle_exec!(stmt.execute());

    // Query by name (would use index if one existed)
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "Bob"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
});

#[cfg(feature = "uuid")]
postgres_test!(query_by_nullable_column, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    // Insert rows with and without email
    let stmt =
        db.insert(complex)
            .values([InsertPgComplex::new("With Email", true, PgRole::User)
                .with_email("test@example.com")]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("No Email", true, PgRole::User)]);
    drizzle_exec!(stmt.execute());

    #[derive(Debug, PostgresFromRow)]
    struct Result {
        name: String,
    }

    // Query using email column
    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.email, "test@example.com"));
    let results: Vec<Result> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "With Email");
});

postgres_test!(query_large_dataset, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    // Insert many rows
    let names: Vec<String> = (0..50).map(|i| format!("User_{:03}", i)).collect();
    let rows: Vec<_> = names
        .iter()
        .map(|n| InsertPgSimple::new(n.as_str()))
        .collect();
    let stmt = db.insert(simple).values(rows);
    drizzle_exec!(stmt.execute());

    // Query specific row (index would speed this up)
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "User_025"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "User_025");
});
