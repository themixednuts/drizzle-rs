#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::core::expr::*;
use drizzle::{sql, sqlite::prelude::*};
use drizzle_macros::sqlite_test;
use drizzle_sqlite::values::SQLiteValue;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, Simple, SimpleSchema};

sqlite_test!(test_simple_select_all_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let query = db.select(()).from(simple);
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
});

#[cfg(feature = "uuid")]
sqlite_test!(test_complex_select_all_sql_generation, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Test select(()).from(complex) - should generate all complex table columns
    let query = db.select(()).from(complex);
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
});

sqlite_test!(test_select_all_with_where_clause, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Test select(()).from(table).where(...) - should still work with qualified columns
    let query = db.select(()).from(simple).r#where(eq(Simple::name, "test"));

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
    let params: Vec<_> = sql.params().collect();
    assert!(
        !params.is_empty(),
        "Should have parameters for WHERE clause"
    );
});

sqlite_test!(test_select_specific_columns_vs_select_all, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Compare select(()) vs select(columns![...])
    let select_all_query = db.select(()).from(simple);
    let select_specific_query = db.select((simple.id, simple.name)).from(simple);

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
});

sqlite_test!(test_sql_macro, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let id = 4;
    drizzle_try!(
        db.insert(simple)
            .values([InsertSimple::new("test").with_id(id)])
            .execute()
    )?;

    let query = sql!("SELECT * FROM {simple} where {simple.id} = {id}");
    let sql = query.sql();
    let params: Vec<_> = query.params().collect();

    assert_eq!(sql, r#"SELECT * FROM "simple" where "simple"."id" = ?"#);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Integer(id as i64));

    let results: Vec<SelectSimple> = drizzle_try!(db.all(query))?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
    assert_eq!(results[0].name, "test");
});

sqlite_test!(test_sql_printf_style, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let id = 5;
    let name = "printf_test";

    drizzle_try!(
        db.insert(simple)
            .values([InsertSimple::new(name).with_id(id)])
            .execute()
    )?;

    // Test printf-style syntax: sql!("template", arg1, arg2, ...)
    let query = sql!("SELECT * FROM {} WHERE {} = {}", simple, simple.id, id);
    let sql = query.sql();
    let params: Vec<_> = query.params().collect();

    assert_eq!(sql, r#"SELECT * FROM "simple" WHERE "simple"."id" = ?"#);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Integer(id as i64));
});

sqlite_test!(test_sql_mixed_named_positional, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let id = 6;
    let name = "mixed_test";

    drizzle_try!(
        db.insert(simple)
            .values([InsertSimple::new(name).with_id(id)])
            .execute()
    )?;

    // Test mixing positional {} and named {simple.id} expressions
    let query = sql!("SELECT * FROM {} WHERE {simple.id} = {}", simple, id);
    let sql = query.sql();
    let params: Vec<_> = query.params().collect();

    assert_eq!(sql, r#"SELECT * FROM "simple" WHERE "simple"."id" = ?"#);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Integer(id as i64));
});
