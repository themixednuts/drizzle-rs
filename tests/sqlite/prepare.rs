#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::sqlite::prelude::*;
use drizzle_core::{SQL, SQLChunk, ToSQL, prepared::prepare_render};
use drizzle_macros::sqlite_test;

use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};
use drizzle_sqlite::{SQLiteSQL, SQLiteValue};

sqlite_test!(test_prepare_with_placeholder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("Alice"), InsertSimple::new("")])
            .execute()
    );

    // Test prepare with simple raw SQL and placeholder
    let prepared_sql = db
        .select(simple.name)
        .from(simple)
        .r#where(and([eq(simple.name, SQL::placeholder("name"))]))
        .prepare();

    println!("{prepared_sql}");

    #[derive(SQLiteFromRow, Default)]
    struct PartialSimple {
        name: String,
    }

    let result: Vec<PartialSimple> =
        drizzle_exec!(prepared_sql.all(db.conn(), params![{name: "Alice"}]));

    // Verify we have the right parameter count and value
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Alice");
});

#[test]
fn test_prepare_render_basic() {
    // Test the basic prepare_render functionality
    let sql = SQLiteSQL::raw("SELECT * FROM users WHERE id = ")
        .append(SQL::placeholder("user_id"))
        .append(SQL::raw(" AND name = "))
        .append(SQL::placeholder("user_name"));

    let prepared = prepare_render(sql);

    // Should have 3 text segments: before first param, between params, after last param
    assert_eq!(prepared.text_segments.len(), 3);
    assert_eq!(prepared.params.len(), 2);

    // Verify text segments (note: raw text segments may have different spacing)
    assert!(prepared.text_segments[0].contains("SELECT * FROM users WHERE id"));
    assert!(prepared.text_segments[1].contains("AND name"));
    // Text segments vary based on SQL generation

    // Verify we have the expected parameters
    assert_eq!(prepared.params.len(), 2);
}

#[test]
fn test_prepare_with_multiple_parameters() {
    // Test preparing SQL with multiple parameters of different types
    let sql = SQLiteSQL::raw("INSERT INTO users (name, age, active) VALUES (")
        .append(SQL::placeholder("name"))
        .append(SQL::raw(", "))
        .append(SQL::placeholder("age"))
        .append(SQL::raw(", "))
        .append(SQL::placeholder("active"))
        .append(SQL::raw(")"));

    let prepared = prepare_render(sql);
    let (final_sql, bound_params_iter) = prepared.bind(params![
        {name: "alice"},
        {age: 25i32},
        {active: true}
    ]);
    let bound_params: Vec<_> = bound_params_iter.collect();

    // Verify SQL structure
    assert!(final_sql.contains("INSERT INTO users (name, age, active) VALUES ("));
    assert!(final_sql.contains(":name"));
    assert!(final_sql.contains(":age"));
    assert!(final_sql.contains(":active"));

    // Verify bound parameters
    assert_eq!(bound_params.len(), 3);
    assert_eq!(bound_params[0], SQLiteValue::from("alice"));
    assert_eq!(bound_params[1], SQLiteValue::from(25i32));
    assert_eq!(bound_params[2], SQLiteValue::from(true));
}

#[test]
fn test_prepare_sql_reconstruction() {
    // Test that we can reconstruct complete SQL from prepared statement
    let first = "SELECT * FROM posts WHERE author = ";
    let second = " AND published = ";
    let last = " ORDER BY created_at DESC";

    let sql = SQL::<SQLiteValue>::raw(first)
        .append(SQL::placeholder("author"))
        .append(SQL::raw(second))
        .append(SQL::placeholder("published"))
        .append(SQL::raw(last));

    let prepared = prepare_render(sql);
    let chunk_sql = prepared.to_sql();
    let chunks: Vec<_> = chunk_sql.into_iter().collect();

    // Should have exactly 5 chunks: text[0], param[0], text[1], param[1], text[2]
    assert_eq!(chunks.len(), 5);

    // Check interweaving pattern: text, param, text, param, text
    match (&chunks[0], &chunks[1], &chunks[2], &chunks[3], &chunks[4]) {
        (
            SQLChunk::Raw(text1),
            SQLChunk::Param(param1),
            SQLChunk::Raw(text2),
            SQLChunk::Param(param2),
            SQLChunk::Raw(text3),
        ) => {
            // Verify text content (trimmed to handle whitespace differences)
            assert!(
                text1
                    .to_string()
                    .contains("SELECT * FROM posts WHERE author")
            );
            assert!(text2.to_string().contains("AND published"));
            assert!(text3.to_string().contains("ORDER BY created_at"));

            // Verify param names
            assert_eq!(param1.placeholder.name, Some("author"));
            assert_eq!(param2.placeholder.name, Some("published"));
        }
        _ => panic!("Chunks are not in expected text-param-text-param-text pattern"),
    }

    let (final_sql, _) = prepared.bind(params![
        {author: "john_doe"},
        {published: true}
    ]);

    // The reconstructed SQL should have the same structure as the original
    assert!(final_sql.contains("SELECT * FROM posts WHERE author = :author"));
    assert!(final_sql.contains("AND published = :published"));
    assert!(final_sql.contains("ORDER BY created_at DESC"));
}

