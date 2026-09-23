#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
//! SQLite-specific subquery behavior. Portable scalar/IN/EXISTS subqueries
//! live in `crate::common::subquery`; this file keeps SQLite reading the first
//! row of a multi-row scalar subquery and the integer-tuple row-value LHS.
use crate::common::schema::sqlite::{InsertSimple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct SubqueryResult {
    id: i32,
    name: String,
}

// Note: Turso doesn't support nested subqueries in AVG() - turso variant will fail
#[drizzle::test]
fn test_two_level_subquery(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("user1").with_id(1),
        InsertSimple::new("user2").with_id(2),
        InsertSimple::new("user3").with_id(3),
        InsertSimple::new("user4").with_id(4),
    ];

    db.insert(simple).values(test_data).execute();

    // Test two level subquery: find records where id is greater than the average of ids greater than 1
    let inner_subquery = db.select(simple.id).from(simple).r#where(gt(simple.id, 1));
    let avg_subquery = db.select(avg(inner_subquery)).from(simple);

    let results: Vec<SubqueryResult> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(gt(simple.id, avg_subquery))
        .all();

    // Should find records with id > average of (2,3,4) = 3, so only id=4
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.name == "user4"));
}

#[drizzle::test]
fn test_three_level_subquery(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alpha").with_id(10),
        InsertSimple::new("beta").with_id(20),
        InsertSimple::new("gamma").with_id(30),
        InsertSimple::new("delta").with_id(40),
        InsertSimple::new("epsilon").with_id(50),
    ];

    db.insert(simple).values(test_data).execute();

    // Test three level subquery
    // Level 1: Get ids > 20
    let level1 = db.select(simple.id).from(simple).r#where(gt(simple.id, 20));
    // Level 2: Get average of those ids
    let level2 = db.select(avg(level1)).from(simple);
    // Level 3: Find records where id is greater than that average
    let results: Vec<SubqueryResult> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(gt(simple.id, level2))
        .all();

    // Average of (30,40,50) = 40, so should return records with id > 40 (just epsilon with id=50)
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.name == "epsilon"));
}

// SQLite's Integer is `BooleanLike`, so a tuple of integer columns is also a
// valid condition list. The row-value LHS must still win here — see the
// `Conjunction` SQL type in drizzle-types.
#[drizzle::test]
fn test_in_subquery_all_integer_tuple_lhs(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("alice").with_id(1)])
        .execute();

    let self_row = db.select((simple.id, simple.id)).from(simple);
    let results: Vec<SubqueryResult> = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(in_subquery((simple.id, simple.id), self_row))
        .all();

    assert_eq!(1, results.len());
}
