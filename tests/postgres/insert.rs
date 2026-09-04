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
        .select_raw(SQL::raw("SELECT 9001, 'pg_from_select'"))
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
fn insert_selected_columns_from_checked_select(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    db.insert(simple)
        .value(InsertSimple::new("checked_source").with_id(9001))
        .execute();

    let source = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>()
        .select(simple.name)
        .from(simple)
        .r#where(eq(simple.id, 9001));
    let stmt = db.insert(simple).columns(simple.name).select(source);

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("name") SELECT "simple"."name" FROM "simple" WHERE "simple"."id" = $1"#
    );
    let inserted = stmt.execute();
    assert_eq!(inserted, 1);

    let copied: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "checked_source"))
        .all();
    assert_eq!(copied.len(), 2);
    assert!(copied.iter().any(|row| row.id != 9001));

    let borrowed = db.insert(&simple).columns(simple.name).select(
        drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>()
            .select(simple.name)
            .from(simple)
            .r#where(eq(simple.id, -1)),
    );
    assert_eq!(
        borrowed.to_sql().sql(),
        r#"INSERT INTO "simple" ("name") SELECT "simple"."name" FROM "simple" WHERE "simple"."id" = $1"#
    );
}

#[drizzle::test]
fn checked_full_insert_select_names_every_target_column(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let source = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>()
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, -1));
    let stmt = db.insert(simple).select(source);

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") SELECT "simple"."id", "simple"."name" FROM "simple" WHERE "simple"."id" = $1"#
    );
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

// =============================================================================
// ON CONFLICT (upsert)
// =============================================================================

#[drizzle::test]
fn upsert_do_nothing_without_target_skips_duplicates(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("first").with_id(1)])
        .on_conflict_do_nothing();
    let sql = stmt.to_sql().sql();
    assert!(
        sql.ends_with("ON CONFLICT DO NOTHING"),
        "unexpected SQL: {sql}"
    );

    db.insert(simple)
        .values([InsertSimple::new("original").with_id(10)])
        .execute();
    let affected = db
        .insert(simple)
        .values([InsertSimple::new("duplicate").with_id(10)])
        .on_conflict_do_nothing()
        .execute();
    assert_eq!(affected, 0);

    let rows: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 10))
        .all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "original");
}

#[drizzle::test]
fn upsert_do_nothing_on_column_keeps_existing_row(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("first").with_id(1)])
        .on_conflict(simple.id)
        .do_nothing();
    let sql = stmt.to_sql().sql();
    assert!(
        sql.ends_with(r#"ON CONFLICT ("id") DO NOTHING"#),
        "unexpected SQL: {sql}"
    );

    db.insert(simple)
        .values([InsertSimple::new("first").with_id(20)])
        .execute();
    db.insert(simple)
        .values([InsertSimple::new("second").with_id(20)])
        .on_conflict(simple.id)
        .do_nothing()
        .execute();

    let rows: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 20))
        .all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "first");
}

#[drizzle::test]
fn upsert_do_update_replaces_existing_row(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated"));
    let sql = stmt.to_sql().sql();
    assert!(
        sql.ends_with(r#"ON CONFLICT ("id") DO UPDATE SET "name" = $3"#),
        "unexpected SQL: {sql}"
    );

    db.insert(simple)
        .values([InsertSimple::new("before").with_id(30)])
        .execute();
    let affected = db
        .insert(simple)
        .values([InsertSimple::new("ignored").with_id(30)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("after"))
        .execute();
    assert_eq!(affected, 1);

    let rows: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 30))
        .all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "after");
}

#[drizzle::test]
fn upsert_do_update_where_gates_the_update(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated"))
        .r#where(gt(simple.id, 0));
    let sql = stmt.to_sql().sql();
    assert!(
        sql.ends_with(r#"DO UPDATE SET "name" = $3 WHERE "simple"."id" > $4"#),
        "unexpected SQL: {sql}"
    );

    db.insert(simple)
        .values([InsertSimple::new("original").with_id(40)])
        .execute();

    // A false predicate leaves the existing row alone.
    let untouched = db
        .insert(simple)
        .values([InsertSimple::new("ignored").with_id(40)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("never"))
        .r#where(gt(simple.id, 1_000))
        .execute();
    assert_eq!(untouched, 0);
    let rows: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 40))
        .all();
    assert_eq!(rows[0].name, "original");

    // A true predicate applies the update.
    let updated = db
        .insert(simple)
        .values([InsertSimple::new("ignored").with_id(40)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated_via_where"))
        .r#where(gt(simple.id, 0))
        .execute();
    assert_eq!(updated, 1);
    let rows: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 40))
        .all();
    assert_eq!(rows[0].name, "updated_via_where");
}

#[drizzle::test]
fn upsert_do_update_uses_excluded_values(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name(excluded(simple.name)));
    let sql = stmt.to_sql().sql();
    assert!(
        sql.ends_with(r#"ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name""#),
        "unexpected SQL: {sql}"
    );

    db.insert(simple)
        .values([InsertSimple::new("old_name").with_id(50)])
        .execute();
    db.insert(simple)
        .values([InsertSimple::new("new_name").with_id(50)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name(excluded(simple.name)))
        .execute();

    let rows: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 50))
        .all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "new_name");
}

#[drizzle::test]
fn upsert_returning_reports_the_stored_row(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("before").with_id(60)])
        .execute();

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("proposed").with_id(60)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name(excluded(simple.name)))
        .returning((simple.id, simple.name));
    let sql = stmt.to_sql().sql();
    assert!(
        sql.ends_with(r#"RETURNING "simple"."id", "simple"."name""#),
        "unexpected SQL: {sql}"
    );

    let returned: Vec<SelectSimple> = stmt.all();
    assert_eq!(returned.len(), 1);
    assert_eq!(returned[0].id, 60);
    assert_eq!(returned[0].name, "proposed");
}
