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
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;
use crate::common::schema::sqlite::{SelectSimple, SimpleSchema};

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

#[cfg(feature = "turso")]
#[SQLiteTable(STRICT)]
struct StrictAutoRow {
    #[column(PRIMARY)]
    id: i64,
    marker: Option<Vec<u8>>,
    name: String,
}

#[cfg(feature = "turso")]
#[derive(SQLiteSchema)]
struct StrictAutoSchema {
    rows: StrictAutoRow,
}

#[drizzle::test]
fn simple_insert(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert Simple record
    let data = InsertSimple::new("test");
    let result = db.insert(simple).values([data]).execute();

    assert_eq!(result, 1);

    // Verify insertion by selecting the record
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "test"))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
}

#[drizzle::test]
fn insert_with_table_and_column_refs(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let simple_ref = &simple;
    let name_ref = &simple.name;

    let data = InsertSimple::new("ref_test");
    let result = db.insert(simple_ref).values([data]).execute();
    assert_eq!(result, 1);

    let results: Vec<SelectSimple> = db
        .select((simple_ref.id, simple_ref.name))
        .from(simple_ref)
        .r#where(eq(name_ref, "ref_test"))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "ref_test");
}

#[drizzle::test]
fn insert_from_select(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .select(SQL::raw("SELECT 42, 'from_select'"));

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" SELECT 42, 'from_select'"#
    );

    let rows = stmt.execute();
    assert_eq!(rows, 1);

    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 42))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "from_select");
}

