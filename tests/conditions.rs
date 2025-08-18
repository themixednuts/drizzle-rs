#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use common::{Complex, InsertComplex, InsertSimple, Role, SelectComplex, SelectSimple, Simple};
use drizzle_core::expressions::conditions::*;
use drizzle_rs::prelude::*;

mod common;

#[derive(Debug, FromRow)]
struct JsonExtractResult {
    extract: String,
}

#[derive(Debug, FromRow)]
struct ConcatResult {
    concat: String,
}

#[tokio::test]
async fn test_basic_comparison_conditions() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Item A").with_id(1),
        InsertSimple::new("Item B").with_id(2),
        InsertSimple::new("Item C").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test eq condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(eq(simple.name, "Item A"))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Item A");

    // Test neq condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(neq(simple.name, "Item A"))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test gt condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(gt(simple.id, 1)).all());
    assert_eq!(result.len(), 2);

    // Test gte condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(gte(simple.id, 2)).all());
    assert_eq!(result.len(), 2);

    // Test lt condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(lt(simple.id, 3)).all());
    assert_eq!(result.len(), 2);

    // Test lte condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(lte(simple.id, 2)).all());
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_in_array_conditions() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Apple").with_id(1),
        InsertSimple::new("Banana").with_id(2),
        InsertSimple::new("Cherry").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test in_array condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(in_array(simple.name, ["Apple", "Cherry"]))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test not_in_array condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(not_in_array(simple.name, ["Apple"]))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test is_in condition (alias for in_array)
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(is_in(simple.id, [1, 3]))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test not_in condition (alias for not_in_array)
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(not_in(simple.id, [1]))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test empty array
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(in_array(simple.name, Vec::<String>::new()))
            .all()
    );
    assert_eq!(result.len(), 0);
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_null_conditions() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    let test_data = vec![
        InsertComplex::new("User A", true, Role::User).with_email("user@example.com"),
        InsertComplex::new("User B", false, Role::Admin),
        InsertComplex::new("User C", true, Role::User).with_age(25),
    ];

    let stmt = db.insert(complex).values(test_data);
    let sql = stmt.to_sql();

    println!("{sql}");

    drizzle_exec!(stmt.execute());

    // Test is_null condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_null(complex.email))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test is_not_null condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_not_null(complex.email))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "User A");

    // Test is_null with integer field
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_null(complex.age))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test is_not_null with integer field
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_not_null(complex.age))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "User C");
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_between_conditions() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    let test_data = vec![
        InsertComplex::new("User A", true, Role::User)
            .with_age(20)
            .with_score(85.5),
        InsertComplex::new("User B", false, Role::Admin)
            .with_age(30)
            .with_score(92.0),
        InsertComplex::new("User C", true, Role::User)
            .with_age(25)
            .with_score(78.3),
    ];

    drizzle_exec!(db.insert(complex).values(test_data).execute());

    // Test between condition with integers
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(between(complex.age, 22, 28))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "User C");

    // Test between condition with floats
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(between(complex.score, 80.0, 95.0))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test not_between condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(not_between(complex.age, 22, 28))
            .all()
    );
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_like_conditions() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Apple Pie").with_id(1),
        InsertSimple::new("Apple Juice").with_id(2),
        InsertSimple::new("Orange Juice").with_id(3),
        InsertSimple::new("Berry Split").with_id(4),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test like condition with prefix
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "Apple%"))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test like condition with suffix
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "%Juice"))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test like condition with wildcard in middle
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "%a%"))
            .all()
    );
    assert_eq!(result.len(), 3);

    // Test not_like condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(not_like(simple.name, "Apple%"))
            .all()
    );
    assert_eq!(result.len(), 2);
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_logical_conditions() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    let test_data = vec![
        InsertComplex::new("Active Admin", true, Role::Admin).with_age(30),
        InsertComplex::new("Inactive Admin", false, Role::Admin).with_age(35),
        InsertComplex::new("Active User", true, Role::User).with_age(25),
        InsertComplex::new("Inactive User", false, Role::User).with_age(20),
    ];

    drizzle_exec!(db.insert(complex).values(test_data).execute());

    // Test and condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(and([
                eq(complex.active, true),
                eq(complex.role, Role::Admin)
            ]))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Active Admin");

    // Test or condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(or([eq(complex.role, Role::Admin), gt(complex.age, 30)]))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test not condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(not(eq(complex.active, true)))
            .all()
    );
    assert_eq!(result.len(), 2);

    // Test complex nested conditions
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(and([
                or([eq(complex.role, Role::Admin), gt(complex.age, 23)]),
                eq(complex.active, true)
            ]))
            .all()
    );
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_single_condition_logical_operations() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Test").with_id(1),
        InsertSimple::new("Other").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test single condition in and() - should not add extra parentheses
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(and([eq(simple.name, "Test")]))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Test");

    // Test single condition in or() - should not add extra parentheses
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(or([eq(simple.name, "Other")]))
            .all()
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Other");

    // Test empty conditions
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(and(Vec::<SQL<_>>::new()))
            .all()
    );
    assert_eq!(result.len(), 2); // Empty condition should return all
}

