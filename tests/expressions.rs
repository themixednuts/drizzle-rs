#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use std::marker::PhantomData;

use common::{Complex, InsertComplex, InsertSimple, Role, Simple};
use drizzle_core::expressions::*;
use drizzle_rs::prelude::*;

mod common;

#[derive(Debug, FromRow)]
struct CountResult {
    count: i32,
}

#[derive(Debug, FromRow)]
struct SumResult {
    sum: i32,
}

#[derive(Debug, FromRow)]
struct MinResult {
    min: i32,
}

#[derive(Debug, FromRow)]
struct MaxResult {
    max: i32,
}

#[derive(Debug, FromRow)]
struct AvgResult {
    avg: f64,
}

#[derive(Debug, FromRow)]
struct SumRealResult {
    sum: f64,
}

#[derive(Debug, FromRow)]
struct MinRealResult {
    min: f64,
}

#[derive(Debug, FromRow)]
struct MaxRealResult {
    max: f64,
}

#[derive(Debug, FromRow)]
struct DistinctResult {
    name: String,
}

#[derive(Debug, FromRow)]
struct CoalesceStringResult {
    coalesce: String,
}

#[derive(Debug, FromRow)]
struct CoalesceIntResult {
    coalesce: i32,
}

#[derive(Debug, FromRow)]
struct AliasResult {
    item_name: String,
}

#[derive(Debug, FromRow)]
struct CountAliasResult {
    total_count: i32,
}

#[derive(Debug, FromRow)]
struct SumAliasResult {
    id_sum: i32,
}

#[derive(Debug, FromRow)]
struct ComplexAggregateResult {
    count: i32,
    avg: f64,
    max_age: i32,
}

#[derive(Debug, FromRow)]
struct CoalesceAvgResult {
    coalesce: f64,
}

#[tokio::test]
async fn test_aggregate_functions() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Item A").with_id(10),
        InsertSimple::new("Item B").with_id(20),
        InsertSimple::new("Item C").with_id(30),
        InsertSimple::new("Item D").with_id(40),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test count function
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].count, 4);

    // Test sum function
    let result: Vec<SumResult> =
        drizzle_exec!(db.select(alias(sum(simple.id), "sum")).from(simple).all());
    assert_eq!(result[0].sum, 100);

    // Test min function
    let result: Vec<MinResult> =
        drizzle_exec!(db.select(alias(min(simple.id), "min")).from(simple).all());
    assert_eq!(result[0].min, 10);

    // Test max function
    let result: Vec<MaxResult> =
        drizzle_exec!(db.select(alias(max(simple.id), "max")).from(simple).all());
    assert_eq!(result[0].max, 40);

    // Test avg function
    let result: Vec<AvgResult> =
        drizzle_exec!(db.select(alias(avg(simple.id), "avg")).from(simple).all());
    assert_eq!(result[0].avg, 25.0);
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_aggregate_functions_with_real_numbers() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    let test_data = vec![
        InsertComplex::new("User A", true, Role::User).with_score(85.5),
        InsertComplex::new("User B", false, Role::Admin).with_score(92.0),
        InsertComplex::new("User C", true, Role::User).with_score(78.3),
        InsertComplex::new("User D", false, Role::User).with_score(88.7),
    ];

    drizzle_exec!(db.insert(complex).values(test_data).execute());

    // Test count with non-null values
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(complex.score), "count"))
            .from(complex)
            .all()
    );
    assert_eq!(result[0].count, 4);

    // Test sum with real numbers
    let result: Vec<SumRealResult> = drizzle_exec!(
        db.select(alias(sum(complex.score), "sum"))
            .from(complex)
            .all()
    );
    assert!((result[0].sum - 344.5).abs() < 0.1);

    // Test avg with real numbers
    let result: Vec<AvgResult> = drizzle_exec!(
        db.select(alias(avg(complex.score), "avg"))
            .from(complex)
            .all()
    );
    assert!((result[0].avg - 86.125).abs() < 0.1);

    // Test min with real numbers
    let result: Vec<MinRealResult> = drizzle_exec!(
        db.select(alias(min(complex.score), "min"))
            .from(complex)
            .all()
    );
    assert!((result[0].min - 78.3).abs() < 0.1);

    // Test max with real numbers
    let result: Vec<MaxRealResult> = drizzle_exec!(
        db.select(alias(max(complex.score), "max"))
            .from(complex)
            .all()
    );
    assert!((result[0].max - 92.0).abs() < 0.1);
}