#[drizzle::test]
fn insert_returning_star(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("returning_star").with_id(101)])
        .returning(());

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) RETURNING *"#
    );

    let rows: Vec<SelectSimple> = stmt.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 101);
    assert_eq!(rows[0].name, "returning_star");
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_get_finishes_returning_cursor_before_another_connection_writes() {
    let path = crate::common::helpers::temp_db_path();
    let path_text = path
        .to_str()
        .expect("temporary sqlite path must be valid UTF-8");
    let database = turso::Builder::new_local(path_text)
        .experimental_index_method(true)
        .experimental_mvcc_passive_checkpoint(true)
        .build()
        .await
        .expect("build Turso database");
    let first_connection = database.connect().expect("first connection");
    let second_connection = database.connect().expect("second connection");
    second_connection
        .busy_timeout(std::time::Duration::from_millis(50))
        .expect("set writer busy timeout");
    let (mut first, SimpleSchema { simple }) =
        drizzle::sqlite::turso::Drizzle::new(first_connection, SimpleSchema::new());
    let (second, _) = drizzle::sqlite::turso::Drizzle::new(second_connection, SimpleSchema::new());
    first.create().await.expect("create schema");

    let missing: drizzle::Result<SelectSimple> = first
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 999))
        .limit(1)
        .get()
        .await;
    assert!(matches!(
        missing,
        Err(drizzle::error::DrizzleError::NotFound)
    ));

    let returned: SelectSimple = first
        .insert(simple)
        .values([InsertSimple::new("first").with_id(1)])
        .returning(())
        .get()
        .await
        .expect("insert returning one row");
    assert_eq!(returned.id, 1);

    second
        .insert(simple)
        .values([InsertSimple::new("second").with_id(2)])
        .execute()
        .await
        .expect("the completed returning cursor must release its write transaction");

    let prepared = first
        .insert(simple)
        .values([InsertSimple::new("prepared").with_id(3)])
        .returning(())
        .prepare();
    let returned: SelectSimple = prepared
        .get(first.conn(), [])
        .await
        .expect("prepared insert returning one row");
    assert_eq!(returned.id, 3);

    second
        .insert(simple)
        .values([InsertSimple::new("after-prepared").with_id(4)])
        .execute()
        .await
        .expect("the completed prepared cursor must release its write transaction");

    first
        .transaction(
            drizzle::sqlite::connection::SQLiteTransactionType::Immediate,
            async |transaction| {
                let returned: SelectSimple = transaction
                    .insert(simple)
                    .values([InsertSimple::new("transaction").with_id(5)])
                    .returning(())
                    .get()
                    .await?;
                assert_eq!(returned.id, 5);
                Ok(())
            },
        )
        .await
        .expect("commit insert returning transaction");

    let visible: Vec<SelectSimple> = second.select(()).from(simple).all().await.unwrap();
    assert_eq!(visible.len(), 5);

    let mut transaction_context = first.clone();
    transaction_context
        .transaction(
            drizzle::sqlite::connection::SQLiteTransactionType::Immediate,
            async |transaction| {
                let existing: Vec<SelectSimple> = transaction
                    .select(())
                    .from(simple)
                    .r#where(eq(simple.id, 6))
                    .all()
                    .await?;
                assert!(existing.is_empty());
                let _: SelectSimple = transaction
                    .insert(simple)
                    .values([InsertSimple::new("cloned-transaction").with_id(6)])
                    .returning(())
                    .get()
                    .await?;
                transaction
                    .update(simple)
                    .set(UpdateSimple::default().with_name("updated-cloned-transaction"))
                    .r#where(eq(simple.id, 6))
                    .execute()
                    .await?;
                Ok(())
            },
        )
        .await
        .expect("commit cloned transaction context");

    let visible: Vec<SelectSimple> = second.select(()).from(simple).all().await.unwrap();
    assert_eq!(visible.len(), 6);
    assert_eq!(
        visible.iter().find(|row| row.id == 6).unwrap().name,
        "updated-cloned-transaction"
    );
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_strict_insert_returning_generates_integer_primary_key() {
    let database = turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("build Turso database");
    let connection = database.connect().expect("connect Turso database");
    let (mut db, StrictAutoSchema { rows }) =
        drizzle::sqlite::turso::Drizzle::new(connection, StrictAutoSchema::new());
    db.create().await.expect("create strict schema");

    let returned: SelectStrictAutoRow = db
        .transaction(
            drizzle::sqlite::connection::SQLiteTransactionType::Immediate,
            async |transaction| {
                transaction
                    .insert(rows)
                    .values([InsertStrictAutoRow::new("generated")])
                    .returning(())
                    .get()
                    .await
            },
        )
        .await
        .expect("insert returning must observe SQLite's generated rowid");

    assert_eq!(returned.id, 1);
    assert_eq!(returned.marker, None);
    assert_eq!(returned.name, "generated");
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn complex_insert(db: &mut TestDb<ComplexSchema>) {
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

    let result = db.insert(complex).values([data]).execute();

    assert_eq!(result, 1);

    // Verify insertion by selecting the record
    let results: Vec<ComplexResult> = db
        .select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(Complex::name, "complex_user"))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "complex_user");
    assert_eq!(results[0].email, Some("test@example.com".to_string()));
    assert_eq!(results[0].age, Some(25));
    assert_eq!(results[0].description, Some("Test description".to_string()));
}

#[drizzle::test]
fn conflict_resolution(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial Simple record
    let initial_data = InsertSimple::new("conflict_test").with_id(1);

    db.insert(simple).values([initial_data]).execute();

    // Try to insert duplicate - should conflict and be ignored
    let duplicate_data = InsertSimple::new("conflict_test").with_id(1);
    let stmt = db
        .insert(simple)
        .values([duplicate_data])
        .on_conflict_do_nothing();
    let result = stmt.execute();

    assert_eq!(result, 0); // No rows affected due to conflict

    // Verify only one record exists
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "conflict_test"))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "conflict_test");
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[drizzle::test]
fn feature_gated_insert(db: &mut TestDb<ComplexSchema>) {
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
    let result = stmt.execute();

    assert_eq!(result, 1);

    // Verify insertion
    let results: Vec<ComplexResult> = db
        .select((
            complex.id,
            complex.name,
            complex.email,
            complex.age,
            complex.description,
        ))
        .from(complex)
        .r#where(eq(complex.name, "feature_test"))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "feature_test");
}

