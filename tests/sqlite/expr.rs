#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{ComplexSchema, InsertComplex};
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, Simple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[derive(Debug, SQLiteFromRow)]
struct CountResult {
    count: i64,
}

#[derive(Debug, SQLiteFromRow)]
struct SumResult {
    sum: Option<i32>,
}

#[derive(Debug, SQLiteFromRow)]
struct MinResult {
    min: Option<i32>,
}

#[derive(Debug, SQLiteFromRow)]
struct MaxResult {
    max: Option<i32>,
}

#[derive(Debug, SQLiteFromRow)]
struct AvgResult {
    avg: Option<f64>,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct SumRealResult {
    sum: Option<f64>,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct MinRealResult {
    min: Option<f64>,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct MaxRealResult {
    max: Option<f64>,
}

#[derive(Debug, SQLiteFromRow)]
struct DistinctResult {
    name: String,
}

#[derive(Debug, SQLiteFromRow)]
struct CoalesceStringResult {
    coalesce: String,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct CoalesceIntResult {
    coalesce: i32,
}

#[derive(Debug, SQLiteFromRow)]
struct AliasResult {
    item_name: String,
}

#[derive(Debug, SQLiteFromRow)]
struct CountAliasResult {
    total_count: i64,
}

#[derive(Debug, SQLiteFromRow)]
struct SumAliasResult {
    id_sum: Option<i32>,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct ComplexAggregateResult {
    count: i64,
    avg: Option<f64>,
    max_age: i32,
}

#[cfg(feature = "uuid")]
#[derive(Debug, SQLiteFromRow)]
struct CoalesceAvgResult {
    coalesce: f64,
}

sqlite_test!(test_aggregate_functions, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Item A").with_id(10),
        InsertSimple::new("Item B").with_id(20),
        InsertSimple::new("Item C").with_id(30),
        InsertSimple::new("Item D").with_id(40),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test count function
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].count, 4);

    // Test sum function
    let result: Vec<SumResult> =
        drizzle_exec!(db.select(alias(sum(simple.id), "sum")).from(simple) => all);
    assert_eq!(result[0].sum, Some(100));

    // Test min function
    let result: Vec<MinResult> =
        drizzle_exec!(db.select(alias(min(simple.id), "min")).from(simple) => all);
    assert_eq!(result[0].min, Some(10));

    // Test max function
    let result: Vec<MaxResult> =
        drizzle_exec!(db.select(alias(max(simple.id), "max")).from(simple) => all);
    assert_eq!(result[0].max, Some(40));

    // Test avg function
    let result: Vec<AvgResult> =
        drizzle_exec!(db.select(alias(avg(simple.id), "avg")).from(simple) => all);
    assert_eq!(result[0].avg, Some(25.0));
});

#[cfg(feature = "uuid")]
sqlite_test!(test_aggregate_functions_with_real_numbers, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let test_data = vec![
        InsertComplex::new("User A", true, Role::User).with_score(85.5),
        InsertComplex::new("User B", false, Role::Admin).with_score(92.0),
        InsertComplex::new("User C", true, Role::User).with_score(78.3),
        InsertComplex::new("User D", false, Role::User).with_score(88.7),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // Test count with non-null values
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(complex.score), "count"))
            .from(complex)
            => all
    );
    assert_eq!(result[0].count, 4);

    // Test sum with real numbers
    let result: Vec<SumRealResult> = drizzle_exec!(
        db.select(alias(sum(complex.score), "sum"))
            .from(complex)
            => all
    );
    assert!((result[0].sum.expect("sum") - 344.5).abs() < 0.1);

    // Test avg with real numbers
    let result: Vec<AvgResult> = drizzle_exec!(
        db.select(alias(avg(complex.score), "avg"))
            .from(complex)
            => all
    );
    assert!((result[0].avg.expect("avg") - 86.125).abs() < 0.1);

