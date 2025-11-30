//! PostgreSQL INSERT statement tests
//!
//! Tests for INSERT statement generation with PostgreSQL-specific syntax.

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_simple_insert_sql_generation() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let data = InsertPgSimple::new("test_name");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(simple)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Simple insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_simple""#));
    assert!(sql_string.contains("VALUES"));
    // PostgreSQL uses $1, $2, etc. for parameters
    assert!(
        sql_string.contains("$1"),
        "Should use PostgreSQL numbered placeholders: {}",
        sql_string
    );
}

#[test]
fn test_complex_insert_sql_generation() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let data = InsertPgComplex::new("Complex User", true, PgRole::Admin)
        .with_email("admin@example.com")
        .with_age(30)
        .with_score(95.5)
        .with_description("An administrator");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(complex)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Complex insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_complex""#));
    assert!(sql_string.contains("VALUES"));
}

#[test]
fn test_insert_multiple_rows() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let data = vec![
        InsertPgSimple::new("row1"),
        InsertPgSimple::new("row2"),
        InsertPgSimple::new("row3"),
    ];

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(simple)
        .values(data);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Multiple rows insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains("VALUES"));
    // Should have multiple value groups
    let values_count = sql_string.matches("$").count();
    assert!(values_count >= 3, "Should have placeholders for 3 rows");
}

#[test]
fn test_insert_with_default_values() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let data = InsertPgSimple::new("with_defaults");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(simple)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with defaults SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
}

#[test]
fn test_insert_returning_clause() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let data = InsertPgSimple::new("returning_test");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(simple)
        .values([data])
        .returning(simple.id);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with RETURNING SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains("RETURNING"));
    assert!(sql_string.contains(r#""pg_simple"."id""#));
}

#[test]
fn test_insert_returning_all() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let data = InsertPgSimple::new("returning_all_test");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(simple)
        .values([data])
        .returning(());

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with RETURNING * SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains("RETURNING"));
}

#[test]
fn test_insert_on_conflict_do_nothing() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    let data = InsertPgSimple::new("conflict_test");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(simple)
        .values([data])
        .on_conflict_do_nothing();

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert ON CONFLICT DO NOTHING SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains("ON CONFLICT"));
    assert!(sql_string.contains("DO NOTHING"));
}

#[test]
fn test_insert_with_optional_fields() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    // Only set required fields
    let data = InsertPgComplex::new("minimal_user", true, PgRole::User);

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(complex)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with optional fields SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_complex""#));
}

#[test]
fn test_insert_with_all_optional_fields() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let data = InsertPgComplex::new("full_user", true, PgRole::Admin)
        .with_email("user@example.com")
        .with_age(25)
        .with_score(88.5)
        .with_description("A test user")
        .with_data_blob(vec![1, 2, 3, 4])
        .with_created_at("2024-01-01");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(complex)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with all fields SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
}

#[test]
fn test_insert_post_with_foreign_key() {
    let PgComplexPostSchema { post, .. } = PgComplexPostSchema::new();

    let author_id = uuid::Uuid::new_v4();
    let data = InsertPgPost::new("Test Post", true)
        .with_content("Post content")
        .with_author_id(author_id);

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(post)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with foreign key SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_posts""#));
}

#[test]
fn test_insert_category() {
    let PgCategorySchema { category } = PgCategorySchema::new();

    let data = InsertPgCategory::new("Technology").with_description("Tech related posts");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(category)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert category SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_categories""#));
}

#[test]
fn test_insert_task_with_native_enums() {
    let PgTaskSchema { task, .. } = PgTaskSchema::new();

    let data = InsertPgTask::new("Complete feature", Priority::High, PostStatus::Draft)
        .with_description("Implement the new feature");

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(task)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert task with native enums SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_tasks""#));
}

#[cfg(feature = "serde")]
#[test]
fn test_insert_with_json() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let metadata_json = r#"{"theme":"dark","notifications":true}"#;
    let config_json = r#"{"language":"en"}"#;

    let data = InsertPgComplex::new("json_user", true, PgRole::User)
        .with_metadata(metadata_json)
        .with_config(config_json);

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(complex)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with JSON SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
}

#[cfg(feature = "uuid")]
#[test]
fn test_insert_with_uuid() {
    let PgComplexSchema { complex, .. } = PgComplexSchema::new();

    let id = uuid::Uuid::new_v4();
    let data = InsertPgComplex::new("uuid_user", true, PgRole::User).with_id(id);

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .insert(complex)
        .values([data]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Insert with UUID SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
}
