use common::{Complex, InsertComplex, InsertPost, InsertSimple, Post, Simple, setup_db};
use drizzle_core::OrderBy;
use drizzle_rs::prelude::*;
use rusqlite::Row;

mod common;

#[derive(Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

impl TryFrom<&Row<'_>> for SimpleResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
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
}

#[cfg(feature = "uuid")]
#[derive(Debug)]
struct ComplexResult {
    id: String,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

impl TryFrom<&Row<'_>> for ComplexResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
            age: row.get(3)?,
        })
    }
}

#[derive(Debug)]
struct JoinResult {
    name: String,
    title: String,
}

impl TryFrom<&Row<'_>> for JoinResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: row.get(0)?,
            title: row.get(1)?,
        })
    }
}

#[test]
fn simple_select_with_conditions() {
    let db = setup_db();
    let (drizzle, (simple, complex, ..)) = drizzle!(db, [Simple, Complex, Post]);

    // Insert test data
    let test_data = vec![
        InsertSimple::default().with_name("alpha"),
        InsertSimple::default().with_name("beta"),
        InsertSimple::default().with_name("gamma"),
        InsertSimple::default().with_name("delta"),
    ];

    drizzle.insert(simple).values(test_data).execute().unwrap();

    // Test WHERE condition
    let where_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .r#where(eq(Simple::name, "beta"))
        .all()
        .unwrap();

    assert_eq!(where_results.len(), 1);
    assert_eq!(where_results[0].name, "beta");

    // Test ORDER BY with LIMIT
    let ordered_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .order_by(vec![(Simple::name, OrderBy::Asc)])
        .limit(2)
        .all()
        .unwrap();

    assert_eq!(ordered_results.len(), 2);
    assert_eq!(ordered_results[0].name, "alpha");
    assert_eq!(ordered_results[1].name, "beta");

    // Test LIMIT with OFFSET
    let offset_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .order_by(vec![(Simple::name, OrderBy::Asc)])
        .limit(2)
        .offset(2)
        .all()
        .unwrap();

    assert_eq!(offset_results.len(), 2);
    assert_eq!(offset_results[0].name, "delta");
    assert_eq!(offset_results[1].name, "gamma");
}

#[test]
fn complex_select_with_conditions() {
    let db = setup_db();
    let (drizzle, (simple, complex, ..)) = drizzle!(db, [Simple, Complex, Post]);

    // Insert test data with different ages
    #[cfg(not(feature = "uuid"))]
    let test_data = vec![
        InsertComplex::default()
            .with_name("young")
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::default()
            .with_name("middle")
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::default()
            .with_name("old")
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    #[cfg(feature = "uuid")]
    let test_data = vec![
        InsertComplex::default()
            .with_id(uuid::Uuid::new_v4())
            .with_name("young")
            .with_email("young@test.com".to_string())
            .with_age(20),
        InsertComplex::default()
            .with_id(uuid::Uuid::new_v4())
            .with_name("middle")
            .with_email("middle@test.com".to_string())
            .with_age(35),
        InsertComplex::default()
            .with_id(uuid::Uuid::new_v4())
            .with_name("old")
            .with_email("old@test.com".to_string())
            .with_age(50),
    ];

    drizzle.insert(complex).values(test_data).execute().unwrap();

    // Test complex WHERE with GT condition
    let gt_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from(complex)
        .r#where(gt(Complex::age, 25))
        .all()
        .unwrap();

    assert_eq!(gt_results.len(), 2);
    let names: Vec<String> = gt_results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"middle".to_string()));
    assert!(names.contains(&"old".to_string()));

    // Test complex WHERE with range conditions (AND logic)
    let range_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from(complex)
        .r#where(and(vec![
            Some(gte(Complex::age, 25)),
            Some(lt(Complex::age, 45)),
        ]))
        .all()
        .unwrap();

    assert_eq!(range_results.len(), 1);
    assert_eq!(range_results[0].name, "middle");
    assert_eq!(range_results[0].age, Some(35));
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[test]
fn feature_gated_select() {
    let db = setup_db();
    let (drizzle, (simple, complex, post)) = drizzle!(db, [Simple, Complex, Post]);

    // Insert Complex record with feature-gated fields
    let test_id = uuid::Uuid::new_v4();
    let data = InsertComplex::default()
        .with_id(test_id)
        .with_name("feature_user")
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

    drizzle.insert(complex).values([data]).execute().unwrap();

    // Query using UUID field
    let uuid_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from(complex)
        .r#where(eq(Complex::id, test_id.to_string()))
        .all()
        .unwrap();

    assert_eq!(uuid_results.len(), 1);
    assert_eq!(uuid_results[0].name, "feature_user");

    // Query using name to verify metadata exists (can't easily verify content without custom result type)
    let metadata_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age
        ])
        .from(complex)
        .r#where(eq(Complex::name, "feature_user"))
        .all()
        .unwrap();

    assert_eq!(metadata_results.len(), 1);
    assert_eq!(metadata_results[0].name, "feature_user");
}