    // Test min with real numbers
    let result: Vec<MinRealResult> = drizzle_exec!(
        db.select(alias(min(complex.score), "min"))
            .from(complex)
            => all
    );
    assert!((result[0].min.expect("min") - 78.3).abs() < 0.1);

    // Test max with real numbers
    let result: Vec<MaxRealResult> = drizzle_exec!(
        db.select(alias(max(complex.score), "max"))
            .from(complex)
            => all
    );
    assert!((result[0].max.expect("max") - 92.0).abs() < 0.1);
});

sqlite_test!(test_distinct_expression, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Apple").with_id(1),
        InsertSimple::new("Apple").with_id(2),
        InsertSimple::new("Banana").with_id(3),
        InsertSimple::new("Apple").with_id(4),
        InsertSimple::new("Cherry").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test distinct function
    let result: Vec<DistinctResult> = drizzle_exec!(
        db.select(alias(distinct(simple.name), "name"))
            .from(simple)
            => all
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
            => all
    );
    assert_eq!(result[0].count, 3);
});

#[cfg(feature = "uuid")]
sqlite_test!(test_coalesce_expression, ComplexSchema, {
    let ComplexSchema { complex } = schema;

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
            => execute
    );

    // User B: has no optional fields set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User B", false, Role::Admin)])
            => execute
    );

    // Test coalesce with email field (some null, some not)
    let result: Vec<CoalesceStringResult> = drizzle_exec!(
        db.select(alias(
            coalesce(complex.email, "no-email@example.com"),
            "coalesce"
        ))
        .from(complex)
        => all
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
            => all
    );
    assert_eq!(result.len(), 3);
    // All should be 0 since we didn't set any ages
    assert!(result.iter().all(|r| r.coalesce == 0));
});

sqlite_test!(test_alias_expression, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Test Item").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test alias with simple column
    let result: Vec<AliasResult> = drizzle_exec!(
        db.select(alias(simple.name, "item_name"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].item_name, "Test Item");

    // Test alias with aggregate function
    let result: Vec<CountAliasResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "total_count"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].total_count, 1);

    // Test alias with expression
    let result: Vec<SumAliasResult> = drizzle_exec!(
        db.select(alias(sum(simple.id), "id_sum"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].id_sum, Some(1));
});

#[cfg(feature = "uuid")]
sqlite_test!(test_complex_expressions, ComplexSchema, {
    let ComplexSchema { complex } = schema;

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
            => execute
    );

    // User C: has only score set
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("User C", true, Role::User).with_score(78.3)])
            => execute
    );

    // Test multiple expressions in one query
    let result: Vec<ComplexAggregateResult> = drizzle_exec!(
        db.select((
            alias(count(complex.id), "count"),
            alias(avg(complex.score), "avg"),
            alias(coalesce(max(complex.age), 0), "max_age")
        ))
        .from(complex)
        => all
    );
    assert_eq!(result[0].count, 3); // count
    assert!((result[0].avg.expect("avg") - 85.266).abs() < 0.1); // avg score
    assert_eq!(result[0].max_age, 30); // max age (coalesced)

    // Test nested expressions
    let result: Vec<CoalesceAvgResult> = drizzle_exec!(
        db.select(alias(coalesce(avg(complex.score), 0.0), "coalesce"))
            .from(complex)
            .r#where(is_not_null(complex.score))
            => all
    );
    assert!((result[0].coalesce - 85.266).abs() < 0.1);
});

#[cfg(feature = "uuid")]
sqlite_test!(test_expressions_with_conditions, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let test_data = [
        InsertComplex::new("Active User", true, Role::User).with_score(85.5),
        InsertComplex::new("Active Admin", true, Role::Admin).with_score(92.0),
        InsertComplex::new("Inactive User", false, Role::User).with_score(78.3),
        InsertComplex::new("Inactive Admin", false, Role::Admin).with_score(88.7),
    ];

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // Test count with condition
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(complex.id), "count"))
            .from(complex)
            .r#where(eq(complex.active, true))
            => all
    );
    assert_eq!(result[0].count, 2);

    // Test avg with condition
    let result: Vec<AvgResult> = drizzle_exec!(
        db.select(alias(avg(complex.score), "avg"))
            .from(complex)
            .r#where(eq(complex.role, Role::Admin))
            => all
    );
    assert!((result[0].avg.expect("avg") - 90.35).abs() < 0.1);

    // Test max with condition
    let result: Vec<MaxRealResult> = drizzle_exec!(
        db.select(alias(max(complex.score), "max"))
            .from(complex)
            .r#where(eq(complex.active, false))
            => all
    );
    assert!((result[0].max.expect("max") - 88.7).abs() < 0.1);
});

