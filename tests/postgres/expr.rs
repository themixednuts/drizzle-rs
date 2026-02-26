//! PostgreSQL expression function tests
//!
//! Tests for string functions, math functions, CASE/WHEN, and window functions
//! executed against a real PostgreSQL database.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

// =============================================================================
// String Function Result Types
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct StringResult {
    result: String,
}

#[derive(Debug, PostgresFromRow)]
struct LengthResult {
    length: i32,
}

#[derive(Debug, PostgresFromRow)]
struct PositionResult {
    position: i32,
}

// =============================================================================
// String Function Tests
// =============================================================================

postgres_test!(test_string_upper_lower, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Hello World"),
        InsertSimple::new("Test String"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // UPPER
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(upper(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Hello World"))
            => all
    );
    assert_eq!(result[0].result, "HELLO WORLD");

    // LOWER
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(lower(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Hello World"))
            => all
    );
    assert_eq!(result[0].result, "hello world");
});

postgres_test!(test_string_trim, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("  trimmed  "),
        InsertSimple::new("  left"),
        InsertSimple::new("right  "),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // TRIM
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(trim(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.name, "  trimmed  "))
            => all
    );
    assert_eq!(result[0].result, "trimmed");

    // LTRIM
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(ltrim(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.name, "  left"))
            => all
    );
    assert_eq!(result[0].result, "left");

    // RTRIM
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(rtrim(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.name, "right  "))
            => all
    );
    assert_eq!(result[0].result, "right");
});

postgres_test!(test_string_length, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("hello"),
        InsertSimple::new(""),
        InsertSimple::new("test string"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(simple.name), "length"))
            .from(simple)
            .r#where(eq(simple.name, "hello"))
            => all
    );
    assert_eq!(result[0].length, 5);

    // Empty string
    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(simple.name), "length"))
            .from(simple)
            .r#where(eq(simple.name, ""))
            => all
    );
    assert_eq!(result[0].length, 0);
});

postgres_test!(test_string_substr, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello World")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Extract "Hello"
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(substr(simple.name, 1, 5), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello");

    // Extract "World"
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(substr(simple.name, 7, 5), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "World");
});

postgres_test!(test_string_replace, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello World")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(replace(simple.name, "World", "Rust"), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello Rust");

    // Non-existent pattern returns original
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(replace(simple.name, "xyz", "abc"), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello World");
});

postgres_test!(test_string_strpos, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello World")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Find position of "World"
    let result: Vec<PositionResult> = drizzle_exec!(
        db.select(alias(strpos(simple.name, "World"), "position"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].position, 7);

    // Non-existent pattern returns 0
    let result: Vec<PositionResult> = drizzle_exec!(
        db.select(alias(strpos(simple.name, "xyz"), "position"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].position, 0);
});

postgres_test!(test_string_concat, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Hello")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Concat with literal
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(concat(simple.name, "!"), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello!");

    // Chained concat
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(concat(concat(simple.name, " "), "there"), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "Hello there");
});

postgres_test!(test_string_functions_combined, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("  Hello World  ")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // UPPER(TRIM(name))
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(upper(trim(simple.name)), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "HELLO WORLD");

    // LOWER(TRIM(name))
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(lower(trim(simple.name)), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, "hello world");

    // LENGTH(TRIM(name))
    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(trim(simple.name)), "length"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].length, 11);
});

// =============================================================================
// Math Function Tests
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct MathIntResult {
    result: i32,
}

#[derive(Debug, PostgresFromRow)]
struct MathFloatResult {
    result: f64,
}

