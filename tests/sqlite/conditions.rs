#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{ComplexSchema, InsertComplex, Role, SelectComplex};
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};
use drizzle::core::expr::*;
#[cfg(feature = "serde")]
use drizzle::sqlite::expressions::json_extract;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[cfg(feature = "serde")]
#[derive(Debug, SQLiteFromRow)]
struct JsonExtractResult {
    extract: String,
}

#[derive(Debug, SQLiteFromRow)]
struct ConcatResult {
    concat: String,
}

sqlite_test!(test_basic_comparison_conditions, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let test_data = vec![
        InsertSimple::new("Item A").with_id(1),
        InsertSimple::new("Item B").with_id(2),
        InsertSimple::new("Item C").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test eq condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(eq(simple.name, "Item A"))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Item A");

    // Test neq condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(neq(simple.name, "Item A"))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test gt condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(gt(simple.id, 1)) => all);
    assert_eq!(result.len(), 2);

    // Test gte condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(gte(simple.id, 2)) => all);
    assert_eq!(result.len(), 2);

    // Test lt condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(lt(simple.id, 3)) => all);
    assert_eq!(result.len(), 2);

    // Test lte condition
    let result: Vec<SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).r#where(lte(simple.id, 2)) => all);
    assert_eq!(result.len(), 2);
});

sqlite_test!(test_in_array_conditions, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Apple").with_id(1),
        InsertSimple::new("Banana").with_id(2),
        InsertSimple::new("Cherry").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test in_array condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(in_array(simple.name, ["Apple", "Cherry"]))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test not_in_array condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(not_in_array(simple.name, ["Apple"]))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test empty array
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(in_array(simple.name, Vec::<String>::new()))
            => all
    );
    assert_eq!(result.len(), 0);
});

#[cfg(feature = "uuid")]
sqlite_test!(test_null_conditions, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Insert data with separate operations since each has different column patterns
    // User A: has email set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User A", true, Role::User).with_email("user@example.com")])
            => execute
    );

    // User B: has no optional fields set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User B", false, Role::Admin)])
            => execute
    );

    // User C: has age set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User C", true, Role::User).with_age(25)])
            => execute
    );

    // Test is_null condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_null(complex.email))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test is_not_null condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_not_null(complex.email))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "User A");

    // Test is_null with integer field
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_null(complex.age))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test is_not_null with integer field
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(is_not_null(complex.age))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "User C");
});

#[cfg(feature = "uuid")]
sqlite_test!(test_between_conditions, ComplexSchema, {
    let ComplexSchema { complex } = schema;

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

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // Test between condition with integers
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(between(complex.age, 22, 28))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "User C");

    // Test between condition with floats
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(between(complex.score, 80.0, 95.0))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test not_between condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(not_between(complex.age, 22, 28))
            => all
    );
    assert_eq!(result.len(), 2);
});

sqlite_test!(test_like_conditions, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Apple Pie").with_id(1),
        InsertSimple::new("Apple Juice").with_id(2),
        InsertSimple::new("Orange Juice").with_id(3),
        InsertSimple::new("Berry Split").with_id(4),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test like condition with prefix
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "Apple%"))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test like condition with suffix
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "%Juice"))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test like condition with wildcard in middle
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "%a%"))
            => all
    );
    assert_eq!(result.len(), 3);

    // Test not_like condition
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(not_like(simple.name, "Apple%"))
            => all
    );
    assert_eq!(result.len(), 2);
});

#[cfg(feature = "uuid")]
sqlite_test!(test_logical_conditions, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let test_data = vec![
        InsertComplex::new("Active Admin", true, Role::Admin).with_age(30),
        InsertComplex::new("Inactive Admin", false, Role::Admin).with_age(35),
        InsertComplex::new("Active User", true, Role::User).with_age(25),
        InsertComplex::new("Inactive User", false, Role::User).with_age(20),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // Test and condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(and([
                eq(complex.active, true),
                eq(complex.role, Role::Admin)
            ]))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Active Admin");

    // Test or condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(or([eq(complex.role, Role::Admin), gt(complex.age, 30)]))
            => all
    );
    assert_eq!(result.len(), 2);

    // Test not condition
    let result: Vec<SelectComplex> = drizzle_exec!(
        db.select(())
            .from(complex)
            .r#where(not(eq(complex.active, true)))
            => all
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
            => all
    );
    assert_eq!(result.len(), 2);
});

sqlite_test!(test_single_condition_logical_operations, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data - both have same pattern (both set id)
    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("Test").with_id(1),
                InsertSimple::new("Other").with_id(2),
            ])
            => execute
    );

    // Test single condition in and() - should not add extra parentheses
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(and([eq(simple.name, "Test")]))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Test");

    // Test single condition in or() - should not add extra parentheses
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(or([eq(simple.name, "Other")]))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Other");

    // Test no condition (get all records)
    let result: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(result.len(), 2); // No condition should return all
});

sqlite_test!(test_string_operations, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Hello").with_id(1),
        InsertSimple::new("World").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test string concatenation
    let result: Vec<ConcatResult> = drizzle_exec!(
        db.select(alias(string_concat(simple.name, " - Suffix"), "concat"))
            .from(simple)
            => all
    );
    assert_eq!(result.len(), 2);
    let concats: Vec<String> = result.iter().map(|r| r.concat.clone()).collect();
    assert!(concats.contains(&"Hello - Suffix".to_string()));
    assert!(concats.contains(&"World - Suffix".to_string()));
});

#[cfg(all(feature = "sqlite", feature = "serde"))]
sqlite_test!(test_sqlite_json_conditions, ComplexSchema, {
    use crate::common::schema::sqlite::{ComplexSchema, UserMetadata};

    let ComplexSchema { complex } = schema;

    // Insert test data with JSON metadata
    let metadata = UserMetadata {
        preferences: vec!["dark_theme".to_string(), "compact_view".to_string()],
        last_login: Some("2024-01-01T10:00:00Z".to_string()),
        theme: "dark".to_string(),
    };

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("User A", true, Role::User).with_metadata(metadata.clone()),
            ])
            => execute
    );

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User B", false, Role::Admin),])
            => execute
    );

    // Test json_extract helper on the metadata field
    let result: Vec<JsonExtractResult> = drizzle_exec!(
        db.select(alias(json_extract(complex.metadata, "theme"), "extract"))
            .from(complex)
            .r#where(is_not_null(complex.metadata))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].extract, "dark");
});

sqlite_test!(test_condition_edge_cases, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Test").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test condition with empty string
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(neq(simple.name, ""))
            => all
    );
    assert_eq!(result.len(), 1);

    // Test condition with special characters
    let special_data = InsertSimple::new("Test's \"quoted\" string").with_id(2);
    drizzle_exec!(db.insert(simple).values([special_data]) => execute);

    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select(())
            .from(simple)
            .r#where(like(simple.name, "%quoted%"))
            => all
    );
    assert_eq!(result.len(), 1);
});