sqlite_test!(test_aggregate_with_empty_result, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // No data inserted, test aggregate functions on empty table

    // Count should return 0
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].count, 0);

    // Other aggregates on empty table should handle NULL appropriately
    // Note: Different databases handle this differently, but typically return NULL
    // which would be handled by the driver
});

sqlite_test!(test_expression_edge_cases, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = [
        InsertSimple::new("").with_id(0), // Empty string and zero id
        InsertSimple::new("Test").with_id(1),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test count with all rows
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].count, 2);

    // Test distinct with empty string
    let result: Vec<DistinctResult> = drizzle_exec!(
        db.select(alias(distinct(simple.name), "name"))
            .from(simple)
            => all
    );
    assert_eq!(result.len(), 2);
    let names: Vec<String> = result.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"".to_string()));
    assert!(names.contains(&"Test".to_string()));

    // Test sum with zero
    let result: Vec<SumResult> =
        drizzle_exec!(db.select(alias(sum(simple.id), "sum")).from(simple) => all);
    assert_eq!(result[0].sum, Some(1));

    // Test coalesce with empty string
    let result: Vec<CoalesceStringResult> = drizzle_exec!(
        db.select(alias(coalesce(simple.name, "default"), "coalesce"))
            .from(simple)
            .r#where(eq(simple.name, ""))
            => all
    );
    assert_eq!(result[0].coalesce, ""); // Empty string is not NULL, so coalesce returns it
});

sqlite_test!(test_multiple_aliases, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = [
        InsertSimple::new("Item A").with_id(1),
        InsertSimple::new("Item B").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    #[derive(SQLiteFromRow)]
    struct ResultRow {
        identifier: i32,
        item_name: String,
        total: i64,
    }
    // Test multiple aliases in same query
    let result: Vec<ResultRow> = drizzle_exec!(
        db.select((
            alias(simple.id, "identifier"),
            alias(simple.name, "item_name"),
            alias(count(simple.id), "total")
        ))
        .from(simple)
        => all
    );

    assert_eq!(result[0].identifier, 1);
    assert_eq!(result[0].item_name, "Item A");
    assert_eq!(result[0].total, 2);
});

// CTE tests have moved to use .into_cte() API - see test_cte_integration_* tests below

sqlite_test!(test_cte_integration_simple, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    struct FilteredUsersTag;
    impl drizzle::core::Tag for FilteredUsersTag {
        const NAME: &'static str = "filtered_users";
    }

    #[derive(SQLiteFromRow)]
    struct CteSimpleRow {
        id: i32,
        name: String,
    }

    // Insert test data
    let test_data = [
        InsertSimple::new("Alice").with_id(1),
        InsertSimple::new("Bob").with_id(2),
        InsertSimple::new("Charlie").with_id(3),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Create a CTE with typed field access using .into_cte()
    let filtered_users = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(gt(simple.id, 1))
        .into_cte::<FilteredUsersTag>();

    // Use the CTE with typed column access via Deref
    let result: Vec<CteSimpleRow> = drizzle_exec!(
        db.with(&filtered_users)
            .select(CteSimpleRow::Select)
            .from(&filtered_users)
            => all
    );

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Bob");
    assert_eq!(result[1].name, "Charlie");
});

