#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use common::{Complex, Simple};
use drizzle_rs::prelude::*;

mod common;

#[tokio::test]
async fn test_simple_select_all_sql_generation() {
    let db = setup_test_db!();
    let (drizzle, simple) = drizzle!(db, [Simple]);

    let query = drizzle.select(()).from(simple);
    let sql = query.to_sql();

    println!("Simple select all SQL: {}", sql.sql());

    // Should generate: SELECT "simple"."id", "simple"."name" FROM "simple"
    let expected_columns = vec![r#""simple"."id""#, r#""simple"."name""#];
    let sql_string = sql.sql();

    assert!(sql_string.starts_with("SELECT "));
    assert!(sql_string.contains(r#"FROM "simple""#));

    // Check that both columns are included
    for col in expected_columns {
        assert!(
            sql_string.contains(col),
            "SQL should contain {}: {}",
            col,
            sql_string
        );
    }
}

#[tokio::test]
async fn test_complex_select_all_sql_generation() {
    let db = setup_test_db!();
    let (drizzle, complex) = drizzle!(db, [Complex]);

    // Test select(()).from(complex) - should generate all complex table columns
    let query = drizzle.select(()).from(complex);
    let sql = query.to_sql();

    println!("Complex select all SQL: {}", sql.sql());

    let sql_string = sql.sql();
    assert!(sql_string.starts_with("SELECT "));
    assert!(sql_string.contains(r#"FROM "complex""#));

    // Check that key columns are included (complex has many columns)
    let key_columns = vec![
        r#""complex"."id""#,
        r#""complex"."name""#,
        r#""complex"."email""#,
    ];
    for col in key_columns {
        assert!(
            sql_string.contains(col),
            "SQL should contain {}: {}",
            col,
            sql_string
        );
    }
}

#[tokio::test]
async fn test_select_all_with_where_clause() {
    let db = setup_test_db!();
    let (drizzle, (simple, _complex)) = drizzle!(db, [Simple, Complex]);

    // Test select(()).from(table).where(...) - should still work with qualified columns
    let query = drizzle
        .select(())
        .from(simple)
        .r#where(eq(Simple::name, "test"));

    let sql = query.to_sql();
    println!("Select all with WHERE SQL: {}", sql.sql());

    let sql_string = sql.sql();

    // Should contain qualified columns in SELECT
    assert!(sql_string.contains(r#""simple"."id""#));
    assert!(sql_string.contains(r#""simple"."name""#));

    // Should contain FROM and WHERE clauses
    assert!(sql_string.contains(r#"FROM "simple""#));
    assert!(sql_string.contains("WHERE"));

    // Parameters should include the where condition value
    let params = sql.params();
    assert!(
        !params.is_empty(),
        "Should have parameters for WHERE clause"
    );
}

#[tokio::test]
async fn test_select_specific_columns_vs_select_all() {
    let db = setup_test_db!();
    let (drizzle, (simple, _complex)) = drizzle!(db, [Simple, Complex]);

    // Compare select(()) vs select(columns![...])
    let select_all_query = drizzle.select(()).from(simple);
    let select_specific_query = drizzle.select((simple.id, simple.name)).from(simple);

    let select_all_sql = select_all_query.to_sql().sql();
    let select_specific_sql = select_specific_query.to_sql().sql();

    println!("Select all SQL: {}", select_all_sql);
    println!("Select specific SQL: {}", select_specific_sql);

    // Both should contain the same columns (since Simple only has id and name)
    assert!(select_all_sql.contains(r#""simple"."id""#));
    assert!(select_all_sql.contains(r#""simple"."name""#));
    assert!(select_specific_sql.contains(r#""simple"."id""#));
    assert!(select_specific_sql.contains(r#""simple"."name""#));

    // Both should have FROM clause
    assert!(select_all_sql.contains(r#"FROM "simple""#));
    assert!(select_specific_sql.contains(r#"FROM "simple""#));
}
