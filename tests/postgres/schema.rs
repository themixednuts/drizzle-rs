//! PostgreSQL schema tests

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_simple_schema_create_statements() {
    let schema = PgSimpleSchema::new();
    let statements = schema.create_statements();

    println!("Simple schema statements:");
    for stmt in &statements {
        println!("  {}", stmt);
    }

    assert!(!statements.is_empty());
    let has_simple_table = statements
        .iter()
        .any(|s| s.contains("CREATE TABLE") && s.contains("pg_simple"));
    assert!(has_simple_table, "Should have CREATE TABLE for pg_simple");
}

#[test]
fn test_simple_table_sql() {
    let simple = PgSimple::new();
    let sql = simple.sql();
    let sql_string = sql.sql();

    println!("Simple table SQL: {}", sql_string);

    assert!(sql_string.contains("CREATE TABLE"));
    assert!(sql_string.contains(r#""pg_simple""#));
    assert!(sql_string.contains("SERIAL"));
    assert!(sql_string.contains("PRIMARY KEY"));
}

#[test]
fn test_complex_schema_create_statements() {
    let schema = PgComplexSchema::new();
    let statements = schema.create_statements();

    println!("Complex schema statements:");
    for stmt in &statements {
        println!("  {}", stmt);
    }

    assert!(!statements.is_empty());
    let has_complex_table = statements
        .iter()
        .any(|s| s.contains("CREATE TABLE") && s.contains("pg_complex"));
    assert!(has_complex_table, "Should have CREATE TABLE for pg_complex");
    let has_enum = statements.iter().any(|s| s.contains("CREATE TYPE"));
    assert!(has_enum, "Should have CREATE TYPE for PgRole enum");
}

#[test]
fn test_complex_table_sql() {
    let complex = PgComplex::new();
    let sql = complex.sql();
    let sql_string = sql.sql();

    println!("Complex table SQL: {}", sql_string);

    assert!(sql_string.contains("CREATE TABLE"));
    assert!(sql_string.contains(r#""pg_complex""#));
    assert!(sql_string.contains("TEXT"));
    assert!(sql_string.contains("INTEGER"));
    assert!(sql_string.contains("BOOLEAN"));
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid_primary_key() {
    let complex = PgComplex::new();
    let sql = complex.sql();
    let sql_string = sql.sql();

    println!("UUID primary key SQL: {}", sql_string);

    assert!(sql_string.contains("UUID"));
    assert!(sql_string.contains("PRIMARY KEY"));
}

#[test]
fn test_full_blog_schema_order() {
    let schema = PgFullBlogSchema::new();
    let statements = schema.create_statements();

    println!("Full blog schema statements:");
    for (i, stmt) in statements.iter().enumerate() {
        println!("  {}: {}", i, stmt);
    }

    let enum_pos = statements.iter().position(|s| s.contains("CREATE TYPE"));
    let table_pos = statements.iter().position(|s| s.contains("CREATE TABLE"));

    if let (Some(enum_idx), Some(table_idx)) = (enum_pos, table_pos) {
        assert!(enum_idx < table_idx, "Enums should be created before tables");
    }
}

#[test]
fn test_task_schema_with_native_enums() {
    let schema = PgTaskSchema::new();
    let statements = schema.create_statements();

    println!("Task schema statements:");
    for stmt in &statements {
        println!("  {}", stmt);
    }

    let has_priority = statements.iter().any(|s| s.contains("Priority"));
    assert!(has_priority, "Should have Priority enum");

    let has_post_status = statements.iter().any(|s| s.contains("PostStatus"));
    assert!(has_post_status, "Should have PostStatus enum");
}

#[test]
fn test_schema_items_method() {
    let schema = PgSimpleSchema::new();
    let (_simple,) = schema.items();
    // Schema items method returns references to table instances
}

#[test]
fn test_schema_with_index() {
    let schema = PgSimpleWithIndexSchema::new();
    let statements = schema.create_statements();

    println!("Schema with index statements:");
    for (i, stmt) in statements.iter().enumerate() {
        println!("  {}: {}", i, stmt);
    }

    let has_table = statements.iter().any(|s| s.contains("CREATE TABLE"));
    assert!(has_table, "Should have CREATE TABLE statement");

    let has_index = statements.iter().any(|s| s.contains("INDEX"));
    assert!(has_index, "Should have CREATE INDEX statement");
}

#[test]
fn test_foreign_key_references() {
    let post = PgPost::new();
    let sql = post.sql();
    let sql_string = sql.sql();

    println!("Post table SQL: {}", sql_string);

    assert!(sql_string.contains("CREATE TABLE"));
    assert!(sql_string.contains(r#""pg_posts""#));
    assert!(sql_string.contains("REFERENCES"));
}
