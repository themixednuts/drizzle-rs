#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use common::{InsertSimple, Simple, setup_db};
use drizzle_core::{SQL, prepare_render};
use drizzle_rs::prelude::*;
use sqlite::{SQLiteValue, params};

mod common;

#[test]
fn test_params_macro_named() {
    // Test the params! macro with named parameters
    let params_array = params![{name: "alice"}, {active: true}];

    assert_eq!(params_array.len(), 2);
    assert_eq!(params_array[0].value, SQLiteValue::from("alice"));
    assert_eq!(params_array[1].value, SQLiteValue::from(true));
}

#[test]
fn test_params_macro_different_types() {
    // Test that params! macro handles different data types correctly
    let string_val = "test";
    let int_val = 42i32;
    let bool_val = true;
    let optional_val: Option<String> = Some("optional".to_string());

    let params_array = params![
        {name: string_val},
        {id: int_val},
        {active: bool_val},
        {description: optional_val}
    ];

    assert_eq!(params_array.len(), 4);
    assert_eq!(params_array[0].value, SQLiteValue::from(string_val));
    assert_eq!(params_array[1].value, SQLiteValue::from(int_val));
    assert_eq!(params_array[2].value, SQLiteValue::from(bool_val));
    assert_eq!(
        params_array[3].value,
        SQLiteValue::from(Some("optional".to_string()))
    );
}

#[test]
fn test_sql_parameter_binding() {
    // Test SQL parameter binding functionality with real SQL construction
    let sql = SQL::<SQLiteValue>::raw("SELECT * FROM users WHERE name = ")
        .append(SQL::placeholder("user_name"))
        .append_raw(" AND active = ")
        .append(SQL::placeholder("active"));

    let prepared = prepare_render(sql);
    let (final_sql, bound_params) = prepared.bind(params![
        {user_name: "alice"},
        {active: true}
    ]);

    // Verify the SQL contains proper placeholders
    assert!(final_sql.contains(":user_name"));
    assert!(final_sql.contains(":active"));

    // Verify we have the correct bound parameters
    assert_eq!(bound_params.len(), 2);
}

#[test]
fn test_placeholder_styles() {
    // Test that prepare_render preserves placeholder names
    let sql = SQL::<SQLiteValue>::raw("SELECT * WHERE id = ").append(SQL::placeholder("user_id"));
    let prepared = prepare_render(sql);

    // Should have one parameter
    assert_eq!(prepared.params.len(), 1);
    // Parameters contain placeholder name internally
}

#[tokio::test]
async fn test_parameter_integration_with_query_builder() {
    #[derive(FromRow, Default)]
    struct SimpleResult(String);
    let db = setup_test_db!();
    let (drizzle, simple) = drizzle!(db, [Simple]);

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];
    drizzle_exec!(drizzle.insert(simple).values(test_data).execute());

    // Test that normal query builder still works (this uses internal parameter binding)
    let results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select(simple.name)
            .from(simple)
            .r#where(eq(simple.name, "alice"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "alice");

    // Test multiple parameter conditions using multiple queries
    let alice_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select(simple.name)
            .from(simple)
            .r#where(eq(simple.name, "alice"))
            .all()
    );

    let bob_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select(simple.name)
            .from(simple)
            .r#where(eq(simple.name, "bob"))
            .all()
    );

    assert_eq!(alice_results.len(), 1);
    assert_eq!(bob_results.len(), 1);
    assert_eq!(alice_results[0].0, "alice");
    assert_eq!(bob_results[0].0, "bob");
}
