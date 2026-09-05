#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    feature = "uuid"
))]
//! SQLite-specific DELETE coverage (UUID primary keys); the portable cases
//! live in `crate::common::delete`.

use crate::common::schema::sqlite::{InsertComplex, Role, SimpleComplexSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ComplexResult {
    id: Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

#[drizzle::test]
fn feature_gated_delete(db: &mut TestDb<SimpleComplexSchema>) {
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

    let insert_result = db.insert(complex).values(test_data).execute();
    assert_eq!(insert_result, 2);

    // Verify initial state
    let initial_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .all();
    assert_eq!(2, initial_results.len());

    // Delete specific record using UUID primary key
    let delete_result = db
        .delete(complex)
        .r#where(eq(complex.id, test_id_1))
        .execute();
    assert_eq!(1, delete_result);

    // Verify deletion - should only have keep_user left
    let remaining_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .all();

    assert_eq!(1, remaining_results.len());
    assert_eq!("keep_user", remaining_results[0].name.as_str());
    assert_eq!(test_id_2, remaining_results[0].id);

    // Verify specific UUID record is gone
    let deleted_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(eq(complex.id, test_id_1.to_string()))
        .all();

    assert_eq!(0, deleted_results.len());
}
