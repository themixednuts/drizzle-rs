//! PostgreSQL schema tests
//!
//! Schema creation is implicitly tested by all other tests via db.create().
//! These tests verify various schema configurations work correctly.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::pg::*;
use drizzle::postgres::prelude::*;
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
}

postgres_test!(schema_simple_works, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertPgSimple::new("test")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

#[cfg(feature = "uuid")]
postgres_test!(schema_complex_works, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("test", true, PgRole::User)]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

#[cfg(feature = "uuid")]
postgres_test!(schema_with_enum_works, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    // Insert with different enum values to verify enum type was created
    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Admin User", true, PgRole::Admin),
        InsertPgComplex::new("Regular User", true, PgRole::User),
        InsertPgComplex::new("Mod User", true, PgRole::Moderator),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 3);
});

postgres_test!(schema_multiple_inserts, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    // Multiple separate inserts should work
    let stmt = db.insert(simple).values([InsertPgSimple::new("First")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.insert(simple).values([InsertPgSimple::new("Second")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.insert(simple).values([InsertPgSimple::new("Third")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 3);
});
