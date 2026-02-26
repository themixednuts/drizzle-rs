//! SQLite GROUP BY and HAVING tests
//!
//! Tests for GROUP BY clause with aggregate functions and HAVING filter,
//! executed against a real SQLite database.

#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{ComplexSchema, InsertComplex};
use crate::common::schema::sqlite::{InsertSimple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

// =============================================================================
// Result Types
// =============================================================================

#[derive(Debug, SQLiteFromRow)]
struct GroupCountResult {
    name: String,
    count: i64,
}

#[derive(Debug, SQLiteFromRow)]
struct BoolGroupResult {
    active: bool,
    total: Option<i32>,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct GroupAvgResult {
    active: bool,
    avg_score: Option<f64>,
}

// =============================================================================
// GROUP BY Tests
// =============================================================================

sqlite_test!(test_group_by_simple_count, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("alice").with_id(2),
        InsertSimple::new("bob").with_id(3),
        InsertSimple::new("bob").with_id(4),
        InsertSimple::new("bob").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let results: Vec<GroupCountResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(count(simple.id), "count"),
        ))
        .from(simple)
        .group_by([simple.name])
        .order_by(asc(simple.name))
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[0].count, 2);
    assert_eq!(results[1].name, "bob");
    assert_eq!(results[1].count, 3);
});

#[cfg(feature = "uuid")]
sqlite_test!(test_group_by_with_sum, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User).with_age(25),
        InsertComplex::new("bob", true, Role::Admin).with_age(30),
        InsertComplex::new("charlie", false, Role::User).with_age(35),
        InsertComplex::new("diana", false, Role::User).with_age(40),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    let results: Vec<BoolGroupResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(sum(complex.age), "total"),
        ))
        .from(complex)
        .group_by([complex.active])
        .order_by(asc(complex.active))
            => all
    );

    assert_eq!(results.len(), 2);
    // SQLite: false = 0, true = 1; order_by(asc) puts false first
    assert!(!results[0].active);
    assert_eq!(results[0].total, Some(75)); // 35 + 40
    assert!(results[1].active);
    assert_eq!(results[1].total, Some(55)); // 25 + 30
});

#[cfg(feature = "uuid")]
sqlite_test!(test_group_by_with_avg, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User).with_score(80.0),
        InsertComplex::new("bob", true, Role::Admin).with_score(90.0),
        InsertComplex::new("charlie", false, Role::User).with_score(70.0),
        InsertComplex::new("diana", false, Role::User).with_score(60.0),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    let results: Vec<GroupAvgResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(avg(complex.score), "avg_score"),
        ))
        .from(complex)
        .group_by([complex.active])
        .order_by(asc(complex.active))
            => all
    );

    assert_eq!(results.len(), 2);
    assert!(!results[0].active);
    assert!((results[0].avg_score.unwrap() - 65.0).abs() < 0.01);
    assert!(results[1].active);
    assert!((results[1].avg_score.unwrap() - 85.0).abs() < 0.01);
});

// =============================================================================
// HAVING Tests
// =============================================================================

sqlite_test!(test_having_filters_groups, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("alice").with_id(2),
        InsertSimple::new("alice").with_id(3),
        InsertSimple::new("bob").with_id(4),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // GROUP BY name HAVING COUNT(*) > 2
    // alice has 3, bob has 1 — only alice should appear
    let results: Vec<GroupCountResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(count(simple.id), "count"),
        ))
        .from(simple)
        .group_by([simple.name])
        .having(gt(count(simple.id), 2_i64))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[0].count, 3);
});

sqlite_test!(test_having_with_sum, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(10),
        InsertSimple::new("alice").with_id(20),
        InsertSimple::new("bob").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // GROUP BY name HAVING SUM(id) > 10
    // alice sum=30, bob sum=5 — only alice
    #[derive(Debug, SQLiteFromRow)]
    struct SumGroupResult {
        name: String,
        total: Option<i32>,
    }

    let results: Vec<SumGroupResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(sum(simple.id), "total"),
        ))
        .from(simple)
        .group_by([simple.name])
        .having(gt(sum(simple.id), 10))
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[0].total, Some(30));
});

sqlite_test!(test_having_no_matching_groups, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // HAVING COUNT(*) > 10 — no group qualifies
    let results: Vec<GroupCountResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(count(simple.id), "count"),
        ))
        .from(simple)
        .group_by([simple.name])
        .having(gt(count(simple.id), 10_i64))
            => all
    );

    assert_eq!(results.len(), 0);
});

// =============================================================================
// GROUP BY with ORDER BY
// =============================================================================

sqlite_test!(test_group_by_order_by_aggregate, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(10),
        InsertSimple::new("alice").with_id(20),
        InsertSimple::new("bob").with_id(100),
        InsertSimple::new("charlie").with_id(1),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // GROUP BY name, ORDER BY SUM(id) DESC
    #[derive(Debug, SQLiteFromRow)]
    struct SumGroupResult {
        name: String,
        total: Option<i32>,
    }

    let results: Vec<SumGroupResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(sum(simple.id), "total"),
        ))
        .from(simple)
        .group_by([simple.name])
        .order_by(desc(sum(simple.id)))
            => all
    );

    assert_eq!(results.len(), 3);
    // bob=100, alice=30, charlie=1
    assert_eq!(results[0].name, "bob");
    assert_eq!(results[0].total, Some(100));
    assert_eq!(results[1].name, "alice");
    assert_eq!(results[1].total, Some(30));
    assert_eq!(results[2].name, "charlie");
    assert_eq!(results[2].total, Some(1));
});

// =============================================================================
// GROUP BY with LIMIT
// =============================================================================