#[tokio::test]
async fn test_string_operations() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Hello").with_id(1),
        InsertSimple::new("World").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test string concatenation
    let result: Vec<ConcatResult> = drizzle_exec!(
        db.select(alias(string_concat(simple.name, " - Suffix"), "concat"))
            .from(simple)
            .all()
    );
    assert_eq!(result.len(), 2);
    let concats: Vec<String> = result.iter().map(|r| r.concat.clone()).collect();
    assert!(concats.contains(&"Hello - Suffix".to_string()));
    assert!(concats.contains(&"World - Suffix".to_string()));
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn test_sqlite_json_conditions() {
    use drizzle_rs::sqlite::conditions::*;

    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    // Create a simple table with a JSON column for testing
    exec_sql!(
        db.conn(),
        "ALTER TABLE simple ADD COLUMN json_data TEXT",
        db_params!()
    );

    let test_data = vec![
        InsertSimple::new("Item A").with_id(1),
        InsertSimple::new("Item B").with_id(2),
        InsertSimple::new("Item C").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Update with JSON data
    exec_sql!(
        db.conn(),
        "UPDATE simple SET json_data = '{\"name\": \"test\", \"value\": 42}' WHERE id = 1",
        db_params!()
    );
    exec_sql!(
        db.conn(),
        "UPDATE simple SET json_data = '{\"name\": \"other\", \"value\": 100}' WHERE id = 2",
        db_params!()
    );
    exec_sql!(
        db.conn(),
        "UPDATE simple SET json_data = '{\"items\": [1, 2, 3], \"status\": \"active\"}' WHERE id = 3",
        db_params!()
    );

    // Test json_eq condition - just testing function compilation
    // Note: This wouldn't actually work with the simple table structure but tests the function exists
    // let result = db.select(simple.id)
    //     .from(simple)
    //     .r#where(json_eq(simple.name, "$.name", "test"));
    // We're just testing that the json functions compile and generate SQL

    // Test json_extract helper
    let result: Vec<JsonExtractResult> = drizzle_exec!(
        db.select(alias(json_extract(simple.name, "name"), "extract"))
            .from(simple)
            .all()
    );
    assert_eq!(result.len(), 3);
}

#[tokio::test]
async fn test_condition_edge_cases() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![InsertSimple::new("Test").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test condition with empty string
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(neq(simple.name, ""))
            .all()
    );
    assert_eq!(result.len(), 1);

    // Test condition with special characters
    let special_data = InsertSimple::new("Test's \"quoted\" string").with_id(2);
    drizzle_exec!(db.insert(simple).values([special_data]).execute());

    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "%quoted%"))
            .all()
    );
    assert_eq!(result.len(), 1);
}
