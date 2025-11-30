//! PostgreSQL condition expression tests
//!
//! These tests verify that condition expressions are properly generated
//! with correct PostgreSQL syntax.

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_eq_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(eq(simple.name, "test"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("EQ condition SQL: {}", sql_string);

    assert!(sql_string.contains("="));
    // PostgreSQL uses $1, $2, etc. for parameters
    assert!(sql_string.contains("$1"), "Should use PostgreSQL $1 placeholder: {}", sql_string);
}

#[test]
fn test_neq_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(neq(simple.name, "test"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("NEQ condition SQL: {}", sql_string);

    assert!(sql_string.contains("<>") || sql_string.contains("!="));
}

#[test]
fn test_gt_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(gt(complex.age, 25));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("GT condition SQL: {}", sql_string);

    assert!(sql_string.contains(">"));
    assert!(!sql_string.contains(">="));
}

#[test]
fn test_gte_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(gte(complex.age, 25));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("GTE condition SQL: {}", sql_string);

    assert!(sql_string.contains(">="));
}

#[test]
fn test_lt_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(lt(complex.age, 25));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("LT condition SQL: {}", sql_string);

    assert!(sql_string.contains("<"));
    assert!(!sql_string.contains("<="));
}

#[test]
fn test_lte_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(lte(complex.age, 25));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("LTE condition SQL: {}", sql_string);

    assert!(sql_string.contains("<="));
}

#[test]
fn test_in_array_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(in_array(simple.name, ["Alice", "Bob", "Charlie"]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("IN array condition SQL: {}", sql_string);

    assert!(sql_string.contains("IN"));
    assert!(sql_string.contains("$1"));
    assert!(sql_string.contains("$2"));
    assert!(sql_string.contains("$3"));
}

#[test]
fn test_not_in_array_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(not_in_array(simple.name, ["Alice", "Bob"]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("NOT IN array condition SQL: {}", sql_string);

    assert!(sql_string.contains("NOT IN"));
}

#[test]
fn test_is_null_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(is_null(complex.email));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("IS NULL condition SQL: {}", sql_string);

    assert!(sql_string.contains("IS NULL"));
}

#[test]
fn test_is_not_null_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(is_not_null(complex.email));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("IS NOT NULL condition SQL: {}", sql_string);

    assert!(sql_string.contains("IS NOT NULL"));
}

#[test]
fn test_like_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(like(simple.name, "%test%"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("LIKE condition SQL: {}", sql_string);

    assert!(sql_string.contains("LIKE"));
    assert!(sql_string.contains("$1"));
}

#[test]
fn test_not_like_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(not_like(simple.name, "%test%"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("NOT LIKE condition SQL: {}", sql_string);

    assert!(sql_string.contains("NOT LIKE"));
}

#[test]
fn test_between_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(between(complex.age, 18, 65));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("BETWEEN condition SQL: {}", sql_string);

    assert!(sql_string.contains("BETWEEN"));
    assert!(sql_string.contains("$1"));
    assert!(sql_string.contains("$2"));
}

#[test]
fn test_not_between_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(not_between(complex.age, 18, 65));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("NOT BETWEEN condition SQL: {}", sql_string);

    assert!(sql_string.contains("NOT BETWEEN"));
}

#[test]
fn test_and_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(and([eq(complex.active, true), gt(complex.age, 18)]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("AND condition SQL: {}", sql_string);

    assert!(sql_string.contains("AND"));
}

#[test]
fn test_or_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(or([eq(complex.role, PgRole::Admin), eq(complex.role, PgRole::Moderator)]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("OR condition SQL: {}", sql_string);

    assert!(sql_string.contains("OR"));
}

#[test]
fn test_not_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(not(eq(complex.active, true)));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("NOT condition SQL: {}", sql_string);

    assert!(sql_string.contains("NOT"));
}

#[test]
fn test_nested_and_or_conditions() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(and([
            or([eq(complex.role, PgRole::Admin), eq(complex.role, PgRole::Moderator)]),
            eq(complex.active, true),
            gt(complex.age, 21),
        ]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Nested AND/OR conditions SQL: {}", sql_string);

    assert!(sql_string.contains("AND"));
    assert!(sql_string.contains("OR"));
}

#[test]
fn test_enum_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(eq(complex.role, PgRole::Admin));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Enum condition SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_complex"."role""#));
    assert!(sql_string.contains("$1"));
}

#[test]
fn test_boolean_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(eq(complex.active, true));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Boolean condition SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_complex"."active""#));
}

#[test]
fn test_comparison_with_float() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(gt(complex.score, 90.5));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Float comparison SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_complex"."score""#));
    assert!(sql_string.contains(">"));
}

#[test]
fn test_between_with_float() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(between(complex.score, 0.0, 100.0));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Float BETWEEN SQL: {}", sql_string);

    assert!(sql_string.contains("BETWEEN"));
}

#[test]
fn test_like_patterns() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    // Prefix pattern
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(like(simple.name, "test%"));
    println!("LIKE prefix pattern SQL: {}", query.to_sql().sql());
    assert!(query.to_sql().sql().contains("LIKE"));

    // Suffix pattern
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(like(simple.name, "%test"));
    println!("LIKE suffix pattern SQL: {}", query.to_sql().sql());
    assert!(query.to_sql().sql().contains("LIKE"));

    // Contains pattern
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(like(simple.name, "%test%"));
    println!("LIKE contains pattern SQL: {}", query.to_sql().sql());
    assert!(query.to_sql().sql().contains("LIKE"));
}

#[test]
fn test_empty_in_array() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let empty: Vec<&str> = vec![];
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(in_array(simple.name, empty));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Empty IN array SQL: {}", sql_string);

    // Empty IN should produce a valid SQL (usually FALSE or similar)
    assert!(sql_string.contains("WHERE"));
}

#[test]
fn test_single_condition_in_and() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(and([eq(simple.name, "test")]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Single condition in AND SQL: {}", sql_string);

    assert!(sql_string.contains("WHERE"));
}

#[test]
fn test_single_condition_in_or() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(or([eq(simple.name, "test")]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Single condition in OR SQL: {}", sql_string);

    assert!(sql_string.contains("WHERE"));
}
