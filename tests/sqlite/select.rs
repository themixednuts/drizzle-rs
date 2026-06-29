#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::InsertComplex;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema};
#[cfg(feature = "serde")]
use crate::common::schema::sqlite::{UserConfig, UserMetadata};

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;

#[cfg(feature = "uuid")]
#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

#[drizzle::test]
fn simple_select_with_conditions(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Insert test data
    let test_data = vec![
        InsertSimple::new("alpha"),
        InsertSimple::new("beta"),
        InsertSimple::new("gamma"),
        InsertSimple::new("delta"),
    ];

    let stmt = db.insert(simple).values(test_data);
    stmt.execute();

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "beta"));

    // Test WHERE condition
    let where_results: Vec<SelectSimple> = stmt.all();

    assert_eq!(where_results.len(), 1);
    assert_eq!(where_results[0].name, "beta");

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(2);
    // Test ORDER BY with LIMIT
    let ordered_results: Vec<SelectSimple> = stmt.all();

    assert_eq!(ordered_results.len(), 2);
    assert_eq!(ordered_results[0].name, "alpha");
    assert_eq!(ordered_results[1].name, "beta");

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(2)
        .offset(2);

    // Test LIMIT with OFFSET
    let offset_results: Vec<SelectSimple> = stmt.all();

    assert_eq!(offset_results.len(), 2);
    assert_eq!(offset_results[0].name, "delta");
    assert_eq!(offset_results[1].name, "gamma");
}

#[drizzle::test]
fn select_limit_offset_with_placeholders(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([
            InsertSimple::new("alpha").with_id(1),
            InsertSimple::new("beta").with_id(2),
            InsertSimple::new("gamma").with_id(3),
            InsertSimple::new("delta").with_id(4),
        ])
        .execute();

    let literal = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(2)
        .offset(1);
    assert_eq!(
        literal.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" ORDER BY "simple"."name" ASC LIMIT 2 OFFSET 1"#
    );

    let untyped_limit = drizzle::core::Placeholder::named("limit");
    let untyped_offset = drizzle::core::Placeholder::named("offset");
    let untyped_stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(untyped_limit)
        .offset(untyped_offset);
    assert_eq!(
        untyped_stmt.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" ORDER BY "simple"."name" ASC LIMIT :limit OFFSET :offset"#
    );

    let limit = simple.id.placeholder("limit");
    let offset = simple.id.placeholder("offset");
    let prepared = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.name)])
        .limit(limit)
        .offset(offset)
        .prepare()
        .into_owned();

    let rows: Vec<SelectSimple> = prepared.all(db.conn(), [limit.bind(2), offset.bind(1)]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].name, "beta");
    assert_eq!(rows[1].name, "delta");
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn complex_select_with_conditions(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex } = schema;
    // Insert test data with different ages
    #[cfg(not(feature = "uuid"))]
    let test_data = [
        InsertComplex::new("young", true, Role::User)
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::new("middle", true, Role::User)
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::new("old", true, Role::User)
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    #[cfg(feature = "uuid")]
    let test_data = [
        InsertComplex::new("young", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::new("middle", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::new("old", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    let stmt = db.insert(complex).values(test_data);
    stmt.execute();

    // Test complex WHERE with GT condition
    let gt_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(gt(complex.age, 25))
        .all();

    assert_eq!(gt_results.len(), 2);
    let names: Vec<String> = gt_results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"middle".to_string()));
    assert!(names.contains(&"old".to_string()));

    // Test complex WHERE with range conditions (AND logic)
    let range_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(and(gte(complex.age, 25), lt(complex.age, 45)))
        .all();

    assert_eq!(range_results.len(), 1);
    assert_eq!(range_results[0].name, "middle");
    assert_eq!(range_results[0].age, Some(35));
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[drizzle::test]
fn feature_gated_select(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex } = schema;

    // Insert Complex record with feature-gated fields
    let test_id = uuid::Uuid::new_v4();
    let data = InsertComplex::new("feature_user", true, Role::User)
        .with_id(test_id)
        .with_metadata(UserMetadata {
            preferences: vec!["admin_panel".to_string()],
            last_login: Some("2023-12-01".to_string()),
            theme: "admin".to_string(),
        })
        .with_config(UserConfig {
            notifications: false,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    db.insert(complex).values([data]).execute();

    // Query using UUID field
    let uuid_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(eq(complex.id, test_id))
        .all();

    assert_eq!(uuid_results.len(), 1);
    assert_eq!(uuid_results[0].name, "feature_user");

    // Query using name to verify metadata exists (can't easily verify content without custom result type)
    let metadata_results: Vec<ComplexResult> = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .r#where(eq(complex.name, "feature_user"))
        .all();

    assert_eq!(metadata_results.len(), 1);
    assert_eq!(metadata_results[0].name, "feature_user");
}