// SQL generation tests for ON CONFLICT variants
#[drizzle::test]
fn on_conflict_do_nothing_no_target_sql(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("test").with_id(1)])
        .on_conflict_do_nothing();

    assert_eq!(
        stmt.to_sql().sql(),
        r#"INSERT INTO "simple" ("id", "name") VALUES (?, ?) ON CONFLICT DO NOTHING"#
    );

    // Also verify via DB execution: insert then conflict should be silently ignored

    db.insert(simple)
        .values([InsertSimple::new("original").with_id(10)])
        .execute();

    db.insert(simple)
        .values([InsertSimple::new("duplicate").with_id(10)])
        .on_conflict_do_nothing()
        .execute();
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 10))
        .all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "original");
}

#[drizzle::test]
fn on_conflict_column_do_nothing_sql(db: &mut TestDb<SimpleSchema>) {
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

    // Also verify via DB execution

    db.insert(simple)
        .values([InsertSimple::new("first").with_id(20)])
        .execute();

    db.insert(simple)
        .values([InsertSimple::new("second").with_id(20)])
        .on_conflict(simple.id)
        .do_nothing()
        .execute();
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 20))
        .all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "first");
}

#[drizzle::test]
fn on_conflict_do_update_sql(db: &mut TestDb<SimpleSchema>) {
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

    // Also verify via DB execution

    db.insert(simple)
        .values([InsertSimple::new("before").with_id(30)])
        .execute();

    db.insert(simple)
        .values([InsertSimple::new("ignored").with_id(30)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("after"))
        .execute();
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 30))
        .all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "after");
}

#[drizzle::test]
fn on_conflict_do_update_where_sql(db: &mut TestDb<SimpleSchema>) {
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

    // Also verify via DB execution: WHERE condition should gate the update

    db.insert(simple)
        .values([InsertSimple::new("original").with_id(40)])
        .execute();
    // Conflict with WHERE id > 0 (true) — should update

    db.insert(simple)
        .values([InsertSimple::new("ignored").with_id(40)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated_via_where"))
        .r#where(gt(simple.id, 0))
        .execute();
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 40))
        .all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "updated_via_where");
}

#[drizzle::test]
fn on_conflict_do_update_e2e(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial row

    db.insert(simple)
        .values([InsertSimple::new("original").with_id(1)])
        .execute();

    // Insert conflicting row with do_update — should update the name
    let result = db
        .insert(simple)
        .values([InsertSimple::new("ignored").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name("updated"))
        .execute();

    assert_eq!(result, 1);

    // Verify the name was updated
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 1))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "updated");
}

#[drizzle::test]
fn on_conflict_do_update_excluded_sql(db: &mut TestDb<SimpleSchema>) {
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

    // Also verify via DB execution: EXCLUDED refers to the proposed insert value

    db.insert(simple)
        .values([InsertSimple::new("old_name").with_id(50)])
        .execute();

    db.insert(simple)
        .values([InsertSimple::new("new_name").with_id(50)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name(excluded(simple.name)))
        .execute();
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 50))
        .all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "new_name");
}

#[drizzle::test]
fn on_conflict_do_update_excluded_e2e(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert initial row

    db.insert(simple)
        .values([InsertSimple::new("original").with_id(1)])
        .execute();

    // Upsert with excluded — should update name to the proposed insert value
    let result = db
        .insert(simple)
        .values([InsertSimple::new("from_excluded").with_id(1)])
        .on_conflict(simple.id)
        .do_update(UpdateSimple::default().with_name(excluded(simple.name)))
        .execute();

    assert_eq!(result, 1);

    // Verify the name was updated to the EXCLUDED value
    let results: Vec<SelectSimple> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.id, 1))
        .all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "from_excluded");
}
