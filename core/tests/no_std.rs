//! Test that drizzle-core compiles and works in no_std environments
//!
//! Run with: cargo test -p drizzle-core --no-default-features --features "alloc,sqlite"

#![no_std]

extern crate alloc;

use drizzle_core::{SQL, SQLParam, Token};

// Define a mock value type for testing
#[derive(Clone, Debug)]
pub struct TestValue;

impl SQLParam for TestValue {
    const DIALECT: drizzle_core::dialect::Dialect = drizzle_core::dialect::Dialect::SQLite;
}

#[test]
fn test_sql_empty_no_std() {
    let sql: SQL<'_, TestValue> = SQL::empty();
    assert!(sql.chunks.is_empty());
}

#[test]
fn test_sql_raw_no_std() {
    let sql: SQL<'_, TestValue> = SQL::raw("SELECT * FROM users");
    let result = sql.sql();
    assert_eq!(result, "SELECT * FROM users");
}

#[test]
fn test_sql_raw_cow_no_std() {
    let sql: SQL<'_, TestValue> = SQL::raw("SELECT 1");
    let result = sql.sql();
    assert_eq!(result, "SELECT 1");
}

#[test]
fn test_sql_ident_no_std() {
    let sql: SQL<'_, TestValue> = SQL::ident("users");
    let result = sql.sql();
    assert_eq!(result, "\"users\"");
}

#[test]
fn test_sql_push_token_no_std() {
    let sql: SQL<'_, TestValue> = SQL::empty()
        .push(Token::SELECT)
        .push(Token::STAR)
        .push(Token::FROM)
        .append(SQL::ident("users"));
    let result = sql.sql();
    assert_eq!(result, "SELECT * FROM \"users\"");
}

#[test]
fn test_sql_append_no_std() {
    let sql1: SQL<'_, TestValue> = SQL::raw("SELECT");
    let sql2: SQL<'_, TestValue> = SQL::raw("FROM users");
    let combined = sql1.append(sql2);
    let result = combined.sql();
    assert_eq!(result, "SELECT FROM users");
}

#[test]
fn test_sql_clone_no_std() {
    let sql: SQL<'_, TestValue> = SQL::raw("test");
    let cloned = sql.clone();
    assert_eq!(sql.sql(), cloned.sql());
}

#[test]
fn test_tokens_no_std() {
    // Verify tokens work in no_std
    assert_eq!(Token::SELECT.as_str(), "SELECT");
    assert_eq!(Token::FROM.as_str(), "FROM");
    assert_eq!(Token::WHERE.as_str(), "WHERE");
    assert_eq!(Token::AND.as_str(), "AND");
    assert_eq!(Token::OR.as_str(), "OR");
}

#[test]
fn test_sql_builder_pattern_no_std() {
    let sql: SQL<'_, TestValue> = SQL::empty()
        .push(Token::SELECT)
        .append(SQL::ident("id"))
        .push(Token::COMMA)
        .append(SQL::ident("name"))
        .push(Token::FROM)
        .append(SQL::ident("users"))
        .push(Token::WHERE)
        .append(SQL::ident("active"))
        .push(Token::EQ)
        .append(SQL::raw("1"));

    let result = sql.sql();
    assert_eq!(
        result,
        "SELECT \"id\", \"name\" FROM \"users\" WHERE \"active\" = 1"
    );
}
