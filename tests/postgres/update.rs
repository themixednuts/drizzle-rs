//! PostgreSQL UPDATE statement tests
//!
//! Tests for UPDATE statement generation with PostgreSQL-specific syntax.

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_simple_update_sql_generation() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(simple)
        .set(UpdatePgSimple::default().with_name("updated"))
        .r#where(eq(PgSimple::id, 1));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Simple update SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains(r#""pg_simple""#));
    assert!(sql_string.contains("SET"));
    assert!(sql_string.contains("WHERE"));
    // PostgreSQL uses $1, $2, etc. for parameters
    assert!(
        sql_string.contains("$1"),
        "Should use PostgreSQL numbered placeholders: {}",
        sql_string
    );
}

#[test]
fn test_update_multiple_columns() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(
            UpdatePgComplex::default()
                .with_email("new@example.com")
                .with_age(35)
                .with_description("Updated description"),
        )
        .r#where(eq(PgComplex::name, "Test User"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update multiple columns SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
    // Should have column = value format for each column
    assert!(sql_string.contains(r#""email""#));
    assert!(sql_string.contains(r#""age""#));
    assert!(sql_string.contains(r#""description""#));
}

#[test]
fn test_update_with_complex_where() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_active(false))
        .r#where(and([eq(complex.role, PgRole::User), lt(complex.age, 18)]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update with complex WHERE SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("AND"));
}

#[test]
fn test_update_with_or_condition() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_role(PgRole::Admin))
        .r#where(or([eq(complex.name, "Alice"), eq(complex.name, "Bob")]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update with OR condition SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("OR"));
}

#[test]
fn test_update_with_in_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(simple)
        .set(UpdatePgSimple::default().with_name("Batch Updated"))
        .r#where(in_array(simple.id, [1, 2, 3, 4, 5]));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update with IN condition SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("IN"));
}

#[test]
fn test_update_with_like_condition() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(simple)
        .set(UpdatePgSimple::default().with_name("Pattern Updated"))
        .r#where(like(simple.name, "Test%"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update with LIKE condition SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains("LIKE"));
}

#[test]
fn test_update_numeric_fields() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_age(30).with_score(85.5))
        .r#where(eq(PgComplex::name, "Test"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update numeric fields SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
}

#[test]
fn test_update_boolean_field() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_active(true))
        .r#where(eq(complex.active, false));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update boolean field SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
    assert!(sql_string.contains(r#""active""#));
}

#[test]
fn test_update_enum_field() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_role(PgRole::Moderator))
        .r#where(eq(complex.role, PgRole::User));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update enum field SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
    assert!(sql_string.contains(r#""role""#));
}

#[test]
fn test_update_task_with_native_enum() {
    let PgTaskSchema { task, .. } = PgTaskSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(task)
        .set(
            UpdatePgTask::default()
                .with_status(PostStatus::Published)
                .with_priority(Priority::High),
        )
        .r#where(eq(PgTask::id, 1));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update task with native enum SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains(r#""pg_tasks""#));
    assert!(sql_string.contains("SET"));
}

#[cfg(feature = "serde")]
#[test]
fn test_update_json_field() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let metadata_json = r#"{"theme":"light","notifications":false}"#;

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_metadata(metadata_json.to_string()))
        .r#where(eq(PgComplex::name, "Test"));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update JSON field SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("SET"));
}

#[cfg(feature = "uuid")]
#[test]
fn test_update_by_uuid() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let id = uuid::Uuid::new_v4();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(complex)
        .set(UpdatePgComplex::default().with_name("Updated by UUID"))
        .r#where(eq(complex.id, id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update by UUID SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("WHERE"));
}

#[test]
fn test_update_returning() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(simple)
        .set(UpdatePgSimple::default().with_name("returning_test"))
        .r#where(eq(PgSimple::id, 1))
        .returning(simple.id);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update with RETURNING SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("RETURNING"));
    assert!(sql_string.contains(r#""pg_simple"."id""#));
}

#[test]
fn test_update_returning_all() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .update(simple)
        .set(UpdatePgSimple::default().with_name("returning_all_test"))
        .r#where(eq(PgSimple::id, 1))
        .returning(());

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Update with RETURNING * SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains("RETURNING"));
}