sqlite_test!(test_group_by_with_limit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("bob").with_id(3),
        InsertSimple::new("charlie").with_id(4),
        InsertSimple::new("charlie").with_id(5),
        InsertSimple::new("charlie").with_id(6),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // GROUP BY name ORDER BY count DESC LIMIT 2 — top 2 groups
    let results: Vec<GroupCountResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(count(simple.id), "count"),
        ))
        .from(simple)
        .group_by([simple.name])
        .order_by(desc(count(simple.id)))
        .limit(2)
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "charlie");
    assert_eq!(results[0].count, 3);
    assert_eq!(results[1].name, "bob");
    assert_eq!(results[1].count, 2);
});

sqlite_test!(test_group_by_limit_without_order, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("alice").with_id(2),
        InsertSimple::new("bob").with_id(3),
        InsertSimple::new("charlie").with_id(4),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // GROUP BY name LIMIT 2 — direct LIMIT on GROUP BY without ORDER BY
    let results: Vec<GroupCountResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(count(simple.id), "count"),
        ))
        .from(simple)
        .group_by([simple.name])
        .limit(2)
            => all
    );

    assert_eq!(results.len(), 2);
});

// =============================================================================
// SELECT with OFFSET directly from FROM
// =============================================================================

sqlite_test!(test_select_from_offset, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
        InsertSimple::new("diana").with_id(4),
        InsertSimple::new("eve").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // SELECT ... FROM simple LIMIT 2 OFFSET 2
    #[derive(Debug, SQLiteFromRow)]
    struct NameResult {
        name: String,
    }

    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
        .from(simple)
        .order_by(asc(simple.id))
        .limit(2)
        .offset(2)
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "charlie");
    assert_eq!(results[1].name, "diana");
});

// =============================================================================
// GROUP BY on empty table
// =============================================================================

sqlite_test!(test_group_by_empty_table, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // No data inserted
    let results: Vec<GroupCountResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(count(simple.id), "count"),
        ))
        .from(simple)
        .group_by([simple.name])
            => all
    );

    assert_eq!(results.len(), 0);
});

// =============================================================================
// Set Operation Tests (UNION / UNION ALL / INTERSECT / EXCEPT)
// =============================================================================

#[derive(Debug, SQLiteFromRow)]
struct NameResult {
    name: String,
}

sqlite_test!(test_union_via_drizzle_builder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // UNION two DrizzleBuilder selects: names where id <= 2 UNION names where id >= 2
    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(lte(simple.id, 2))
          .union(
              db.select(simple.name)
                .from(simple)
                .r#where(gte(simple.id, 2))
          )
          .order_by(asc(simple.name))
            => all
    );

    // UNION deduplicates: alice, bob (from left), bob, charlie (from right) → alice, bob, charlie
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[1].name, "bob");
    assert_eq!(results[2].name, "charlie");
});

sqlite_test!(test_union_all_preserves_duplicates, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // UNION ALL keeps duplicates: bob appears in both sets
    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(lte(simple.id, 2))
          .union_all(
              db.select(simple.name)
                .from(simple)
                .r#where(gte(simple.id, 2))
          )
          .order_by(asc(simple.name))
            => all
    );

    // UNION ALL: alice, bob + bob, charlie → alice, bob, bob, charlie
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[1].name, "bob");
    assert_eq!(results[2].name, "bob");
    assert_eq!(results[3].name, "charlie");
});

sqlite_test!(test_union_mixed_drizzle_and_raw, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // DrizzleBuilder.union(raw SelectBuilder) — interop test
    let qb = drizzle_sqlite::builder::QueryBuilder::new::<SimpleSchema>();

    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(eq(simple.id, 1))
          .union(
              qb.select(simple.name)
                .from(simple)
                .r#where(eq(simple.id, 3))
          )
          .order_by(asc(simple.name))
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[1].name, "charlie");
});

sqlite_test!(test_chained_set_operations, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Chain multiple set ops: q1 UNION q2 UNION ALL q3
    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(eq(simple.id, 1))
          .union(
              db.select(simple.name)
                .from(simple)
                .r#where(eq(simple.id, 2))
          )
          .union_all(
              db.select(simple.name)
                .from(simple)
                .r#where(eq(simple.id, 1))
          )
          .order_by(asc(simple.name))
            => all
    );

    // (alice UNION bob) UNION ALL alice → alice, alice, bob
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[1].name, "alice");
    assert_eq!(results[2].name, "bob");
});

sqlite_test!(test_set_op_with_order_limit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
        InsertSimple::new("diana").with_id(4),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // UNION with ORDER BY and LIMIT on the combined result
    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(lte(simple.id, 2))
          .union(
              db.select(simple.name)
                .from(simple)
                .r#where(gte(simple.id, 3))
          )
          .order_by(desc(simple.name))
          .limit(2)
            => all
    );

    // All 4 names combined, ordered desc, limited to 2: diana, charlie
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "diana");
    assert_eq!(results[1].name, "charlie");
});

sqlite_test!(test_intersect_via_drizzle_builder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // INTERSECT: rows in both sets (id <= 2) AND (id >= 2) → only bob (id=2)
    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(lte(simple.id, 2))
          .intersect(
              db.select(simple.name)
                .from(simple)
                .r#where(gte(simple.id, 2))
          )
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "bob");
});

sqlite_test!(test_except_via_drizzle_builder, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice").with_id(1),
        InsertSimple::new("bob").with_id(2),
        InsertSimple::new("charlie").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // EXCEPT: rows in left set (id <= 2) but NOT in right set (id >= 2) → alice only
    let results: Vec<NameResult> = drizzle_exec!(
        db.select(simple.name)
          .from(simple)
          .r#where(lte(simple.id, 2))
          .except(
              db.select(simple.name)
                .from(simple)
                .r#where(gte(simple.id, 2))
          )
            => all
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "alice");
});
