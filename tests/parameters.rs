#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use common::{InsertSimple, Simple};
use drizzle_core::{SQL, prepared::prepare_render};
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

#[cfg(all(feature = "rusqlite", feature = "serde", feature = "uuid"))]
#[tokio::test]
async fn test_insert_with_placeholders() {
    let db = setup_test_db!();
    let (drizzle, simple) = drizzle!(db, [Simple]);

    // Create insert model with explicit placeholders
    let insert_data = InsertSimple::new(Placeholder::colon("user_name"));

    // Insert the data (should preserve the placeholder in the SQL)
    let insert_result = drizzle.insert(simple).values([insert_data]);

    // Check that the generated SQL contains the placeholder
    let sql_string = insert_result.to_sql().sql();
    println!("Generated SQL: {}", sql_string);

    // The SQL should contain the named placeholder
    assert!(
        sql_string.contains(":user_name"),
        "SQL should contain the :user_name placeholder"
    );

    // Test that parameters are correctly preserved
    let sql = insert_result.to_sql();
    let params = sql.params();
    assert!(
        params.is_empty(),
        "Should have no bound parameters since we used a placeholder"
    );
}

#[cfg(all(feature = "rusqlite", feature = "serde", feature = "uuid"))]
#[tokio::test]
async fn test_insert_with_placeholders_execute_and_retrieve() {
    use drizzle_core::{SQL, prepared::prepare_render};
    use sqlite::{InsertValue, SQLiteValue, params, values::ValueWrapper};

    #[derive(FromRow, Debug)]
    struct SimpleResult {
        id: i32,
        name: String,
    }

    let db = setup_test_db!();
    let (drizzle, simple) = drizzle!(db, [Simple]);

    // Create insert model with explicit placeholders
    let insert_data = InsertSimple::new(Placeholder::colon("user_name"));

    // Prepare the insert statement and execute it with bound parameters
    let prepared_insert = drizzle.insert(simple).values([insert_data]).prepare();

    // Execute the prepared insert with bound parameters
    let row_count = drizzle_exec!(prepared_insert.execute(
        drizzle.conn(),
        params![
            {user_name: "Alice"}
        ]
    ));
    assert_eq!(row_count, 1, "Should have inserted one row");

    // Retrieve the data to verify it was inserted correctly
    let results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "Alice"))
            .all()
    );

    assert_eq!(results.len(), 1, "Should have found one result");
    assert_eq!(
        results[0].name, "Alice",
        "Name should match the bound placeholder value"
    );

    println!("Successfully inserted and retrieved: {:?}", results[0]);
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