#[tokio::test]
async fn test_distinct_expression() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![
        InsertSimple::new("Apple").with_id(1),
        InsertSimple::new("Apple").with_id(2),
        InsertSimple::new("Banana").with_id(3),
        InsertSimple::new("Apple").with_id(4),
        InsertSimple::new("Cherry").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test distinct function
    let result: Vec<DistinctResult> = drizzle_exec!(
        db.select(alias(distinct(simple.name), "name"))
            .from(simple)
            .all()
    );
    assert_eq!(result.len(), 3);
    let names: Vec<String> = result.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"Apple".to_string()));
    assert!(names.contains(&"Banana".to_string()));
    assert!(names.contains(&"Cherry".to_string()));

    // Test count with distinct
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(distinct(simple.name)), "count"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].count, 3);
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_coalesce_expression() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    // Insert data with separate operations since each has different column patterns
    // Users A and C: have email set
    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("User A", true, Role::User)
                    .with_email("user@example.com".to_string()),
                InsertComplex::new("User C", true, Role::User)
                    .with_email("user3@example.com".to_string()),
            ])
            .execute()
    );

    // User B: has no optional fields set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User B", false, Role::Admin)])
            .execute()
    );

    // Test coalesce with email field (some null, some not)
    let result: Vec<CoalesceStringResult> = drizzle_exec!(
        db.select(alias(
            coalesce(complex.email, "no-email@example.com"),
            "coalesce"
        ))
        .from(complex)
        .all()
    );
    assert_eq!(result.len(), 3);
    let emails: Vec<String> = result.iter().map(|r| r.coalesce.clone()).collect();
    assert!(emails.contains(&"user@example.com".to_string()));
    assert!(emails.contains(&"user3@example.com".to_string()));
    assert!(emails.contains(&"no-email@example.com".to_string()));

    // Test coalesce with age field
    let result: Vec<CoalesceIntResult> = drizzle_exec!(
        db.select(alias(coalesce(complex.age, 0), "coalesce"))
            .from(complex)
            .all()
    );
    assert_eq!(result.len(), 3);
    // All should be 0 since we didn't set any ages
    assert!(result.iter().all(|r| r.coalesce == 0));
}

#[tokio::test]
async fn test_alias_expression() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = vec![InsertSimple::new("Test Item").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test alias with simple column
    let result: Vec<AliasResult> = drizzle_exec!(
        db.select(alias(simple.name, "item_name"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].item_name, "Test Item");

    // Test alias with aggregate function
    let result: Vec<CountAliasResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "total_count"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].total_count, 1);

    // Test alias with expression
    let result: Vec<SumAliasResult> = drizzle_exec!(
        db.select(alias(sum(simple.id), "id_sum"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].id_sum, 1);
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_complex_expressions() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    // Insert data with separate operations since each has different column patterns
    // Users A and B: have both age and score set
    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("User A", true, Role::User)
                    .with_age(25)
                    .with_score(85.5),
                InsertComplex::new("User B", false, Role::Admin)
                    .with_age(30)
                    .with_score(92.0),
            ])
            .execute()
    );

    // User C: has only score set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User C", true, Role::User).with_score(78.3)])
            .execute()
    );

    // Test multiple expressions in one query
    let result: Vec<ComplexAggregateResult> = drizzle_exec!(
        db.select((
            alias(count(complex.id), "count"),
            alias(avg(complex.score), "avg"),
            alias(coalesce(max(complex.age), 0), "max_age")
        ))
        .from(complex)
        .all()
    );
    assert_eq!(result[0].count, 3); // count
    assert!((result[0].avg - 85.266).abs() < 0.1); // avg score
    assert_eq!(result[0].max_age, 30); // max age (coalesced)

    // Test nested expressions
    let result: Vec<CoalesceAvgResult> = drizzle_exec!(
        db.select(alias(coalesce(avg(complex.score), 0.0), "coalesce"))
            .from(complex)
            .r#where(is_not_null(complex.score))
            .all()
    );
    assert!((result[0].coalesce - 85.266).abs() < 0.1);
}

