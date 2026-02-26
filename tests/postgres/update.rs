//! PostgreSQL UPDATE statement tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[allow(dead_code)]
#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    active: bool,
}

postgres_test!(update_single_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Original")]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("Updated"))
        .r#where(eq(simple.name, "Original"));
    drizzle_exec!(stmt => execute);

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Updated");
});

postgres_test!(update_multiple_rows, SimpleSchema, {
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
        .r#where(like(simple.name, "test%"));
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "updated"));
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "other"));
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 1);
});

#[cfg(feature = "uuid")]
postgres_test!(update_multiple_columns, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("Alice", true, Role::User)
            .with_email("old@example.com")
            .with_age(25)]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(complex)
        .set(
            UpdateComplex::default()
                .with_email("new@example.com")
                .with_age(30)
                .with_active(false),
        )
        .r#where(eq(complex.name, "Alice"));
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
    assert_eq!(results[0].email, Some("new@example.com".to_string()));
    assert_eq!(results[0].age, Some(30));
    assert!(!results[0].active);
});

#[cfg(feature = "uuid")]
postgres_test!(update_with_complex_where, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Young", true, Role::User).with_age(16),
        InsertComplex::new("Adult", true, Role::User).with_age(25),
        InsertComplex::new("Senior", true, Role::User).with_age(70),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(complex)
        .set(UpdateComplex::default().with_active(false))
        .r#where(and([gte(complex.age, 18), lte(complex.age, 65)]));
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.active, false));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Adult");
});

postgres_test!(update_with_in_condition, SimpleSchema, {
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

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "Updated"));
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(in_array(simple.name, ["Bob", "David"]));
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);
});

postgres_test!(update_no_matching_rows, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("Updated"))
        .r#where(eq(simple.name, "NonExistent"));
    drizzle_exec!(stmt => execute);

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

postgres_test!(update_with_placeholders_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let new_name = simple.name.placeholder("new_name");
    let old_name = simple.name.placeholder("old_name");

    let update = UpdateSimple::default().with_name(new_name);
    let stmt = db
        .update(simple)
        .set(update)
        .r#where(eq(simple.name, old_name));

    let sql = stmt.to_sql();
    let sql_string = sql.sql();

    assert!(
        sql_string.starts_with("UPDATE"),
        "Should be an UPDATE statement, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("\"simple\""),
        "Should reference the simple table, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$1"),
        "Expected first PostgreSQL placeholder in SQL, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("$2"),
        "Expected second PostgreSQL placeholder in SQL, got: {}",
        sql_string
    );

    let params: Vec<_> = sql.params().collect();
    assert!(
        params.is_empty(),
        "Should have no bound params when using placeholders, got {}",
        params.len()
    );
});

postgres_test!(update_with_placeholders_execute, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("original_name")])
            => execute
    );

    let new_name = simple.name.placeholder("new_name");
    let old_name = simple.name.placeholder("old_name");

    let update = UpdateSimple::default().with_name(new_name);
    let prepared = db
        .update(simple)
        .set(update)
        .r#where(eq(simple.name, old_name))
        .prepare()
        .into_owned();

    let updated = drizzle_exec!(prepared.execute(
        drizzle_client!(),
        [
            new_name.bind("updated_name"),
            old_name.bind("original_name")
        ]
    ));
    assert_eq!(updated, 1);

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "updated_name"));
    let rows: Vec<SelectSimple> = drizzle_exec!(stmt => all);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "updated_name");
});
