//! PostgreSQL array operator tests
//!
//! Tests for PostgreSQL-specific array operators (@>, <@, &&).

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::postgres::expr::{array_contained, array_contains, array_overlaps};
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

// Test SQL generation for array_contains (@>) operator
postgres_test!(array_contains_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Note: This test verifies SQL generation. The simple.name column is TEXT,
    // not an array type, but the SQL generation should still work correctly.
    // In production, this would be used with actual TEXT[] columns.
    let stmt = db
        .select(())
        .from(simple)
        .r#where(array_contains(simple.name, "test"));

    let sql = stmt.to_sql().sql();

    // Verify the @> operator is present in the generated SQL
    assert!(sql.contains("@>"), "Expected @> operator in SQL: {}", sql);
});

// Test SQL generation for array_contained (<@) operator
postgres_test!(array_contained_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(array_contained(simple.name, "test"));

    let sql = stmt.to_sql().sql();

    // Verify the <@ operator is present in the generated SQL
    assert!(sql.contains("<@"), "Expected <@ operator in SQL: {}", sql);
});

// Test SQL generation for array_overlaps (&&) operator
postgres_test!(array_overlaps_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(array_overlaps(simple.name, "test"));

    let sql = stmt.to_sql().sql();

    // Verify the && operator is present in the generated SQL
    assert!(sql.contains("&&"), "Expected && operator in SQL: {}", sql);
});

// Test that array operators work with method syntax via ArrayExprExt trait
postgres_test!(array_ops_method_syntax, SimpleSchema, {
    use drizzle::postgres::expr::ArrayExprExt;

    let SimpleSchema { simple } = schema;

    // Test method syntax for array_contains
    let stmt = db
        .select(())
        .from(simple)
        .r#where(simple.name.array_contains("test"));

    let sql = stmt.to_sql().sql();
    assert!(sql.contains("@>"), "Expected @> operator in SQL: {}", sql);

    // Test method syntax for array_contained
    let stmt = db
        .select(())
        .from(simple)
        .r#where(simple.name.array_contained("test"));

    let sql = stmt.to_sql().sql();
    assert!(sql.contains("<@"), "Expected <@ operator in SQL: {}", sql);

    // Test method syntax for array_overlaps
    let stmt = db
        .select(())
        .from(simple)
        .r#where(simple.name.array_overlaps("test"));

    let sql = stmt.to_sql().sql();
    assert!(sql.contains("&&"), "Expected && operator in SQL: {}", sql);
});
