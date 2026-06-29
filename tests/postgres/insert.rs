//! PostgreSQL INSERT statement tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    active: bool,
}

#[drizzle::test]
fn insert_single_row(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    stmt.execute();

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
    assert!(results[0].id > 0, "ID should be auto-generated");
}

#[drizzle::test]
fn insert_with_table_and_column_refs(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let simple_ref = &simple;
    let name_ref = &simple.name;

    let stmt = db
        .insert(simple_ref)
        .values([InsertSimple::new("RefAlice")]);
    stmt.execute();

    let stmt = db
        .select((simple_ref.id, simple_ref.name))
        .from(simple_ref)
        .r#where(eq(name_ref, "RefAlice"));
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "RefAlice");
}

#[drizzle::test]
fn insert_from_select_with_returning(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .select(SQL::raw("SELECT 9001, 'pg_from_select'"))
        .returning((simple.id, simple.name));

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" SELECT 9001, 'pg_from_select' RETURNING "simple"."id", "simple"."name""#
    );

    let results: Vec<SelectSimple> = stmt.all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "pg_from_select");
    assert_eq!(results[0].id, 9001);
}

#[drizzle::test]
fn insert_multiple_rows(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
    ]);
    stmt.execute();

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 3);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Alice"));
    assert!(names.contains(&"Bob"));
    assert!(names.contains(&"Charlie"));
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn insert_with_optional_fields(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("Alice", true, Role::Admin)
            .with_email("alice@example.com")
            .with_age(30)]);
    stmt.execute();

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
    assert_eq!(results[0].email, Some("alice@example.com".to_string()));
    assert_eq!(results[0].age, Some(30));
    assert!(results[0].active);
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn insert_with_null_fields(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("Bob", false, Role::User)]);
    stmt.execute();

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
    assert_eq!(results[0].email, None);
    assert_eq!(results[0].age, None);
    assert!(!results[0].active);
}

#[drizzle::test]
fn insert_special_characters(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("O'Brien"),
        InsertSimple::new("Hello \"World\""),
        InsertSimple::new("Line1\nLine2"),
        InsertSimple::new("Tab\there"),
        InsertSimple::new("Emoji 🎉"),
    ]);
    stmt.execute();

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 5);
    assert!(results.iter().any(|r| r.name == "O'Brien"));
    assert!(results.iter().any(|r| r.name == "Hello \"World\""));
    assert!(results.iter().any(|r| r.name == "Line1\nLine2"));
    assert!(results.iter().any(|r| r.name == "Tab\there"));
    assert!(results.iter().any(|r| r.name == "Emoji 🎉"));
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn insert_with_custom_uuid(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    let custom_id = uuid::Uuid::new_v4();
    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("CustomID", true, Role::User).with_id(custom_id)]);
    stmt.execute();

    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.id, custom_id));
    let results: Vec<PgComplexResult> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, custom_id);
    assert_eq!(results[0].name, "CustomID");
}

#[drizzle::test]
fn insert_large_batch(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Create a batch of 100 rows
    let names: Vec<String> = (0..100).map(|i| format!("User_{}", i)).collect();
    let rows: Vec<_> = names
        .iter()
        .map(|n| InsertSimple::new(n.as_str()))
        .collect();

    let stmt = db.insert(simple).values(rows);
    stmt.execute();

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 100);
}
