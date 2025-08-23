#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use common::{Complex, InsertComplex, UpdateComplex};
use common::{InsertSimple, Simple, UpdateSimple, setup_db};
use drizzle_macros::drizzle_test;
use drizzle_rs::prelude::*;

#[cfg(feature = "rusqlite")]
use rusqlite::Row;

#[cfg(feature = "uuid")]
use crate::common::ComplexSchema;
use crate::common::SimpleSchema;

mod common;

#[derive(Debug, FromRow)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(not(feature = "uuid"))]
#[derive(FromRow, Debug)]
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
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

drizzle_test!(simple_update, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    // Insert initial Simple record
    let insert_data = InsertSimple::new("original");
    let insert_result = drizzle_exec!(db.insert(simple).values([insert_data]).execute());
    assert_eq!(insert_result, 1);

    // Update the record
    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("updated"))
        .r#where(eq(Simple::name, "original"));
    println!("{}", stmt.to_sql());
    let update_result = drizzle_exec!(stmt.execute());
    assert_eq!(update_result, 1);

    // Verify the update by selecting the record
    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "updated");

    // Verify original name is gone
    let old_results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "original"))
            .all()
    );

    assert_eq!(old_results.len(), 0);
});

#[cfg(feature = "uuid")]
drizzle_test!(complex_update, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Insert initial Complex record
    #[cfg(not(feature = "uuid"))]
    let insert_data = InsertComplex::new("user", true, common::Role::User)
        .with_email("old@example.com".to_string())
        .with_age(25)
        .with_description("Original description".to_string());

    #[cfg(feature = "uuid")]
    let insert_data = InsertComplex::new("user", true, common::Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_email("old@example.com".to_string())
        .with_age(25)
        .with_description("Original description".to_string());

    let insert_result = drizzle_exec!(db.insert(complex).values([insert_data]).execute());
    assert_eq!(insert_result, 1);

    // Update multiple fields
    let stmt = db
        .update(complex)
        .set(
            UpdateComplex::default()
                .with_email("new@example.com".to_string())
                .with_age(30)
                .with_description("Updated description".to_string()),
        )
        .r#where(eq(Complex::name, "user"));
    println!("{}", stmt.to_sql());
    let update_result = drizzle_exec!(stmt.execute());
    assert_eq!(update_result, 1);

    // Verify the update by selecting the record
    let results: Vec<ComplexResult> = drizzle_exec!(
        db.select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(complex.name, "user"))
        .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "user");
    assert_eq!(results[0].email, Some("new@example.com".to_string()));
    assert_eq!(results[0].age, Some(30));
    assert_eq!(
        results[0].description,
        Some("Updated description".to_string())
    );
});

#[cfg(all(feature = "serde", feature = "uuid"))]
drizzle_test!(feature_gated_update, ComplexSchema, {
    let ComplexSchema { complex } = schema;
    // Insert initial Complex record with UUID
    let test_id = uuid::Uuid::new_v4();
    let insert_data = InsertComplex::new("feature_user", true, common::Role::User)
        .with_id(test_id)
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

    let insert_result = drizzle_exec!(db.insert(complex).values([insert_data]).execute());
    assert_eq!(insert_result, 1);

    // Update feature-gated fields using UUID primary key
    let stmt = db
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
        .r#where(eq(Complex::id, test_id));
    println!("{}", stmt.to_sql());
    let update_result = drizzle_exec!(stmt.execute());
    assert_eq!(update_result, 1);

    // Verify the update by selecting with UUID
    let results: Vec<ComplexResult> = drizzle_exec!(
        db.select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(complex.id, test_id))
        .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_user");
    assert_eq!(results[0].id, test_id);
});
