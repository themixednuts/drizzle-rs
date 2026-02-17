#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{Complex, ComplexSchema, InsertComplex, UpdateComplex};
use crate::common::schema::sqlite::{InsertSimple, Simple, SimpleSchema, UpdateSimple};
#[cfg(all(feature = "serde", feature = "uuid"))]
use crate::common::schema::sqlite::{UserConfig, UserMetadata};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[allow(dead_code)]
#[derive(SQLiteFromRow, Debug)]
struct ComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

sqlite_test!(simple_update, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    // Insert initial Simple record
    let insert_data = InsertSimple::new("original");
    let insert_result = drizzle_exec!(db.insert(simple).values([insert_data]) => execute);
    assert_eq!(insert_result, 1);

    // Update the record
    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("updated"))
        .r#where(eq(Simple::name, "original"));
    let update_result = drizzle_exec!(stmt => execute);
    assert_eq!(update_result, 1);

    // Verify the update by selecting the record
    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated"))
            => all_as
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "updated");

    // Verify original name is gone
    let old_results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "original"))
            => all_as
    );

    assert_eq!(old_results.len(), 0);
});

#[cfg(feature = "uuid")]
sqlite_test!(complex_update, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Insert initial Complex record
    #[cfg(not(feature = "uuid"))]
    let insert_data = InsertComplex::new("user", true, Role::User)
        .with_email("old@example.com".to_string())
        .with_age(25)
        .with_description("Original description".to_string());

    #[cfg(feature = "uuid")]
    let insert_data = InsertComplex::new("user", true, Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_email("old@example.com".to_string())
        .with_age(25)
        .with_description("Original description".to_string());

    let insert_result = drizzle_exec!(db.insert(complex).values([insert_data]) => execute);
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
    let update_result = drizzle_exec!(stmt => execute);
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
        => all_as
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

sqlite_test!(update_multiple_rows, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("test_one"),
        InsertSimple::new("test_two"),
        InsertSimple::new("other"),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("updated"))
        .r#where(like(Simple::name, "test%"));
    drizzle_exec!(stmt => execute);

    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated"))
            => all_as
    );
    assert_eq!(results.len(), 2);

    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "other"))
            => all_as
    );
    assert_eq!(results.len(), 1);
});

#[cfg(feature = "uuid")]
sqlite_test!(update_with_complex_where, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Young", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_age(16),
        InsertComplex::new("Adult", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_age(25),
        InsertComplex::new("Senior", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_age(70),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(complex)
        .set(UpdateComplex::default().with_name("matched"))
        .r#where(and([gte(complex.age, 18), lte(complex.age, 65)]));
    drizzle_exec!(stmt => execute);

    let results: Vec<ComplexResult> = drizzle_exec!(
        db.select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(complex.name, "matched"))
        => all_as
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "matched");
});

sqlite_test!(update_with_in_condition, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
        InsertSimple::new("David"),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("Updated"))
        .r#where(in_array(simple.name, ["Alice", "Charlie"]));
    drizzle_exec!(stmt => execute);

    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "Updated"))
            => all_as
    );
    assert_eq!(results.len(), 2);

    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(in_array(simple.name, ["Bob", "David"]))
            => all_as
    );
    assert_eq!(results.len(), 2);
});

sqlite_test!(update_no_matching_rows, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("Updated"))
        .r#where(eq(simple.name, "NonExistent"));
    drizzle_exec!(stmt => execute);

    let results: Vec<SimpleResult> =
        drizzle_exec!(db.select((simple.id, simple.name)).from(simple) => all_as);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

#[cfg(all(feature = "serde", feature = "uuid"))]
sqlite_test!(feature_gated_update, ComplexSchema, {
    let ComplexSchema { complex } = schema;
    // Insert initial Complex record with UUID
    let test_id = uuid::Uuid::new_v4();
    let insert_data = InsertComplex::new("feature_user", true, Role::User)
        .with_id(test_id)
        .with_metadata(UserMetadata {
            preferences: vec!["user_mode".to_string()],
            last_login: Some("2023-01-15".to_string()),
            theme: "light".to_string(),
        })
        .with_config(UserConfig {
            notifications: true,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    let insert_result = drizzle_exec!(db.insert(complex).values([insert_data]) => execute);
    assert_eq!(insert_result, 1);

    // Update feature-gated fields using UUID primary key
    let stmt = db
        .update(complex)
        .set(
            UpdateComplex::default()
                .with_metadata(UserMetadata {
                    preferences: vec!["admin_mode".to_string(), "updated".to_string()],
                    last_login: Some("2023-12-15".to_string()),
                    theme: "admin".to_string(),
                })
                .with_config(UserConfig {
                    notifications: false,
                    language: "en".to_string(),
                    settings: std::collections::HashMap::from([(
                        "updated".to_string(),
                        "true".to_string(),
                    )]),
                }),
        )
        .r#where(eq(Complex::id, test_id));
    let update_result = drizzle_exec!(stmt => execute);
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
        => all_as
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_user");
    assert_eq!(results[0].id, test_id);
});
