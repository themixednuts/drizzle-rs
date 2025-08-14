use common::{Complex, InsertComplex, InsertSimple, Post, Simple};
use drizzle_core::OrderBy;
use drizzle_rs::prelude::*;
use procmacros::FromRow;

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
}

#[cfg(feature = "uuid")]
#[derive(Debug, FromRow)]
struct ComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

#[derive(Debug, FromRow)]
struct JoinResult {
    name: String,
    title: String,
}

#[tokio::test]
async fn simple_select_with_conditions() {
    let db = setup_test_db!();
    let (drizzle, (simple, ..)) = drizzle!(db, [Simple, Complex, Post]);

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alpha"),
        InsertSimple::new("beta"),
        InsertSimple::new("gamma"),
        InsertSimple::new("delta"),
    ];

    drizzle_exec!(drizzle.insert(simple).values(test_data).execute());

    // Test WHERE condition
    let where_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "beta"))
            .all()
    );

    assert_eq!(where_results.len(), 1);
    assert_eq!(where_results[0].name, "beta");

    // Test ORDER BY with LIMIT
    let ordered_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .order_by([(simple.name, OrderBy::Asc)])
            .limit(2)
            .all()
    );

    assert_eq!(ordered_results.len(), 2);
    assert_eq!(ordered_results[0].name, "alpha");
    assert_eq!(ordered_results[1].name, "beta");

    // Test LIMIT with OFFSET
    let offset_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .order_by([(simple.name, OrderBy::Asc)])
            .limit(2)
            .offset(2)
            .all()
    );

    assert_eq!(offset_results.len(), 2);
    assert_eq!(offset_results[0].name, "delta");
    assert_eq!(offset_results[1].name, "gamma");
}

#[tokio::test]
async fn complex_select_with_conditions() {
    let db = setup_test_db!();
    let (drizzle, (_, complex, ..)) = drizzle!(db, [Simple, Complex, Post]);

    // Insert test data with different ages
    #[cfg(not(feature = "uuid"))]
    let test_data = [
        InsertComplex::new("young", true, common::Role::User)
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::new("middle", true, common::Role::User)
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::new("old", true, common::Role::User)
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    #[cfg(feature = "uuid")]
    let test_data = [
        InsertComplex::new("young", true, common::Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::new("middle", true, common::Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::new("old", true, common::Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    // println!("Test data: {:?}", test_data);
    let stmt = drizzle.insert(complex).values(test_data);
    // let sql = stmt.to_sql();
    // println!("SQL {sql}");

    drizzle_exec!(stmt.execute());

    // Test complex WHERE with GT condition
    let gt_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            .r#where(gt(complex.age, 25))
            .all()
    );

    assert_eq!(gt_results.len(), 2);
    let names: Vec<String> = gt_results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"middle".to_string()));
    assert!(names.contains(&"old".to_string()));

    // Test complex WHERE with range conditions (AND logic)
    let range_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            .r#where(and([gte(complex.age, 25), lt(complex.age, 45)]))
            .all()
    );

    assert_eq!(range_results.len(), 1);
    assert_eq!(range_results[0].name, "middle");
    assert_eq!(range_results[0].age, Some(35));
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[tokio::test]
async fn feature_gated_select() {
    let db = setup_test_db!();
    let (drizzle, (_, complex, _)) = drizzle!(db, [Simple, Complex, Post]);

    // Insert Complex record with feature-gated fields
    let test_id = uuid::Uuid::new_v4();
    let data = InsertComplex::new("feature_user", true, common::Role::User)
        .with_id(test_id)
        .with_metadata(common::UserMetadata {
            preferences: vec!["admin_panel".to_string()],
            last_login: Some("2023-12-01".to_string()),
            theme: "admin".to_string(),
        })
        .with_config(common::UserConfig {
            notifications: false,
            language: "en".to_string(),
            settings: std::collections::HashMap::new(),
        });

    drizzle_exec!(drizzle.insert(complex).values([data]).execute());

    // Query using UUID field
    let uuid_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            .r#where(eq(complex.id, test_id))
            .all()
    );

    assert_eq!(uuid_results.len(), 1);
    assert_eq!(uuid_results[0].name, "feature_user");

    // Query using name to verify metadata exists (can't easily verify content without custom result type)
    let metadata_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((complex.id, complex.name, complex.email, complex.age))
            .from(complex)
            .r#where(eq(complex.name, "feature_user"))
            .all()
    );

    assert_eq!(metadata_results.len(), 1);
    assert_eq!(metadata_results[0].name, "feature_user");
}
