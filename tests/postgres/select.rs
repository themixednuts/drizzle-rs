//! PostgreSQL SELECT query tests
//!
//! Tests for SELECT statement generation with PostgreSQL-specific syntax.

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;
use drizzle_core::OrderBy;

#[test]
fn test_simple_select() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((simple.id, simple.name))
        .from(simple);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Simple select SQL: {}", sql_string);

    assert!(sql_string.contains("SELECT"));
    assert!(sql_string.contains(r#""pg_simple"."id""#));
    assert!(sql_string.contains(r#""pg_simple"."name""#));
    assert!(sql_string.contains(r#"FROM "pg_simple""#));
}

#[test]
fn test_select_all_columns() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select all SQL: {}", sql_string);

    // Should include all columns from the table
    assert!(sql_string.contains(r#""pg_simple"."id""#));
    assert!(sql_string.contains(r#""pg_simple"."name""#));
}

#[test]
fn test_select_with_where() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "test"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with WHERE SQL: {}", sql_string);

    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains(r#""pg_simple"."name""#));
    // PostgreSQL uses $1 for parameters
    assert!(sql_string.contains("$1"), "Expected PostgreSQL $1 placeholder: {}", sql_string);
}

#[test]
fn test_select_with_order_by() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(10);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Order by SQL: {}", sql_string);

    assert!(sql_string.contains("ORDER BY"));
    assert!(sql_string.contains(r#""pg_simple"."name""#));
    assert!(sql_string.contains("ASC"));
    assert!(sql_string.contains("LIMIT"));
}

#[test]
fn test_select_with_limit() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((simple.id, simple.name))
        .from(simple)
        .limit(10);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with LIMIT SQL: {}", sql_string);

    assert!(sql_string.contains("LIMIT"));
}

#[test]
fn test_select_with_offset() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((simple.id, simple.name))
        .from(simple)
        .limit(10)
        .offset(20);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with LIMIT and OFFSET SQL: {}", sql_string);

    assert!(sql_string.contains("LIMIT"));
    assert!(sql_string.contains("OFFSET"));
}

#[test]
fn test_select_with_multiple_order_by() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .order_by([OrderBy::desc(complex.age), OrderBy::asc(complex.name)]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with multiple ORDER BY SQL: {}", sql_string);

    assert!(sql_string.contains("ORDER BY"));
    assert!(sql_string.contains("DESC"));
    assert!(sql_string.contains("ASC"));
}

#[test]
fn test_select_with_in_array() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(in_array(simple.name, ["Alice", "Bob", "Charlie"]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with IN SQL: {}", sql_string);

    assert!(sql_string.contains("IN"));
    // Should have PostgreSQL numbered placeholders
    assert!(sql_string.contains("$1"));
}

#[test]
fn test_select_with_like_pattern() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(simple)
        .r#where(like(simple.name, "%test%"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with LIKE SQL: {}", sql_string);

    assert!(sql_string.contains("LIKE"));
    assert!(sql_string.contains("$1"));
}

#[test]
fn test_select_with_null_check() {
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
fn test_select_with_between() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(between(complex.age, 18, 65));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with BETWEEN SQL: {}", sql_string);

    assert!(sql_string.contains("BETWEEN"));
    assert!(sql_string.contains("$1"));
    assert!(sql_string.contains("$2"));
}

#[test]
fn test_select_with_enum_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(eq(complex.role, PgRole::Admin));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with enum condition SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_complex"."role""#));
    assert!(sql_string.contains("$1"));
}

#[test]
fn test_select_complex_where() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .r#where(and([
            eq(complex.active, true),
            or([eq(complex.role, PgRole::Admin), gt(complex.age, 21)]),
        ]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Complex WHERE SQL: {}", sql_string);

    assert!(sql_string.contains("AND"));
    assert!(sql_string.contains("OR"));
}

#[test]
fn test_select_with_aggregate_count() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(alias(count(simple.id), "count"))
        .from(simple);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with COUNT SQL: {}", sql_string);

    assert!(sql_string.contains("COUNT"));
}

#[test]
fn test_select_with_aggregate_sum() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(alias(sum(complex.age), "total_age"))
        .from(complex);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with SUM SQL: {}", sql_string);

    assert!(sql_string.contains("SUM"));
}

#[test]
fn test_select_with_aggregate_avg() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(alias(avg(complex.score), "avg_score"))
        .from(complex);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with AVG SQL: {}", sql_string);

    assert!(sql_string.contains("AVG"));
}

#[test]
fn test_select_with_aggregate_min_max() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((
            alias(min(complex.age), "min_age"),
            alias(max(complex.age), "max_age"),
        ))
        .from(complex);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with MIN/MAX SQL: {}", sql_string);

    assert!(sql_string.contains("MIN"));
    assert!(sql_string.contains("MAX"));
}

#[test]
fn test_select_distinct() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(alias(distinct(complex.role), "role"))
        .from(complex);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select DISTINCT SQL: {}", sql_string);

    assert!(sql_string.contains("DISTINCT"));
}

#[test]
fn test_select_with_alias() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(alias(simple.name, "user_name"))
        .from(simple);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with alias SQL: {}", sql_string);

    assert!(sql_string.contains("AS"));
}

#[test]
fn test_select_with_coalesce() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(alias(coalesce(complex.email, "unknown@example.com"), "email"))
        .from(complex);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select with COALESCE SQL: {}", sql_string);

    assert!(sql_string.contains("COALESCE"));
}

#[test]
fn test_select_multiple_tables_columns() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Select from multiple tables SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_complex"."name""#));
    assert!(sql_string.contains(r#""pg_posts"."title""#));
}