sqlite_test!(test_cte_integration_with_aggregation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    struct UserCountTag;
    impl drizzle::core::Tag for UserCountTag {
        const NAME: &'static str = "user_count";
    }

    // Insert test data
    let test_data = [
        InsertSimple::new("Test1").with_id(1),
        InsertSimple::new("Test2").with_id(2),
        InsertSimple::new("Test3").with_id(3),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Create a CTE with count using .into_cte()
    // Note: For aggregations, select the computed column using sql!() or SELECT *
    let user_count = db
        .select(count(simple.id).alias("count"))
        .from(simple)
        .into_cte::<UserCountTag>();

    #[derive(SQLiteFromRow)]
    struct CountResult {
        count: i64,
    }

    // Use the CTE with inferred FromRow selector columns
    let result: Vec<CountResult> = drizzle_exec!(
        db.with(&user_count)
            .select(CountResult::Select)
            .from(&user_count)
            => all
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].count, 3);
});

sqlite_test!(test_cte_complex_two_levels, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    struct FilteredUsersTag;
    impl drizzle::core::Tag for FilteredUsersTag {
        const NAME: &'static str = "filtered_users";
    }

    // Insert test data
    let test_data = [
        InsertSimple::new("Alice").with_id(1),
        InsertSimple::new("Bob").with_id(2),
        InsertSimple::new("Charlie").with_id(3),
        InsertSimple::new("David").with_id(4),
        InsertSimple::new("Eve").with_id(5),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Level 1 CTE: Filter users with id > 2 using .into_cte() for typed field access
    let filtered_users = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(gt(simple.id, 2))
        .into_cte::<FilteredUsersTag>();

    #[derive(SQLiteFromRow)]
    struct StatsResult {
        count: i64,
        category: Option<String>,
    }

    // Final query: Use the CTE with aggregation
    // Note: For computed columns, we use sql!() since they're not table fields
    let result: Vec<StatsResult> = drizzle_exec!(
        db.with(&filtered_users)
            .select((
                count(filtered_users.id).alias("count"),
                min(filtered_users.name).alias("category"),
            ))
            .from(&filtered_users)
            => all
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].count, 3); // Should have 3 users with id > 2 (Charlie, David, Eve)
    assert_eq!(result[0].category, Some("Charlie".to_string()));
});

sqlite_test!(test_cte_after_join, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    struct SimpleTag;
    impl drizzle::core::Tag for SimpleTag {
        const NAME: &'static str = "simple_alias";
    }
    struct JoinedSimpleTag;
    impl drizzle::core::Tag for JoinedSimpleTag {
        const NAME: &'static str = "joined_simple";
    }

    let test_data = [
        InsertSimple::new("Alpha").with_id(1),
        InsertSimple::new("Beta").with_id(2),
        InsertSimple::new("Gamma").with_id(3),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let simple_alias = Simple::alias::<SimpleTag>();
    let joined_simple = db
        .select((simple.id, simple.name))
        .from(simple)
        .join((simple_alias, eq(simple.id, simple_alias.id)))
        .into_cte::<JoinedSimpleTag>();

    let results: Vec<SelectSimple> = drizzle_exec!(
        db.with(&joined_simple)
            .select((joined_simple.id, joined_simple.name))
            .from(&joined_simple)
            .order_by([asc(joined_simple.id)])
            => all
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "Alpha");
    assert_eq!(results[2].name, "Gamma");
});

sqlite_test!(test_cte_after_order_limit_offset, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    struct PagedSimpleTag;
    impl drizzle::core::Tag for PagedSimpleTag {
        const NAME: &'static str = "paged_simple";
    }

    let test_data = [
        InsertSimple::new("One").with_id(1),
        InsertSimple::new("Two").with_id(2),
        InsertSimple::new("Three").with_id(3),
        InsertSimple::new("Four").with_id(4),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let paged_simple = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([asc(simple.id)])
        .limit(2)
        .offset(1)
        .into_cte::<PagedSimpleTag>();

    let results: Vec<SelectSimple> = drizzle_exec!(
        db.with(&paged_simple)
            .select((paged_simple.id, paged_simple.name))
            .from(&paged_simple)
            .order_by([asc(paged_simple.id)])
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 2);
    assert_eq!(results[1].id, 3);
});