#[test]
fn test_prepare_with_no_parameters() {
    // Test preparing SQL with no parameters
    let sql = SQL::<SQLiteValue>::raw("SELECT COUNT(*) FROM users");
    let prepared = prepare_render(sql);

    assert_eq!(prepared.text_segments.len(), 1);
    assert_eq!(prepared.params.len(), 0);
    assert_eq!(prepared.text_segments[0], "SELECT COUNT(*) FROM users");

    // Binding no parameters should work
    let (final_sql, bound_params_iter) = prepared.bind::<SQLiteValue<'_>>([]);
    let bound_params: Vec<_> = bound_params_iter.collect();
    assert_eq!(final_sql, "SELECT COUNT(*) FROM users");
    assert_eq!(bound_params.len(), 0);
}

#[test]
fn test_prepare_complex_query() {
    // Test a more complex query with mixed SQL construction
    let sql = SQL::<SQLiteValue>::raw("WITH RECURSIVE category_tree AS (")
        .append(SQL::raw(
            "SELECT id, name, parent_id FROM categories WHERE id = ",
        ))
        .append(SQL::placeholder("root_id"))
        .append(SQL::raw(
            " UNION ALL SELECT c.id, c.name, c.parent_id FROM categories c ",
        ))
        .append(SQL::raw(
            "INNER JOIN category_tree ct ON c.parent_id = ct.id) ",
        ))
        .append(SQL::raw("SELECT * FROM category_tree WHERE name LIKE "))
        .append(SQL::placeholder("search_pattern"));

    let prepared = prepare_render(sql);
    let (final_sql, bound_params_iter) = prepared.bind(params![
        {root_id: 1i32},
        {search_pattern: "%electronics%"}
    ]);
    let bound_params: Vec<_> = bound_params_iter.collect();

    // Verify complex SQL is reconstructed correctly
    assert!(final_sql.contains("WITH RECURSIVE category_tree AS"));
    assert!(final_sql.contains(":root_id"));
    assert!(final_sql.contains(":search_pattern"));
    assert_eq!(bound_params.len(), 2);
    assert_eq!(bound_params[0], SQLiteValue::from(1i32));
    assert_eq!(bound_params[1], SQLiteValue::from("%electronics%"));
}

sqlite_test!(test_prepared_performance_comparison, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    // Insert test data
    let test_data: Vec<_> = (0..1000)
        .map(|i| InsertSimple::new(format!("User{}", i)))
        .collect();
    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test regular query performance
    let start = std::time::Instant::now();
    for i in 0..100 {
        let _results: Vec<SelectSimple> = drizzle_exec!(
            db.select(())
                .from(simple)
                .r#where(eq(simple.name, format!("User{}", i)))
                .all()
        );
    }
    let regular_duration = start.elapsed();

    // Test prepared statement performance
    let prepared = db
        .select(())
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare()
        .into_owned();

    let start = std::time::Instant::now();
    for i in 0..100 {
        let _results: Vec<SelectSimple> =
            drizzle_exec!(prepared.all(db.conn(), params![{name: format!("User{}", i)}]));
    }
    let prepared_duration = start.elapsed();

    println!("Regular queries: {:?}", regular_duration);
    println!("Prepared statements: {:?}", prepared_duration);

    // Prepared statements should generally be faster for repeated queries
    // This is more of a demonstration than a strict assertion since performance can vary
    assert!(
        prepared_duration <= regular_duration * 2,
        "Prepared statements shouldn't be significantly slower"
    );
});

sqlite_test!(test_prepared_insert_performance, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    // Insert test data

    // Test regular insert performance
    let start = std::time::Instant::now();
    for i in 0..100 {
        let data = InsertSimple::new(format!("RegularUser{}", i));
        drizzle_exec!(db.insert(simple).values([data]).execute());
    }
    let regular_duration = start.elapsed();

    // Clear table for prepared test
    let delete_result = drizzle_exec!(db.execute(SQL::raw("DELETE FROM \"simple\"")));
    println!("Delete result: {:?}", delete_result);

    // Test prepared insert performance - use the same data structure as regular inserts
    let start = std::time::Instant::now();
    for i in 0..100 {
        let data = InsertSimple::new(format!("PreparedUser{}", i));
        let prepared = db.insert(simple).values([data]).prepare();
        drizzle_exec!(prepared.execute(db.conn(), []));
    }
    let prepared_duration = start.elapsed();

    println!("Regular inserts: {:?}", regular_duration);
    println!("Prepared inserts: {:?}", prepared_duration);

    // Verify prepared statements work correctly
    let prepared_results: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(SQL::raw("\"simple\".\"name\" LIKE 'PreparedUser%'"))
            .all()
    );

    println!("Prepared results: {}", prepared_results.len());

    // Verify prepared statement execution worked
    assert_eq!(prepared_results.len(), 100);

    // Demonstrate that prepared statements generally provide performance benefits
    assert!(
        prepared_duration <= regular_duration * 2,
        "Prepared statements shouldn't be significantly slower"
    );
});
