//! PostgreSQL index tests

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_simple_index_sql() {
    let idx = PgComplexEmailIdx::new();
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Simple index SQL: {}", sql_string);

    assert!(sql_string.contains("CREATE"));
    assert!(sql_string.contains("INDEX"));
    assert!(sql_string.contains("pg_complex_email_idx"));
    assert!(sql_string.contains(r#""email""#));
}

#[test]
fn test_unique_index_sql() {
    let idx = PgSimpleNameIdx::new();
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Unique index SQL: {}", sql_string);

    assert!(sql_string.contains("CREATE"));
    assert!(sql_string.contains("UNIQUE"));
    assert!(sql_string.contains("INDEX"));
    assert!(sql_string.contains("pg_simple_name_idx"));
    assert!(sql_string.contains(r#""name""#));
}

#[test]
fn test_index_on_foreign_key() {
    let idx = PgPostAuthorIdx::new();
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Foreign key index SQL: {}", sql_string);

    assert!(sql_string.contains("CREATE"));
    assert!(sql_string.contains("INDEX"));
    assert!(sql_string.contains("pg_post_author_idx"));
    assert!(sql_string.contains(r#""author_id""#));
}

#[test]
fn test_schema_with_index_order() {
    let schema = PgSimpleWithIndexSchema::new();
    let statements = schema.create_statements();

    println!("Schema with index statements:");
    for (i, stmt) in statements.iter().enumerate() {
        println!("  {}: {}", i, stmt);
    }

    // Table should come before index
    let table_pos = statements
        .iter()
        .position(|s| s.contains("CREATE TABLE"))
        .expect("Should have CREATE TABLE");
    let index_pos = statements
        .iter()
        .position(|s| s.contains("INDEX"))
        .expect("Should have CREATE INDEX");

    assert!(
        table_pos < index_pos,
        "Table should be created before its indexes"
    );
}

#[test]
fn test_complex_schema_with_index() {
    let schema = PgComplexWithIndexSchema::new();
    let statements = schema.create_statements();

    println!("Complex schema with index statements:");
    for stmt in &statements {
        println!("  {}", stmt);
    }

    // Should have table and index
    let has_table = statements.iter().any(|s| s.contains("CREATE TABLE"));
    let has_index = statements.iter().any(|s| s.contains("INDEX"));

    assert!(has_table, "Should have CREATE TABLE");
    assert!(has_index, "Should have CREATE INDEX");
}

#[test]
fn test_index_table_reference() {
    let idx = PgSimpleNameIdx::new();
    
    // The index should reference the correct table
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Index with table reference: {}", sql_string);

    // Index should reference pg_simple table
    assert!(sql_string.contains("pg_simple") || sql_string.contains("pgsimple"));
}

#[test]
fn test_index_column_reference() {
    let idx = PgComplexEmailIdx::new();
    
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Index column reference: {}", sql_string);

    assert!(sql_string.contains("email"));
}

#[test]
fn test_index_name_convention() {
    let idx = PgSimpleNameIdx::new();
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Index name convention: {}", sql_string);

    // Should have a descriptive index name
    assert!(sql_string.contains("pg_simple_name_idx"));
}

#[test]
fn test_index_on_optional_column() {
    let idx = PgComplexEmailIdx::new();
    let sql = idx.sql();
    let sql_string = sql.sql();

    println!("Index on optional column: {}", sql_string);

    // Should work even for optional/nullable columns
    assert!(sql_string.contains("email"));
}

#[test]
fn test_multiple_indexes_in_schema() {
    let schema = PgComplexWithIndexSchema::new();
    let statements = schema.create_statements();

    let index_count = statements
        .iter()
        .filter(|s| s.contains("INDEX"))
        .count();

    println!(
        "Schema has {} index statements",
        index_count
    );

    // Should have at least one index
    assert!(index_count >= 1, "Should have at least one index");
}

#[test]
fn test_schema_creates_enum_before_table() {
    let schema = PgComplexSchema::new();
    let statements = schema.create_statements();

    println!("Full blog schema statements:");
    for stmt in &statements {
        println!("  {}", stmt);
    }

    // Find positions of enum and table
    let enum_pos = statements.iter().position(|s| s.contains("CREATE TYPE"));
    let table_pos = statements
        .iter()
        .position(|s| s.contains("CREATE TABLE") && s.contains("pg_complex"));

    if let (Some(enum_idx), Some(table_idx)) = (enum_pos, table_pos) {
        assert!(
            enum_idx < table_idx,
            "Enum should be created before table that uses it"
        );
    }
}

#[test]
fn test_schema_items_include_indexes() {
    let schema = PgSimpleWithIndexSchema::new();
    let statements = schema.create_statements();

    let has_index = statements.iter().any(|s| s.contains("INDEX"));
    assert!(has_index, "Schema items should include indexes");
}