// =============================================================================
// New Expression DX Tests
// =============================================================================

sqlite_test!(test_modulo_operator, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Item A").with_id(10),
        InsertSimple::new("Item B").with_id(15),
        InsertSimple::new("Item C").with_id(23),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test modulo operator - find items where id % 5 == 0
    // Numeric columns now have arithmetic operators directly!
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.id % 5, 0))
            => all
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, 10);
    assert_eq!(result[1].id, 15);

    // Test modulo with different values
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.id % 10, 3))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, 23);
});

sqlite_test!(test_between_method, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Item A").with_id(5),
        InsertSimple::new("Item B").with_id(10),
        InsertSimple::new("Item C").with_id(15),
        InsertSimple::new("Item D").with_id(20),
        InsertSimple::new("Item E").with_id(25),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test between method - find items with id between 10 and 20 (inclusive)
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(simple.id.between(10, 20))
            => all
    );
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].id, 10);
    assert_eq!(result[1].id, 15);
    assert_eq!(result[2].id, 20);

    // Test not_between method
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(simple.id.not_between(10, 20))
            => all
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, 5);
    assert_eq!(result[1].id, 25);
});

sqlite_test!(test_in_array_method, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Alice").with_id(1),
        InsertSimple::new("Bob").with_id(2),
        InsertSimple::new("Charlie").with_id(3),
        InsertSimple::new("David").with_id(4),
        InsertSimple::new("Eve").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test in_array method with integers
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(simple.id.in_array([1, 3, 5]))
            => all
    );
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].name, "Alice");
    assert_eq!(result[1].name, "Charlie");
    assert_eq!(result[2].name, "Eve");

    // Test not_in_array method
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(simple.id.not_in_array([1, 3, 5]))
            => all
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Bob");
    assert_eq!(result[1].name, "David");

    // Test in_array with strings
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(simple.name.in_array(["Alice", "Eve"]))
            => all
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Alice");
    assert_eq!(result[1].name, "Eve");
});

sqlite_test!(test_column_arithmetic, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Item A").with_id(10),
        InsertSimple::new("Item B").with_id(20),
        InsertSimple::new("Item C").with_id(30),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    #[derive(Debug, SQLiteFromRow)]
    struct ComputedResult {
        computed: i32,
    }

    // Test direct column arithmetic: simple.id * 2
    let result: Vec<ComputedResult> = drizzle_exec!(
        db.select(alias(simple.id * 2, "computed"))
            .from(simple)
            => all
    );
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].computed, 20); // 10 * 2
    assert_eq!(result[1].computed, 40); // 20 * 2
    assert_eq!(result[2].computed, 60); // 30 * 2

    // Test column arithmetic in comparison
    let result: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(lt(simple.id, 25))
            => all
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, 10);
    assert_eq!(result[1].id, 20);
});

// =============================================================================
// String Function Tests
// =============================================================================

#[derive(Debug, SQLiteFromRow)]
struct StringResult {
    result: String,
}

#[derive(Debug, SQLiteFromRow)]
struct LengthResult {
    length: i64,
}

#[derive(Debug, SQLiteFromRow)]
struct InstrResult {
    position: i64,
}

sqlite_test!(test_string_upper_lower, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Hello World").with_id(1),
        InsertSimple::new("Test String").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test UPPER function
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(upper(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );
    assert_eq!(result[0].result, "HELLO WORLD");

    // Test LOWER function
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(lower(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );
    assert_eq!(result[0].result, "hello world");
});

sqlite_test!(test_string_trim, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("  trimmed  ").with_id(1),
        InsertSimple::new("  left").with_id(2),
        InsertSimple::new("right  ").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test TRIM function
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(trim(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );
    assert_eq!(result[0].result, "trimmed");

    // Test LTRIM function
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(ltrim(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.id, 2))
            => all
    );
    assert_eq!(result[0].result, "left");

    // Test RTRIM function
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(rtrim(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.id, 3))
            => all
    );
    assert_eq!(result[0].result, "right");
});

