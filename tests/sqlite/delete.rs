#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{InsertComplex, Role, SimpleComplexSchema};
use crate::common::schema::sqlite::{InsertSimple, Simple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[allow(dead_code)]
#[derive(SQLiteFromRow, Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ComplexResult {
    id: Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

sqlite_test!(simple_delete, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test records
    let test_data = vec![
        InsertSimple::new("delete_me"),
        InsertSimple::new("keep_me"),
        InsertSimple::new("delete_me"),
    ];

    let insert_result = drizzle_exec!(db.insert(simple).values(test_data) => execute);
    assert_eq!(insert_result, 3);

    // Verify initial state
    let initial_results: Vec<SimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple) => all_as);
    assert_eq!(initial_results.len(), 3);

    // Delete records with specific condition
    let delete_result = drizzle_exec!(
        db.delete(simple)
            .r#where(eq(simple.name, "delete_me"))
            => execute
    );

    assert_eq!(delete_result, 2); // Should delete 2 records

    // Verify deletion - should only have "keep_me" left
    let remaining_results: Vec<SimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple) => all_as);

    assert_eq!(remaining_results.len(), 1);
    assert_eq!(remaining_results[0].name, "keep_me");

    // Verify deleted records are gone
    let deleted_results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(Simple::name, "delete_me"))
            => all_as
    );

    assert_eq!(deleted_results.len(), 0);
});

#[cfg(feature = "uuid")]
sqlite_test!(feature_gated_delete, SimpleComplexSchema, {
    let SimpleComplexSchema { complex, .. } = schema;

    // Insert test records with UUIDs
    let test_id_1 = uuid::Uuid::new_v4();
    let test_id_2 = uuid::Uuid::new_v4();

    let test_data = vec![
        InsertComplex::new("delete_user", true, Role::User)
            .with_id(test_id_1)
            .with_email("delete@example.com".to_string())
            .with_age(25),
        InsertComplex::new("keep_user", true, Role::User)
            .with_id(test_id_2)
            .with_email("keep@example.com".to_string())
            .with_age(35),
    ];

    let insert_result = drizzle_exec!(db.insert(complex).values(test_data) => execute);
    assert_eq!(insert_result, 2);

    // Verify initial state
    let initial_results: Vec<ComplexResult> = drizzle_exec!(
        db.select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            => all_as
    );
    drizzle_assert_eq!(2, initial_results.len());

    // Delete specific record using UUID primary key
    let delete_result = drizzle_exec!(
        db.delete(complex)
            .r#where(eq(complex.id, test_id_1))
            => execute
    );
    drizzle_assert_eq!(1, delete_result);

    // Verify deletion - should only have keep_user left
    let remaining_results: Vec<ComplexResult> = drizzle_exec!(
        db.select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            => all_as
    );

    drizzle_assert_eq!(1, remaining_results.len());
    drizzle_assert_eq!("keep_user", remaining_results[0].name.as_str());
    drizzle_assert_eq!(test_id_2, remaining_results[0].id);

    // Verify specific UUID record is gone
    let deleted_results: Vec<ComplexResult> = drizzle_exec!(
        db.select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            .r#where(eq(complex.id, test_id_1.to_string()))
            => all_as
    );

    drizzle_assert_eq!(0, deleted_results.len());
});