postgres_test!(test_math_abs, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("Negative"),
        InsertSimple::new("Zero"),
        InsertSimple::new("Positive"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // We need to use expressions with known values.
    // PostgreSQL serial starts at 1, so ids are 1, 2, 3.
    // Use arithmetic: id - 2 gives [-1, 0, 1]
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(abs(simple.id - 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Negative"))
            => all
    );
    assert_eq!(result[0].result, 1);

    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(abs(simple.id - 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Zero"))
            => all
    );
    assert_eq!(result[0].result, 0);
});

postgres_test!(test_math_round, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("Test")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // ROUND of integer division: id is serial (1), so 1/1 = 1, ROUND(1) = 1.0
    let result: Vec<MathFloatResult> = drizzle_exec!(
        db.select(alias(round(simple.id), "result"))
            .from(simple)
            => all
    );
    assert_eq!(result[0].result, 1.0);
});

postgres_test!(test_math_sign, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert rows, serial IDs will be 1, 2, 3
    let test_data = vec![
        InsertSimple::new("A"),
        InsertSimple::new("B"),
        InsertSimple::new("C"),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // id - 2: gives [-1, 0, 1] for ids [1, 2, 3]
    // PostgreSQL SIGN() returns numeric, so we use f64
    // SIGN(-1) = -1
    let result: Vec<MathFloatResult> = drizzle_exec!(
        db.select(alias(sign(simple.id - 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "A"))
            => all
    );
    assert_eq!(result[0].result, -1.0);

    // SIGN(0) = 0
    let result: Vec<MathFloatResult> = drizzle_exec!(
        db.select(alias(sign(simple.id - 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "B"))
            => all
    );
    assert_eq!(result[0].result, 0.0);

    // SIGN(1) = 1
    let result: Vec<MathFloatResult> = drizzle_exec!(
        db.select(alias(sign(simple.id - 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "C"))
            => all
    );
    assert_eq!(result[0].result, 1.0);
});

postgres_test!(test_math_mod, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert rows, serial IDs: 1, 2, 3, ...
    // We'll insert values so we know IDs predictably
    let test_data = vec![InsertSimple::new("Ten"), InsertSimple::new("Eleven")];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // IDs are serial starting at 1. We'll use the id in arithmetic.
    // id=1: (1+9) % 3 = 10 % 3 = 1  -- but we can't do literal+column easily
    // Instead test with known column values via expression:
    // mod_(simple.id, 2) for id=1 → 1 % 2 = 1
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(mod_(simple.id, 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Ten"))
            => all
    );
    assert_eq!(result[0].result, 1); // 1 % 2 = 1

    // mod_(simple.id, 2) for id=2 → 2 % 2 = 0
    let result: Vec<MathIntResult> = drizzle_exec!(
        db.select(alias(mod_(simple.id, 2), "result"))
            .from(simple)
            .r#where(eq(simple.name, "Eleven"))
            => all
    );
    assert_eq!(result[0].result, 0); // 2 % 2 = 0
});

// =============================================================================
// Aggregate on Empty Table
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct CountResult {
    count: i64,
}

#[derive(Debug, PostgresFromRow)]
struct SumNullResult {
    total: Option<i64>,
}

postgres_test!(test_aggregate_empty_table, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // No data inserted — COUNT returns 0
    let result: Vec<CountResult> = drizzle_exec!(
        db.select(alias(count(simple.id), "count"))
            .from(simple)
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].count, 0);

    // SUM on empty table returns NULL
    let result: Vec<SumNullResult> = drizzle_exec!(
        db.select(alias(sum(simple.id), "total"))
            .from(simple)
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].total, None);
});

// =============================================================================
// CASE / WHEN expressions
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct CaseNonNullResult {
    label: String,
}

#[derive(Debug, PostgresFromRow)]
struct CaseNullableResult {
    label: Option<String>,
}

postgres_test!(test_case_when_with_else, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // IDs are serial: 1, 2, 3
    // CASE WHEN id > 2 THEN 'Big' WHEN id > 1 THEN 'Medium' ELSE 'Small'
    let results: Vec<CaseNonNullResult> = drizzle_exec!(
        db.select(alias(
            case()
                .when(gt(simple.id, 2), "Big")
                .when(gt(simple.id, 1), "Medium")
                .r#else("Small"),
            "label",
        ))
        .from(simple)
        .order_by(asc(simple.id))
            => all
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].label, "Small"); // id=1
    assert_eq!(results[1].label, "Medium"); // id=2
    assert_eq!(results[2].label, "Big"); // id=3
});

postgres_test!(test_case_when_no_else, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new("alice"), InsertSimple::new("bob")];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Without ELSE, unmatched rows produce NULL
    let results: Vec<CaseNullableResult> = drizzle_exec!(
        db.select(alias(
            case()
                .when(gt(simple.id, 1), "Big")
                .end(),
            "label",
        ))
        .from(simple)
        .order_by(asc(simple.id))
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].label, None); // id=1, no match
    assert_eq!(results[1].label.as_deref(), Some("Big")); // id=2
});