sqlite_test!(test_string_length, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("hello").with_id(1),
        InsertSimple::new("").with_id(2),
        InsertSimple::new("test string").with_id(3),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test LENGTH function
    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(simple.name), "length"))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );
    assert_eq!(result[0].length, 5);

    // Test LENGTH with empty string
    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(simple.name), "length"))
            .from(simple)
            .r#where(eq(simple.id, 2))
            => all
    );
    assert_eq!(result[0].length, 0);
});

sqlite_test!(test_string_substr, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello World").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test SUBSTR function - extract "Hello"
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(substr(simple.name, 1, 5), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello");

    // Test SUBSTR function - extract "World"
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(substr(simple.name, 7, 5), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "World");
});

sqlite_test!(test_string_replace, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello World").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test REPLACE function
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(replace(simple.name, "World", "Rust"), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello Rust");

    // Test REPLACE with non-existent pattern (should return original)
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(replace(simple.name, "xyz", "abc"), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello World");
});

sqlite_test!(test_string_instr, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello World").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test INSTR function - find position of "World"
    let result: Vec<InstrResult> = drizzle_exec!(
        db.select(alias(instr(simple.name, "World"), "position"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].position, 7);

    // Test INSTR with non-existent pattern (should return 0)
    let result: Vec<InstrResult> = drizzle_exec!(
        db.select(alias(instr(simple.name, "xyz"), "position"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].position, 0);
});

sqlite_test!(test_string_concat, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Hello").with_id(1),
        InsertSimple::new("World").with_id(2),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test concat function with literal
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(concat(simple.name, "!"), "result"))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );
    assert_eq!(result[0].result, "Hello!");

    // Test chained concat
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(concat(concat(simple.name, " "), "there"), "result"))
            .from(simple)
            .r#where(eq(simple.id, 1))
            => all
    );
    assert_eq!(result[0].result, "Hello there");
});

sqlite_test!(test_string_functions_combined, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("  Hello World  ").with_id(1)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test combined: UPPER(TRIM(name))
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(upper(trim(simple.name)), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "HELLO WORLD");

    // Test combined: LOWER(TRIM(name))
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(lower(trim(simple.name)), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "hello world");

    // Test combined: LENGTH(TRIM(name))
    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(trim(simple.name)), "length"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].length, 11); // "Hello World" without leading/trailing spaces
});

// =============================================================================
// Math Function Tests
// =============================================================================

#[derive(Debug, SQLiteFromRow)]
struct MathIntResult {
    result: i32,
}

#[derive(Debug, SQLiteFromRow)]
struct MathFloatResult {
    result: f64,
}

sqlite_test!(test_math_abs, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Negative").with_id(-10),
        InsertSimple::new("Zero").with_id(0),
        InsertSimple::new("Positive").with_id(10),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test ABS function
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(abs(simple.id), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Negative"))
            => all
    );
    assert_eq!(result[0].result, 10);

    // Test ABS with zero
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(abs(simple.id), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Zero"))
            => all
    );
    assert_eq!(result[0].result, 0);
});

sqlite_test!(test_math_round, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Use a table with float values - we'll compute from integer for simplicity
    let test_data = vec![InsertSimple::new("Test").with_id(37)];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test ROUND function with computed expression (id / 10.0)
    // 37 / 10.0 = 3.7, ROUND(3.7) = 4.0
    let result: Vec<MathFloatResult> = drizzle_exec!(
        db.select(alias(round(simple.id / 10), "result"))
            .from(simple)
            => all
    );
    // Integer division: 37 / 10 = 3, ROUND(3) = 3.0
    assert_eq!(result[0].result, 3.0);
});