pub struct Test<T = (TestNameNotSet, TestEmailNotSet)> {
    name: &'static str,
    email: &'static str,
    _phantom: PhantomData<T>,
}

struct TestNameSet {}
struct TestNameNotSet {}
struct TestEmailSet {}
struct TestEmailNotSet {}

impl<'a, TestName, TestEmail> Test<(TestName, TestEmail)> {
    pub fn with_name(self, name: &'static str) -> Test<(TestNameSet, TestEmail)> {
        Test {
            name,
            email: self.email,
            _phantom: PhantomData,
        }
    }
    pub fn with_email(self, email: &'static str) -> Test<(TestName, TestEmailSet)> {
        Test {
            name: self.name,
            email,
            _phantom: PhantomData,
        }
    }
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_expressions_with_conditions() {
    let conn = setup_test_db!();
    let (db, complex) = drizzle!(conn, [Complex]);

    let test_data = [
        InsertComplex::new("Active User", true, Role::User).with_score(85.5),
        InsertComplex::new("Active Admin", true, Role::Admin).with_score(92.0),
        InsertComplex::new("Inactive User", false, Role::User).with_score(78.3),
        InsertComplex::new("Inactive Admin", false, Role::Admin).with_score(88.7),
    ];

    drizzle_exec!(db.insert(complex).values(test_data).execute());

    // Test count with condition
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(complex.id), "count"))
            .from(complex)
            .r#where(eq(complex.active, true))
            .all()
    );
    assert_eq!(result[0].count, 2);

    // Test avg with condition
    let result: Vec<AvgResult> = drizzle_exec!(
        db.select(alias(avg(complex.score), "avg"))
            .from(complex)
            .r#where(eq(complex.role, Role::Admin))
            .all()
    );
    assert!((result[0].avg - 90.35).abs() < 0.1);

    // Test max with condition
    let result: Vec<MaxRealResult> = drizzle_exec!(
        db.select(alias(max(complex.score), "max"))
            .from(complex)
            .r#where(eq(complex.active, false))
            .all()
    );
    assert!((result[0].max - 88.7).abs() < 0.1);
}

#[tokio::test]
async fn test_aggregate_with_empty_result() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    // No data inserted, test aggregate functions on empty table

    // Count should return 0
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].count, 0);

    // Other aggregates on empty table should handle NULL appropriately
    // Note: Different databases handle this differently, but typically return NULL
    // which would be handled by the driver
}

#[tokio::test]
async fn test_expression_edge_cases() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = [
        InsertSimple::new("").with_id(0), // Empty string and zero id
        InsertSimple::new("Test").with_id(1),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test count with all rows
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            .all()
    );
    assert_eq!(result[0].count, 2);

    // Test distinct with empty string
    let result: Vec<DistinctResult> = drizzle_exec!(
        db.select(alias(distinct(simple.name), "name"))
            .from(simple)
            .all()
    );
    assert_eq!(result.len(), 2);
    let names: Vec<String> = result.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"".to_string()));
    assert!(names.contains(&"Test".to_string()));

    // Test sum with zero
    let result: Vec<SumResult> =
        drizzle_exec!(db.select(alias(sum(simple.id), "sum")).from(simple).all());
    assert_eq!(result[0].sum, 1);

    // Test coalesce with empty string
    let result: Vec<CoalesceStringResult> = drizzle_exec!(
        db.select(alias(coalesce(simple.name, "default"), "coalesce"))
            .from(simple)
            .r#where(eq(simple.name, ""))
            .all()
    );
    assert_eq!(result[0].coalesce, ""); // Empty string is not NULL, so coalesce returns it
}

#[tokio::test]
async fn test_multiple_aliases() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let test_data = [
        InsertSimple::new("Item A").with_id(1),
        InsertSimple::new("Item B").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    #[derive(FromRow)]
    struct ResultRow {
        identifier: i32,
        item_name: String,
        total: i32,
    }
    // Test multiple aliases in same query
    let result: Vec<ResultRow> = drizzle_exec!(
        db.select((
            alias(simple.id, "identifier"),
            alias(simple.name, "item_name"),
            alias(count(simple.id), "total")
        ))
        .from(simple)
        .all()
    );

    assert_eq!(result[0].identifier, 1);
    assert_eq!(result[0].item_name, "Item A");
    assert_eq!(result[0].total, 2);
}
