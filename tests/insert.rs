#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use common::{Complex, InsertComplex, InsertSimple, Simple, setup_db};
use drizzle_rs::prelude::*;
use drizzle_rs::sqlite::builder::Conflict;
#[cfg(feature = "uuid")]
use uuid::Uuid;

mod common;

#[derive(FromRow, Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(not(feature = "uuid"))]
#[derive(Debug)]
struct ComplexResult {
    id: i32,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

#[cfg(feature = "uuid")]
#[derive(FromRow, Debug)]
struct ComplexResult {
    id: Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

#[tokio::test]
async fn simple_insert() {
    let db = setup_test_db!();
    let (drizzle, (simple, ..)) = drizzle!(db, [Simple, Complex]);

    // Insert Simple record
    let data = InsertSimple::new("test");
    let result = drizzle_exec!(drizzle.insert(simple).values([data]).execute());

    assert_eq!(result, 1);

    // Verify insertion by selecting the record
    let results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "test"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
}

#[tokio::test]
async fn complex_insert() {
    let db = setup_test_db!();
    let (drizzle, (.., complex)) = drizzle!(db, [Simple, Complex]);

    // Insert Complex record with various field types
    #[cfg(not(feature = "uuid"))]
    let data = InsertComplex::new("complex_user", true, common::Role::User)
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_score(95.5)
        .with_description("Test description".to_string())
        .with_data_blob(vec![1, 2, 3, 4]);

    #[cfg(feature = "uuid")]
    let data = InsertComplex::new("complex_user", true, common::Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_score(95.5)
        .with_description("Test description".to_string())
        .with_data_blob(vec![1, 2, 3, 4]);

    let result = drizzle_exec!(drizzle.insert(complex).values([data]).execute());

    assert_eq!(result, 1);

    // Verify insertion by selecting the record
    let results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((
                complex.id,
                complex.name,
                complex.email,
                complex.age,
                complex.description,
            ))
            .from(complex)
            .r#where(eq(Complex::name, "complex_user"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "complex_user");
    assert_eq!(results[0].email, Some("test@example.com".to_string()));
    assert_eq!(results[0].age, Some(25));
    assert_eq!(results[0].description, Some("Test description".to_string()));
}

#[tokio::test]
async fn conflict_resolution() {
    let db = setup_test_db!();
    let (drizzle, (simple, ..)) = drizzle!(db, [Simple, Complex]);

    // Insert initial Simple record
    let initial_data = InsertSimple::new("conflict_test").with_id(1);

    drizzle_exec!(drizzle.insert(simple).values([initial_data]).execute());

    // Try to insert duplicate - should conflict and be ignored
    let duplicate_data = InsertSimple::new("conflict_test").with_id(1);
    let result = drizzle_exec!(
        drizzle
            .insert(simple)
            .values([duplicate_data])
            .on_conflict(Conflict::default())
            .execute()
    );

    assert_eq!(result, 0); // No rows affected due to conflict

    // Verify only one record exists
    let results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "conflict_test"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "conflict_test");
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[tokio::test]
async fn feature_gated_insert() {
    let db = setup_test_db!();
    let (drizzle, (.., complex)) = drizzle!(db, [Simple, Complex]);

    // Insert Complex record using feature-gated fields
    let data = InsertComplex::new("feature_test", true, common::Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_metadata(common::UserMetadata {
            preferences: vec!["dark_mode".to_string()],
            last_login: Some("2023-01-01".to_string()),
            theme: "dark".to_string(),
        })
        .with_config(common::UserConfig {
            notifications: true,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    let result = drizzle_exec!(drizzle.insert(complex).values([data]).execute());

    assert_eq!(result, 1);

    // Verify insertion
    let results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((
                complex.id,
                complex.name,
                complex.email,
                complex.age,
                complex.description,
            ))
            .from(complex)
            .r#where(eq(complex.name, "feature_test"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_test");
}
