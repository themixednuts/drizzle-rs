use common::{Complex, InsertComplex, InsertSimple, Simple, setup_db};
use drizzle_rs::prelude::*;
use drizzle_rs::sqlite::builder::Conflict;
use rusqlite::Row;

mod common;

#[derive(Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

impl TryFrom<&Row<'_>> for SimpleResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<SimpleResult, rusqlite::Error> {
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
    description: Option<String>,
}

#[cfg(feature = "uuid")]
#[derive(Debug)]
struct ComplexResult {
    id: String,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

impl TryFrom<&Row<'_>> for ComplexResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<ComplexResult, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
            age: row.get(3)?,
            description: row.get(4)?,
        })
    }
}

#[test]
fn simple_insert() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    // Insert Simple record
    let data = InsertSimple::default().with_name("test");
    let result = drizzle.insert::<Simple>().values([data]).execute();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // Verify insertion by selecting the record
    let results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from::<Simple>()
        .r#where(eq(Simple::name, "test"))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
}

#[test]
fn complex_insert() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    // Insert Complex record with various field types
    #[cfg(not(feature = "uuid"))]
    let data = InsertComplex::default()
        .with_name("complex_user")
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_score(95.5)
        .with_active(true)
        .with_description("Test description".to_string())
        .with_data_blob(vec![1, 2, 3, 4]);

    #[cfg(feature = "uuid")]
    let data = InsertComplex::default()
        .with_id(uuid::Uuid::new_v4())
        .with_name("complex_user")
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_score(95.5)
        .with_active(true)
        .with_description("Test description".to_string())
        .with_data_blob(vec![1, 2, 3, 4]);

    let result = drizzle.insert::<Complex>().values([data]).execute();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // Verify insertion by selecting the record
    let results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from::<Complex>()
        .r#where(eq(Complex::name, "complex_user"))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "complex_user");
    assert_eq!(results[0].email, Some("test@example.com".to_string()));
    assert_eq!(results[0].age, Some(25));
    assert_eq!(results[0].description, Some("Test description".to_string()));
}

#[test]
fn conflict_resolution() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    // Insert initial Simple record
    let initial_data = InsertSimple::default().with_name("conflict_test");
    drizzle
        .insert::<Simple>()
        .values([initial_data])
        .execute()
        .unwrap();

    // Try to insert duplicate - should conflict and be ignored
    let duplicate_data = InsertSimple::default().with_name("conflict_test");
    let result = drizzle
        .insert::<Simple>()
        .values([duplicate_data])
        .on_conflict(Conflict::default())
        .execute();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // No rows affected due to conflict

    // Verify only one record exists
    let results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from::<Simple>()
        .r#where(eq(Simple::name, "conflict_test"))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "conflict_test");
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[test]
fn feature_gated_insert() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    // Insert Complex record using feature-gated fields
    let data = InsertComplex::default()
        .with_id(uuid::Uuid::new_v4())
        .with_name("feature_test")
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

    let result = drizzle.insert::<Complex>().values([data]).execute();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // Verify insertion
    let results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from::<Complex>()
        .r#where(eq(Complex::name, "feature_test"))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_test");
}
