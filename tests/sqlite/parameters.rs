#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

//! SQLite placeholder rendering (`:name`). Binding and execution contracts live
//! in `crate::common::prepared`.

#[cfg(all(feature = "serde", feature = "uuid"))]
use crate::common::schema::sqlite::InsertSimple;
use crate::common::schema::sqlite::{SimpleSchema, UpdateSimple};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[cfg(all(feature = "serde", feature = "uuid"))]
#[drizzle::test]
fn test_insert_with_placeholders(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Create a typed placeholder from the column
    let user_name = simple.name.placeholder("user_name");

    // Create insert model with typed placeholder
    let insert_data = InsertSimple::new(user_name);

    // Insert the data (should preserve the placeholder in the SQL)
    let insert_result = db.insert(simple).values([insert_data]);

    // Check that the generated SQL contains the placeholder
    let sql_string = insert_result.to_sql().sql();

    // The SQL should contain the named placeholder
    assert!(
        sql_string.contains(":user_name"),
        "SQL should contain the :user_name placeholder"
    );

    // Test that parameters are correctly preserved
    let sql = insert_result.to_sql();
    let params: Vec<_> = sql.params().collect();
    assert!(
        params.is_empty(),
        "Should have no bound parameters since we used a placeholder"
    );
}

#[drizzle::test]
fn test_update_with_placeholders_sql(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Create typed placeholders from columns
    let new_name = simple.name.placeholder("new_name");
    let old_name = simple.name.placeholder("old_name");

    // Create update with placeholder in SET and WHERE
    let update = UpdateSimple::default().with_name(new_name);
    let stmt = db
        .update(simple)
        .set(update)
        .r#where(eq(simple.name, old_name));

    let sql = stmt.to_sql();
    let sql_string = sql.sql();

    // Verify SQL structure
    assert!(
        sql_string.starts_with("UPDATE"),
        "Should be an UPDATE statement, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("\"simple\""),
        "Should reference the simple table, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains(":new_name"),
        "SET clause should contain :new_name placeholder, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains(":old_name"),
        "WHERE clause should contain :old_name placeholder, got: {}",
        sql_string
    );

    // All values are placeholders, so there should be no bound parameters
    let params: Vec<_> = sql.params().collect();
    assert!(
        params.is_empty(),
        "Should have no bound parameters since all values are placeholders, got {} params",
        params.len()
    );
}