sqlite_test!(test_math_sign, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Negative").with_id(-5),
        InsertSimple::new("Zero").with_id(0),
        InsertSimple::new("Positive").with_id(5),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test SIGN function with negative
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(sign(simple.id), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Negative"))
            => all
    );
    assert_eq!(result[0].result, -1);

    // Test SIGN with zero
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(sign(simple.id), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Zero"))
            => all
    );
    assert_eq!(result[0].result, 0);

    // Test SIGN with positive
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(sign(simple.id), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Positive"))
            => all
    );
    assert_eq!(result[0].result, 1);
});

sqlite_test!(test_math_mod, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Ten").with_id(10),
        InsertSimple::new("Seven").with_id(7),
        InsertSimple::new("Fifteen").with_id(15),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test MOD function (10 % 3 = 1)
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(mod_(simple.id, 3), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Ten"))
            => all
    );
    assert_eq!(result[0].result, 1);

    // Test MOD function (15 % 4 = 3)
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(mod_(simple.id, 4), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Fifteen"))
            => all
    );
    assert_eq!(result[0].result, 3);
});

// =============================================================================
// DateTime Function Tests
// =============================================================================

#[derive(Debug, SQLiteFromRow)]
struct DateResult {
    result: String,
}

#[derive(Debug, SQLiteFromRow)]
struct CurrentDateResult {
    today: String,
}

sqlite_test!(test_datetime_current, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Test").with_id(1)];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test CURRENT_DATE - returns format YYYY-MM-DD
    let result: Vec<CurrentDateResult> = drizzle_exec!(db.select(alias(cast::<_, _, drizzle::core::types::Text>(current_date(), "TEXT"), "today")).from(simple) => all);
    // Just verify it's in the expected format (YYYY-MM-DD)
    assert!(result[0].today.len() == 10);
    assert!(result[0].today.contains('-'));
});

sqlite_test!(test_datetime_strftime, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Test").with_id(1)];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test STRFTIME to format current date
    let result: Vec<DateResult> = drizzle_exec!(
        db.select(alias(strftime("%Y", current_date()), "result"))
            .from(simple)
            => all
    );
    // Should return 4-digit year
    assert!(result[0].result.len() == 4);
    assert!(result[0].result.starts_with("20")); // Years 2000-2099
});

// =============================================================================
// SQLTypeToRust Inference Tests
// =============================================================================
// These tests verify that expression functions (current_date, count_all, etc.)
// produce types that can be deserialized correctly via the String fallback
// when chrono/uuid/serde features are not enabled.

#[derive(Debug, SQLiteFromRow)]
struct InferredDateResult {
    today: String,
}

// Tests that current_date() infers String (or chrono::NaiveDate with chrono)
// and deserializes correctly from SQLite's TEXT representation.
sqlite_test!(test_inferred_current_date, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    drizzle_exec!(db.insert(simple).values([InsertSimple::new("seed")]) => execute);

    let result: Vec<InferredDateResult> = drizzle_exec!(db.select(alias(cast::<_, _, drizzle::core::types::Text>(current_date(), "TEXT"), "today")).from(simple) => all);
    assert_eq!(result.len(), 1);
    // SQLite returns YYYY-MM-DD for CURRENT_DATE
    assert_eq!(result[0].today.len(), 10);
    assert!(result[0].today.contains('-'));
});

#[derive(Debug, SQLiteFromRow)]
struct InferredTimestampResult {
    now: String,
}

// Tests that current_timestamp() infers correctly and deserializes from SQLite.
sqlite_test!(test_inferred_current_timestamp, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    drizzle_exec!(db.insert(simple).values([InsertSimple::new("seed")]) => execute);

    let result: Vec<InferredTimestampResult> = drizzle_exec!(db.select(alias(cast::<_, _, drizzle::core::types::Text>(current_timestamp(), "TEXT"), "now")).from(simple) => all);
    assert_eq!(result.len(), 1);
    // SQLite returns YYYY-MM-DD HH:MM:SS for CURRENT_TIMESTAMP
    assert!(result[0].now.contains(' '));
    assert!(result[0].now.contains(':'));
});
