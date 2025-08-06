use common::{Complex, InsertComplex, InsertSimple, Simple, setup_db};
use drizzle_rs::prelude::*;
use rusqlite::Row;

mod common;

#[derive(Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

impl TryFrom<&Row<'_>> for SimpleResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }
}

#[cfg(not(feature = "uuid"))]
#[derive(Debug)]
struct ComplexResult {
    id: i32,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

#[cfg(feature = "uuid")]
#[derive(Debug)]
struct ComplexResult {
    id: Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

impl TryFrom<&Row<'_>> for ComplexResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
            age: row.get(3)?,
        })
    }
}

#[test]
fn simple_delete() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    // Insert test records
    let test_data = vec![
        InsertSimple::default().with_name("delete_me"),
        InsertSimple::default().with_name("keep_me"),
        InsertSimple::default().with_name("delete_me"),
    ];

    let insert_result = drizzle
        .insert::<Simple>()
        .values(test_data)
        .execute()
        .unwrap();
    assert_eq!(insert_result, 3);

    // Verify initial state
    let initial_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from::<Simple>()
        .all()
        .unwrap();
    assert_eq!(initial_results.len(), 3);

    // Delete records with specific condition
    let delete_result = drizzle
        .delete::<Simple>()
        .r#where(eq(Simple::name, "delete_me"))
        .execute()
        .unwrap();
    assert_eq!(delete_result, 2); // Should delete 2 records

    // Verify deletion - should only have "keep_me" left
    let remaining_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from::<Simple>()
        .all()
        .unwrap();

    assert_eq!(remaining_results.len(), 1);
    assert_eq!(remaining_results[0].name, "keep_me");

    // Verify deleted records are gone
    let deleted_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from::<Simple>()
        .r#where(eq(Simple::name, "delete_me"))
        .all()
        .unwrap();

    assert_eq!(deleted_results.len(), 0);
}

#[cfg(feature = "uuid")]
#[test]
fn feature_gated_delete() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    // Insert test records with UUIDs
    let test_id_1 = uuid::Uuid::new_v4();
    let test_id_2 = uuid::Uuid::new_v4();

    let test_data = vec![
        InsertComplex::default()
            .with_id(test_id_1)
            .with_name("delete_user")
            .with_email("delete@example.com".to_string())
            .with_age(25),
        InsertComplex::default()
            .with_id(test_id_2)
            .with_name("keep_user")
            .with_email("keep@example.com".to_string())
            .with_age(35),
    ];

    let insert_result = drizzle
        .insert::<Complex>()
        .values(test_data)
        .execute()
        .unwrap();
    assert_eq!(insert_result, 2);

    // Verify initial state
    let initial_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from::<Complex>()
        .all()
        .unwrap();
    assert_eq!(initial_results.len(), 2);

    // Delete specific record using UUID primary key
    let delete_result = drizzle
        .delete::<Complex>()
        .r#where(eq(Complex::id, test_id_1))
        .execute()
        .unwrap();
    assert_eq!(delete_result, 1);

    // Verify deletion - should only have keep_user left
    let remaining_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from::<Complex>()
        .all()
        .unwrap();

    assert_eq!(remaining_results.len(), 1);
    assert_eq!(remaining_results[0].name, "keep_user");
    assert_eq!(remaining_results[0].id, test_id_2);

    // Verify specific UUID record is gone
    let deleted_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from::<Complex>()
        .r#where(eq(Complex::id, test_id_1.to_string()))
        .all()
        .unwrap();

    assert_eq!(deleted_results.len(), 0);
}
