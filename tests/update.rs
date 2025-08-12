use common::{Complex, InsertComplex, InsertSimple, Simple, UpdateComplex, UpdateSimple, setup_db};
use drizzle_rs::prelude::*;
use procmacros::FromRow;

#[cfg(feature = "rusqlite")]
use rusqlite::Row;

mod common;

#[derive(Debug, FromRow)]
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
#[derive(Debug)]
struct ComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

impl TryFrom<&Row<'_>> for ComplexResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
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
fn simple_update() {
    let db = setup_db();
    let (drizzle, (simple, ..)) = drizzle!(db, [Simple, Complex]);

    // Insert initial Simple record
    let insert_data = InsertSimple::default().with_name("original");
    let insert_result = drizzle
        .insert(simple)
        .values([insert_data])
        .execute()
        .unwrap();
    assert_eq!(insert_result, 1);

    // Update the record
    let update_result = drizzle
        .update(simple)
        .set(UpdateSimple::default().with_name("updated"))
        .r#where(eq(Simple::name, "original"))
        .execute()
        .unwrap();
    assert_eq!(update_result, 1);

    // Verify the update by selecting the record
    let results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .r#where(eq(Simple::name, "updated"))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "updated");

    // Verify original name is gone
    let old_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .r#where(eq(Simple::name, "original"))
        .all()
        .unwrap();

    assert_eq!(old_results.len(), 0);
}

#[test]
fn complex_update() {
    let db = setup_db();
    let (drizzle, (.., complex)) = drizzle!(db, [Simple, Complex]);

    // Insert initial Complex record
    #[cfg(not(feature = "uuid"))]
    let insert_data = InsertComplex::default()
        .with_name("user")
        .with_email("old@example.com".to_string())
        .with_age(25)
        .with_active(true)
        .with_role(common::Role::User)
        .with_description("Original description".to_string());

    #[cfg(feature = "uuid")]
    let insert_data = InsertComplex::default()
        .with_id(uuid::Uuid::new_v4())
        .with_name("user")
        .with_email("old@example.com".to_string())
        .with_age(25)
        .with_active(true)
        .with_role(common::Role::User)
        .with_description("Original description".to_string());

    let insert_result = drizzle
        .insert(complex)
        .values([insert_data])
        .execute()
        .unwrap();
    assert_eq!(insert_result, 1);

    // Update multiple fields
    let update_result = drizzle
        .update(complex)
        .set(
            UpdateComplex::default()
                .with_email("new@example.com".to_string())
                .with_age(30)
                .with_description("Updated description".to_string()),
        )
        .r#where(eq(Complex::name, "user"))
        .execute()
        .unwrap();
    assert_eq!(update_result, 1);

    // Verify the update by selecting the record
    let results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from(complex)
        .r#where(eq(Complex::name, "user"))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "user");
    assert_eq!(results[0].email, Some("new@example.com".to_string()));
    assert_eq!(results[0].age, Some(30));
    assert_eq!(
        results[0].description,
        Some("Updated description".to_string())
    );
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[test]
fn feature_gated_update() {
    let db = setup_db();
    let (drizzle, (.., complex)) = drizzle!(db, [Simple, Complex]);

    // Insert initial Complex record with UUID
    let test_id = uuid::Uuid::new_v4();
    let insert_data = InsertComplex::default()
        .with_id(test_id)
        .with_name("feature_user")
        .with_active(true)
        .with_role(common::Role::User)
        .with_metadata(common::UserMetadata {
            preferences: vec!["user_mode".to_string()],
            last_login: Some("2023-01-15".to_string()),
            theme: "light".to_string(),
        })
        .with_config(common::UserConfig {
            notifications: true,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    let insert_result = drizzle
        .insert(complex)
        .values([insert_data])
        .execute()
        .unwrap();
    assert_eq!(insert_result, 1);

    // Update feature-gated fields using UUID primary key
    let update_result = drizzle
        .update(complex)
        .set(
            UpdateComplex::default()
                .with_metadata(common::UserMetadata {
                    preferences: vec!["admin_mode".to_string(), "updated".to_string()],
                    last_login: Some("2023-12-15".to_string()),
                    theme: "admin".to_string(),
                })
                .with_config(common::UserConfig {
                    notifications: false,
                    language: "en".to_string(),
                    settings: std::collections::HashMap::from([(
                        "updated".to_string(),
                        "true".to_string(),
                    )]),
                }),
        )
        .r#where(eq(Complex::id, test_id))
        .execute()
        .unwrap();
    assert_eq!(update_result, 1);

    // Verify the update by selecting with UUID
    let results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from(complex)
        .r#where(eq(Complex::id, test_id))
        .all()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_user");
    assert_eq!(results[0].id, test_id);
}
