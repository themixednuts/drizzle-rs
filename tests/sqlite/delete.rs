#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{InsertComplex, Role, SimpleComplexSchema};
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, Simple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ComplexResult {
    id: Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

#[drizzle::test]
fn simple_delete(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert test records
    let test_data = vec![
        InsertSimple::new("delete_me"),
        InsertSimple::new("keep_me"),
        InsertSimple::new("delete_me"),
    ];

    let insert_result = db.insert(simple).values(test_data).execute();
    assert_eq!(insert_result, 3);

    // Verify initial state
    let initial_results: Vec<SelectSimple> = db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(initial_results.len(), 3);

    // Delete records with specific condition
    let delete_result = db
        .delete(simple)
        .r#where(eq(simple.name, "delete_me"))
        .execute();

    assert_eq!(delete_result, 2); // Should delete 2 records

    // Verify deletion - should only have "keep_me" left
    let remaining_results: Vec<SelectSimple> =
        db.select((simple.id, simple.name)).from(simple).all();

    assert_eq!(remaining_results.len(), 1);
    assert_eq!(remaining_results[0].name, "keep_me");

    // Verify deleted records are gone
    let deleted_results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(Simple::name, "delete_me"))
        .all();

    assert_eq!(deleted_results.len(), 0);
}

#[drizzle::test]
fn delete_returning_star(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("delete_returning").with_id(103)])
        .execute();

    let stmt = db.delete(simple).r#where(eq(simple.id, 103)).returning(());

    assert_eq!(
        stmt.to_sql().sql(),
        r#"DELETE FROM "simple" WHERE "simple"."id" = ? RETURNING *"#
    );

    let rows: Vec<SelectSimple> = stmt.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 103);
    assert_eq!(rows[0].name, "delete_returning");
}

#[cfg(feature = "uuid")]
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