// =============================================================================
// Window Functions
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct RowNumberResult {
    name: String,
    rn: i64,
}

postgres_test!(test_window_row_number, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let results: Vec<RowNumberResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(
                row_number().over(window().order_by(asc(simple.id))),
                "rn",
            ),
        ))
        .from(simple)
            => all
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[0].rn, 1);
    assert_eq!(results[1].name, "bob");
    assert_eq!(results[1].rn, 2);
    assert_eq!(results[2].name, "charlie");
    assert_eq!(results[2].rn, 3);
});

#[derive(Debug, PostgresFromRow)]
struct RunningSumResult {
    name: String,
    running_total: Option<i64>,
}

postgres_test!(test_window_sum_over, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Running sum of id ordered by id
    // IDs are serial: 1, 2, 3
    let results: Vec<RunningSumResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(
                sum(simple.id).over(
                    window()
                        .order_by(asc(simple.id))
                        .rows_between(
                            FrameBound::UnboundedPreceding,
                            FrameBound::CurrentRow,
                        )
                ),
                "running_total",
            ),
        ))
        .from(simple)
            => all
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].running_total, Some(1)); // 1
    assert_eq!(results[1].running_total, Some(3)); // 1+2
    assert_eq!(results[2].running_total, Some(6)); // 1+2+3
});

#[derive(Debug, PostgresFromRow)]
struct RankResult {
    name: String,
    rnk: i64,
}

postgres_test!(test_window_dense_rank, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let results: Vec<RankResult> = drizzle_exec!(
        db.select((
            simple.name,
            alias(
                dense_rank().over(window().order_by(asc(simple.id))),
                "rnk",
            ),
        ))
        .from(simple)
        .order_by(asc(simple.id))
            => all
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[0].rnk, 1);
    assert_eq!(results[1].name, "bob");
    assert_eq!(results[1].rnk, 2);
    assert_eq!(results[2].name, "charlie");
    assert_eq!(results[2].rnk, 3);
});

// =============================================================================
// Coalesce and Null handling
// =============================================================================

#[derive(Debug, PostgresFromRow)]
struct CoalesceResult {
    value: String,
}

#[cfg(feature = "uuid")]
postgres_test!(test_coalesce_with_null_values, ComplexSchema, {
    let ComplexSchema { role: _, complex } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("alice", true, Role::User).with_email("alice@test.com")])
            => execute
    );
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("bob", true, Role::User)])
            => execute
    );

    let results: Vec<CoalesceResult> = drizzle_exec!(
        db.select(alias(coalesce(complex.email, "no-email"), "value"))
            .from(complex)
            .order_by(asc(complex.name))
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].value, "alice@test.com");
    assert_eq!(results[1].value, "no-email");
});

// =============================================================================
// Expression Edge Cases
// =============================================================================

postgres_test!(test_empty_string_operations, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![InsertSimple::new(""), InsertSimple::new("notempty")];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Length of empty string
    let result: Vec<LengthResult> = drizzle_exec!(
        db.select(alias(length(simple.name), "length"))
            .from(simple)
            .r#where(eq(simple.name, ""))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].length, 0);

    // Upper of empty string
    let result: Vec<StringResult> = drizzle_exec!(
        db.select(alias(upper(simple.name), "result"))
            .from(simple)
            .r#where(eq(simple.name, ""))
            => all
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].result, "");
});

postgres_test!(test_arithmetic_on_serial_ids, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = vec![
        InsertSimple::new("a"),
        InsertSimple::new("b"),
        InsertSimple::new("c"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // id * 10: [10, 20, 30]
    #[derive(Debug, PostgresFromRow)]
    struct ArithResult {
        value: i32,
    }

    let results: Vec<ArithResult> = drizzle_exec!(
        db.select(alias(simple.id * 10, "value"))
            .from(simple)
            .order_by(asc(simple.id))
            => all
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].value, 10);
    assert_eq!(results[1].value, 20);
    assert_eq!(results[2].value, 30);
});
