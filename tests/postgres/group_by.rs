//! PostgreSQL GROUP BY and HAVING tests
//!
//! Tests for GROUP BY clause with aggregate functions and HAVING filter,
//! executed against a real PostgreSQL database.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

#[cfg(feature = "uuid")]
use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

// =============================================================================
// Result Types
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct GroupCountResult {
    name: String,
    count: i64,
}

#[derive(Debug, PostgresFromRow)]
struct GroupSumResult {
    active: bool,
    total_age: Option<i64>,
}

#[derive(Debug, PostgresFromRow)]
struct GroupCountActiveResult {
    active: bool,
    total_age: i64,
}

#[derive(Debug, PostgresFromRow)]
struct GroupAvgResult {
    active: bool,
    avg_score: Option<f64>,
}

#[derive(Debug, PostgresFromRow)]
struct RoleCountResult {
    role: String,
    count: i64,
}

// =============================================================================
// GROUP BY Tests
// =============================================================================

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_with_count, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User),
        InsertComplex::new("bob", true, Role::Admin),
        InsertComplex::new("charlie", false, Role::User),
        InsertComplex::new("diana", true, Role::User),
        InsertComplex::new("eve", false, Role::Admin),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active, COUNT per group
    let results: Vec<GroupCountActiveResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(count(complex.id), "total_age"),
        ))
        .from(complex)
        .group_by([complex.active])
        .order_by(asc(complex.active))
            => all
    );

    assert_eq!(results.len(), 2);
    // active=false group
    assert!(!results[0].active);
    assert_eq!(results[0].total_age, 2);
    // active=true group
    assert!(results[1].active);
    assert_eq!(results[1].total_age, 3);
});

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_with_sum, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User).with_age(25),
        InsertComplex::new("bob", true, Role::Admin).with_age(30),
        InsertComplex::new("charlie", false, Role::User).with_age(35),
        InsertComplex::new("diana", false, Role::User).with_age(40),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active, SUM(age)
    let results: Vec<GroupSumResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(sum(complex.age), "total_age"),
        ))
        .from(complex)
        .group_by([complex.active])
        .order_by(asc(complex.active))
            => all
    );

    assert_eq!(results.len(), 2);
    assert!(!results[0].active);
    assert_eq!(results[0].total_age, Some(75)); // 35 + 40
    assert!(results[1].active);
    assert_eq!(results[1].total_age, Some(55)); // 25 + 30
});

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_with_avg, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

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

#[cfg(feature = "uuid")]
postgres_test!(test_having_filters_groups, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User),
        InsertComplex::new("bob", true, Role::Admin),
        InsertComplex::new("charlie", true, Role::User),
        InsertComplex::new("diana", false, Role::User),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active HAVING COUNT(*) > 2
    // active=true has 3 rows, active=false has 1 row
    // Only active=true should appear
    #[derive(Debug, PostgresFromRow)]
    struct HavingResult {
        active: bool,
        cnt: i64,
    }

    let results: Vec<HavingResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(count(complex.id), "cnt"),
        ))
        .from(complex)
        .group_by([complex.active])
        .having(gt(count(complex.id), 2_i64))
            => all
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].active);
    assert_eq!(results[0].cnt, 3);
});

#[cfg(feature = "uuid")]
postgres_test!(test_having_with_sum, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User).with_age(25),
        InsertComplex::new("bob", true, Role::Admin).with_age(30),
        InsertComplex::new("charlie", false, Role::User).with_age(10),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active HAVING SUM(age) > 20
    // active=true has sum=55, active=false has sum=10
    let results: Vec<GroupSumResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(sum(complex.age), "total_age"),
        ))
        .from(complex)
        .group_by([complex.active])
        .having(gt(sum(complex.age), 20_i64))
            => all
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].active);
    assert_eq!(results[0].total_age, Some(55));
});

#[cfg(feature = "uuid")]
postgres_test!(test_having_no_matching_groups, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User),
        InsertComplex::new("bob", false, Role::User),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active HAVING COUNT(*) > 10 — no group qualifies
    #[derive(Debug, PostgresFromRow)]
    struct HavingResult {
        active: bool,
        cnt: i64,
    }

    let results: Vec<HavingResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(count(complex.id), "cnt"),
        ))
        .from(complex)
        .group_by([complex.active])
        .having(gt(count(complex.id), 10_i64))
            => all
    );

    assert_eq!(results.len(), 0);
});

// =============================================================================
// GROUP BY with ORDER BY
// =============================================================================

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_order_by, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User).with_age(25),
        InsertComplex::new("bob", false, Role::Admin).with_age(30),
        InsertComplex::new("charlie", true, Role::User).with_age(35),
        InsertComplex::new("diana", false, Role::User).with_age(40),
        InsertComplex::new("eve", true, Role::Admin).with_age(45),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active, ORDER BY SUM(age) DESC
    let results: Vec<GroupSumResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(sum(complex.age), "total_age"),
        ))
        .from(complex)
        .group_by([complex.active])
        .order_by(desc(sum(complex.age)))
            => all
    );

    assert_eq!(results.len(), 2);
    // active=true: 25+35+45=105, active=false: 30+40=70
    // DESC order: true first
    assert!(results[0].active);
    assert_eq!(results[0].total_age, Some(105));
    assert!(!results[1].active);
    assert_eq!(results[1].total_age, Some(70));
});

// =============================================================================
// GROUP BY with LIMIT
// =============================================================================

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_with_limit, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User).with_age(10),
        InsertComplex::new("bob", true, Role::Admin).with_age(20),
        InsertComplex::new("charlie", true, Role::User).with_age(30),
        InsertComplex::new("diana", false, Role::User).with_age(40),
        InsertComplex::new("eve", false, Role::Admin).with_age(50),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active ORDER BY SUM(age) DESC LIMIT 1 — top 1 group only
    let results: Vec<GroupSumResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(sum(complex.age), "total_age"),
        ))
        .from(complex)
        .group_by([complex.active])
        .order_by(desc(sum(complex.age)))
        .limit(1)
            => all
    );

    assert_eq!(results.len(), 1);
    // active=false: 40+50=90 > active=true: 10+20+30=60
    assert!(!results[0].active);
    assert_eq!(results[0].total_age, Some(90));
});

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_limit_without_order, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    let test_data = vec![
        InsertComplex::new("alice", true, Role::User),
        InsertComplex::new("bob", true, Role::Admin),
        InsertComplex::new("charlie", false, Role::User),
        InsertComplex::new("diana", false, Role::User),
        InsertComplex::new("eve", false, Role::Admin),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // GROUP BY active LIMIT 1 — direct LIMIT on GROUP BY
    #[derive(Debug, PostgresFromRow)]
    struct LimitGroupResult {
        active: bool,
        cnt: i64,
    }

    let results: Vec<LimitGroupResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(count(complex.id), "cnt"),
        ))
        .from(complex)
        .group_by([complex.active])
        .limit(1)
            => all
    );

    assert_eq!(results.len(), 1);
});

// =============================================================================
// GROUP BY on empty table
// =============================================================================

#[cfg(feature = "uuid")]
postgres_test!(test_group_by_empty_table, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    // No data inserted
    #[derive(Debug, PostgresFromRow)]
    struct EmptyGroupResult {
        active: bool,
        cnt: i64,
    }

    let results: Vec<EmptyGroupResult> = drizzle_exec!(
        db.select((
            complex.active,
            alias(count(complex.id), "cnt"),
        ))
        .from(complex)
        .group_by([complex.active])
            => all
    );

    assert_eq!(results.len(), 0);
});
