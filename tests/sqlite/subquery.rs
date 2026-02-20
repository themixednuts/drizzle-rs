#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use crate::common::schema::sqlite::{InsertSimple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct SubqueryResult {
    id: i32,
    name: String,
}

sqlite_test!(test_one_level_subquery, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test one level subquery: find records where id is greater than the minimum id
    let min_id_subquery = db.select(min(simple.id)).from(simple);
    let results: Vec<SubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(gt(simple.id, min_id_subquery))
            => all
    );

    drizzle_assert_eq!(2, results.len()); // Should exclude the minimum (id=1)
    drizzle_assert!(results.iter().any(|r| r.name == "bob"));
    drizzle_assert!(results.iter().any(|r| r.name == "charlie"));
});

// Note: Turso doesn't support nested subqueries in AVG() - turso variant will fail
sqlite_test!(test_two_level_subquery, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("user1").with_id(1),
        InsertSimple::new("user2").with_id(2),
        InsertSimple::new("user3").with_id(3),
        InsertSimple::new("user4").with_id(4),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test two level subquery: find records where id is greater than the average of ids greater than 1
    let inner_subquery = db.select(simple.id).from(simple).r#where(gt(simple.id, 1));
    let avg_subquery = db.select(avg(inner_subquery)).from(simple);

    let results: Vec<SubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(gt(simple.id, avg_subquery)) => all
    );

    // Should find records with id > average of (2,3,4) = 3, so only id=4
    drizzle_assert!(!results.is_empty());
    drizzle_assert!(results.iter().any(|r| r.name == "user4"));
});

sqlite_test!(test_three_level_subquery, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alpha").with_id(10),
        InsertSimple::new("beta").with_id(20),
        InsertSimple::new("gamma").with_id(30),
        InsertSimple::new("delta").with_id(40),
        InsertSimple::new("epsilon").with_id(50),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test three level subquery
    // Level 1: Get ids > 20
    let level1 = db.select(simple.id).from(simple).r#where(gt(simple.id, 20));
    // Level 2: Get average of those ids
    let level2 = db.select(avg(level1)).from(simple);
    // Level 3: Find records where id is greater than that average
    let results: Vec<SubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(gt(simple.id, level2))
            => all
    );

    // Average of (30,40,50) = 40, so should return records with id > 40 (just epsilon with id=50)
    drizzle_assert!(!results.is_empty());
    drizzle_assert!(results.iter().any(|r| r.name == "epsilon"));
});

// =============================================================================
// Typed subqueries via Expr on SELECT builders
// =============================================================================

sqlite_test!(test_typed_scalar_subquery, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // SELECT builders now implement Expr directly with concrete SQLType
    let min_id = db.select(min(simple.id)).from(simple);
    let results: Vec<SubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(gt(simple.id, min_id))
            => all
    );

    drizzle_assert_eq!(2, results.len());
    drizzle_assert!(results.iter().any(|r| r.name == "bob"));
    drizzle_assert!(results.iter().any(|r| r.name == "charlie"));
});

sqlite_test!(test_typed_scalar_subquery_max, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(10),
        InsertSimple::new("bob").with_id(20),
        InsertSimple::new("charlie").with_id(30),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Typed subquery with max — should find records where id < max
    let max_id = db.select(max(simple.id)).from(simple);
    let results: Vec<SubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(lt(simple.id, max_id))
            => all
    );

    drizzle_assert_eq!(2, results.len());
    drizzle_assert!(results.iter().any(|r| r.name == "alice"));
    drizzle_assert!(results.iter().any(|r| r.name == "bob"));
});

sqlite_test!(test_typed_in_subquery_single_column, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let only_bob_id = db
        .select(simple.id)
        .from(simple)
        .r#where(eq(simple.name, "bob"));

    let results: Vec<SubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(in_subquery(simple.id, only_bob_id))
            => all
    );

    drizzle_assert_eq!(1, results.len());
    drizzle_assert_eq!("bob", results[0].name);
});

sqlite_test!(
    test_typed_in_subquery_multi_column_row_value,
    SimpleSchema,
    {
        let SimpleSchema { simple } = schema;

        let test_data = vec![
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("charlie").with_id(3),
        ];

        drizzle_exec!(db.insert(simple).values(test_data) => execute);

        let bob_row = db
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "bob"));

        let results: Vec<SubqueryResult> = drizzle_exec!(
            db.select((simple.id, simple.name))
                .from(simple)
                .r#where(in_subquery(row((simple.id, simple.name)), bob_row))
                => all
        );

        drizzle_assert_eq!(1, results.len());
        drizzle_assert_eq!("bob", results[0].name);
    }
);
