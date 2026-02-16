#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{Complex, InsertComplex};
use crate::common::schema::sqlite::{InsertSimple, UpdateSimple};
#[cfg(feature = "serde")]
use crate::common::schema::sqlite::{UserConfig, UserMetadata};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;
use crate::common::schema::sqlite::SimpleSchema;

#[allow(dead_code)]
#[derive(SQLiteFromRow, Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[allow(dead_code)]
#[derive(SQLiteFromRow, Debug)]
struct ComplexResult {
    id: Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

sqlite_test!(simple_insert, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert Simple record
    let data = InsertSimple::new("test");
    let result = drizzle_exec!(db.insert(simple).values([data]) => execute);

    assert_eq!(result, 1);

    // Verify insertion by selecting the record
    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "test"))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

#[cfg(feature = "uuid")]
sqlite_test!(complex_insert, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Insert Complex record with various field types
    #[cfg(not(feature = "uuid"))]
    let data = InsertComplex::new("complex_user", true, Role::User)
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_score(95.5)
        .with_description("Test description".to_string())
        .with_data_blob(vec![1, 2, 3, 4]);

    #[cfg(feature = "uuid")]
    let data = InsertComplex::new("complex_user", true, Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_score(95.5)
        .with_description("Test description".to_string())
        .with_data_blob(vec![1, 2, 3, 4]);

    let result = drizzle_exec!(db.insert(complex).values([data]) => execute);

    assert_eq!(result, 1);

    // Verify insertion by selecting the record
    let results: Vec<ComplexResult> = drizzle_exec!(
        db.select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(Complex::name, "complex_user"))
        => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "complex_user");
    assert_eq!(results[0].email, Some("test@example.com".to_string()));
    assert_eq!(results[0].age, Some(25));
    assert_eq!(results[0].description, Some("Test description".to_string()));
});

sqlite_test!(conflict_resolution, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial Simple record
    let initial_data = InsertSimple::new("conflict_test").with_id(1);

    drizzle_exec!(db.insert(simple).values([initial_data]) => execute);

    // Try to insert duplicate - should conflict and be ignored
    let duplicate_data = InsertSimple::new("conflict_test").with_id(1);
    let stmt = db
        .insert(simple)
        .values([duplicate_data])
        .on_conflict_do_nothing();
    let result = drizzle_exec!(stmt => execute);

    assert_eq!(result, 0); // No rows affected due to conflict

    // Verify only one record exists
    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "conflict_test"))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "conflict_test");
});

#[cfg(all(feature = "serde", feature = "uuid"))]
sqlite_test!(feature_gated_insert, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Insert Complex record using feature-gated fields
    let data = InsertComplex::new("feature_test", true, Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_metadata(UserMetadata {
            preferences: vec!["dark_mode".to_string()],
            last_login: Some("2023-01-01".to_string()),
            theme: "dark".to_string(),
        })
        .with_config(UserConfig {
            notifications: true,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    let stmt = db.insert(complex).values([data]);
    let result = drizzle_exec!(stmt => execute);

    assert_eq!(result, 1);

    // Verify insertion
    let results: Vec<ComplexResult> = drizzle_exec!(
        db.select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(complex.name, "feature_test"))
        => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_test");
});

// SQL generation tests for ON CONFLICT variants
sqlite_test!(on_conflict_do_nothing_no_target_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict_do_nothing();

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) ON CONFLICT DO NOTHING"#
    );
});

sqlite_test!(on_conflict_column_do_nothing_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_nothing();

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) ON CONFLICT ("id") DO NOTHING"#
    );
});

sqlite_test!(on_conflict_do_update_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated"));

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) ON CONFLICT ("id") DO UPDATE SET "name" = ?"#
    );
});

sqlite_test!(on_conflict_do_update_where_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated"))
        .r#where(gt(simple.id, 0));

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) ON CONFLICT ("id") DO UPDATE SET "name" = ? WHERE "simple"."id" > ?"#
    );
});

sqlite_test!(on_conflict_do_update_e2e, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial row
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("original").with_id(1)])
            => execute
    );

    // Insert conflicting row with do_update — should update the name
    let result = drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("ignored").with_id(1)])
            .on_conflict(simple.id)
            .do_update(UpdateSimple::default().with_name("updated"))
            => execute
    );

    assert_eq!(result, 1);

    // Verify the name was updated
    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "updated");
});

sqlite_test!(on_conflict_do_update_excluded_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name(excluded(simple.name)));

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name""#
    );
});

sqlite_test!(on_conflict_do_update_excluded_e2e, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial row
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("original").with_id(1)])
            => execute
    );

    // Upsert with excluded — should update name to the proposed insert value
    let result = drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("from_excluded").with_id(1)])
            .on_conflict(simple.id)
            .do_update(UpdateSimple::default().with_name(excluded(simple.name)))
            => execute
    );

    assert_eq!(result, 1);

    // Verify the name was updated to the EXCLUDED value
    let results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "from_excluded");
});
