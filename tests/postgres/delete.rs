//! PostgreSQL DELETE statement tests
//!
//! Tests for DELETE statement generation with PostgreSQL-specific syntax.

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_simple_delete_sql_generation() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(simple)
        .r#where(eq(PgSimple::id, 1));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Simple delete SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains(r#""pg_simple""#));
    assert!(sql_string.contains("WHERE"));
    // PostgreSQL uses $1, $2, etc. for parameters
    assert!(
        sql_string.contains("$1"),
        "Should use PostgreSQL numbered placeholders: {}",
        sql_string
    );
}

#[test]
fn test_delete_with_complex_where() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(and([eq(complex.active, false), is_null(complex.email)]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with complex WHERE SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("AND"));
}

#[test]
fn test_delete_with_or_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(simple)
        .r#where(or([eq(simple.name, "Alice"), eq(simple.name, "Bob")]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with OR condition SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("OR"));
}

#[test]
fn test_delete_with_in_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(simple)
        .r#where(in_array(simple.id, [1, 2, 3, 4, 5]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with IN condition SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("IN"));
}

#[test]
fn test_delete_with_like_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(simple)
        .r#where(like(simple.name, "test%"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with LIKE condition SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("LIKE"));
}

#[test]
fn test_delete_with_comparison_operators() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    // Test gt
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(gt(complex.age, 65));
    let sql_string = query.to_sql().sql();
    println!("Delete with > condition SQL: {}", sql_string);
    assert!(sql_string.contains(">"));

    // Test lt
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(lt(complex.age, 18));
    let sql_string = query.to_sql().sql();
    println!("Delete with < condition SQL: {}", sql_string);
    assert!(sql_string.contains("<"));

    // Test gte
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(gte(complex.age, 21));
    let sql_string = query.to_sql().sql();
    println!("Delete with >= condition SQL: {}", sql_string);
    assert!(sql_string.contains(">="));

    // Test lte
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(lte(complex.age, 30));
    let sql_string = query.to_sql().sql();
    println!("Delete with <= condition SQL: {}", sql_string);
    assert!(sql_string.contains("<="));
}

#[test]
fn test_delete_with_is_null() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(is_null(complex.email));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with IS NULL SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("IS NULL"));
}

#[test]
fn test_delete_with_is_not_null() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(is_not_null(complex.email));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with IS NOT NULL SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("IS NOT NULL"));
}

#[test]
fn test_delete_with_between() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(between(complex.age, 18, 65));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with BETWEEN SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("BETWEEN"));
}

#[test]
fn test_delete_with_not_between() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(not_between(complex.age, 18, 65));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with NOT BETWEEN SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("NOT BETWEEN"));
}

#[test]
fn test_delete_with_nested_conditions() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(and([
            or([eq(complex.role, PgRole::Admin), eq(complex.role, PgRole::Moderator)]),
            eq(complex.active, false),
        ]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with nested conditions SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("AND"));
    assert!(sql_string.contains("OR"));
}

#[test]
fn test_delete_by_enum() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(eq(complex.role, PgRole::User));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete by enum SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains(r#""pg_complex"."role""#));
}

#[cfg(feature = "uuid")]
#[test]
fn test_delete_by_uuid() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let id = uuid::Uuid::new_v4();
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(complex)
        .r#where(eq(complex.id, id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete by UUID SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains(r#""pg_complex"."id""#));
}

#[test]
fn test_delete_post_by_foreign_key() {
    let PgComplexPostSchema { post, .. } = PgComplexPostSchema::new();

    let author_id = uuid::Uuid::new_v4();
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(post)
        .r#where(eq(post.author_id, author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete by foreign key SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains(r#""pg_posts""#));
}

#[test]
fn test_delete_task_by_status() {
    let PgTaskSchema { task, .. } = PgTaskSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(task)
        .r#where(eq(task.status, PostStatus::Archived));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete task by status SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains(r#""pg_tasks""#));
}

#[test]
fn test_delete_returning() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(simple)
        .r#where(eq(PgSimple::id, 1))
        .returning(simple.id);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with RETURNING SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("RETURNING"));
    assert!(sql_string.contains(r#""pg_simple"."id""#));
}

#[test]
fn test_delete_returning_all() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .delete(simple)
        .r#where(eq(PgSimple::id, 1))
        .returning(());

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Delete with RETURNING * SQL: {}", sql_string);

    assert!(sql_string.contains("DELETE FROM"));
    assert!(sql_string.contains("RETURNING"));
}
