//! PostgreSQL SELECT query tests
//!
//! Tests for SELECT statement generation and execution with PostgreSQL-specific syntax.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::pg::*;
use drizzle::prelude::*;
use drizzle_core::OrderBy;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

postgres_test!(simple_select_with_conditions, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertPgSimple::new("alpha"),
        InsertPgSimple::new("beta"),
        InsertPgSimple::new("gamma"),
        InsertPgSimple::new("delta"),
    ];

    let stmt = db.insert(simple).values(test_data);
    println!("Insert stmt: {}", stmt.to_sql());
    drizzle_exec!(stmt.execute());

    // Test WHERE condition
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "beta"));
    println!("Select where stmt: {}", stmt.to_sql());

    let where_results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(where_results.len(), 1);
    assert_eq!(where_results[0].name, "beta");

    // Test ORDER BY with LIMIT
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2);
    println!("Select order stmt: {}", stmt.to_sql());

    let ordered_results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(ordered_results.len(), 2);
    assert_eq!(ordered_results[0].name, "alpha");
    assert_eq!(ordered_results[1].name, "beta");

    // Test LIMIT with OFFSET
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2)
        .offset(2);
    println!("Select limit stmt: {}", stmt.to_sql());

    let offset_results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(offset_results.len(), 2);
    assert_eq!(offset_results[0].name, "delta");
    assert_eq!(offset_results[1].name, "gamma");
});

postgres_test!(select_all_columns, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    // Insert test data
    let stmt = db.insert(simple).values(vec![InsertPgSimple::new("test")]);
    drizzle_exec!(stmt.execute());

    // Select all columns - SQL generation test
    let stmt = db.select(()).from(simple);

    let sql = stmt.to_sql().sql();
    println!("Select all SQL: {}", sql);

    // Should include all columns from the table
    assert!(sql.contains(r#""pg_simple"."id""#));
    assert!(sql.contains(r#""pg_simple"."name""#));
});

postgres_test!(select_with_where, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("test"),
        InsertPgSimple::new("other"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "test"));

    let sql = stmt.to_sql().sql();
    println!("Select with WHERE SQL: {}", sql);

    assert!(sql.contains("WHERE"));
    assert!(sql.contains(r#""pg_simple"."name""#));
    // PostgreSQL uses $1 for parameters
    assert!(
        sql.contains("$1"),
        "Expected PostgreSQL $1 placeholder: {}",
        sql
    );

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

postgres_test!(select_with_order_by, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("zebra"),
        InsertPgSimple::new("alpha"),
        InsertPgSimple::new("beta"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2);

    let sql = stmt.to_sql().sql();
    println!("Order by SQL: {}", sql);

    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains(r#""pg_simple"."name""#));
    assert!(sql.contains("ASC"));
    assert!(sql.contains("LIMIT"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alpha");
    assert_eq!(results[1].name, "beta");
});

postgres_test!(select_with_limit, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("one"),
        InsertPgSimple::new("two"),
        InsertPgSimple::new("three"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple).limit(2);

    let sql = stmt.to_sql().sql();
    println!("Select with LIMIT SQL: {}", sql);

    assert!(sql.contains("LIMIT"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);
});

postgres_test!(select_with_offset, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("one"),
        InsertPgSimple::new("two"),
        InsertPgSimple::new("three"),
        InsertPgSimple::new("four"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2)
        .offset(1);

    let sql = stmt.to_sql().sql();
    println!("Select with LIMIT and OFFSET SQL: {}", sql);

    assert!(sql.contains("LIMIT"));
    assert!(sql.contains("OFFSET"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);
    // After ordering: four, one, three, two - offset 1 skips "four"
    assert_eq!(results[0].name, "one");
    assert_eq!(results[1].name, "three");
});

// Validate that the generated Select model can be used directly
postgres_test!(select_with_generated_model, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("sel_a"),
        InsertPgSimple::new("sel_b"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(simple)
        .order_by([OrderBy::asc(simple.id)]);

    let results: Vec<SelectPgSimple> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "sel_a");
    assert_eq!(results[1].name, "sel_b");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_multiple_order_by, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertPgComplex::new("Alice", true, PgRole::User)
            .with_email("alice@example.com")
            .with_age(30),
        InsertPgComplex::new("Bob", true, PgRole::User)
            .with_email("bob@example.com")
            .with_age(25),
        InsertPgComplex::new("Charlie", true, PgRole::User)
            .with_email("charlie@example.com")
            .with_age(30),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .order_by([OrderBy::desc(complex.age), OrderBy::asc(complex.name)]);

    let sql = stmt.to_sql().sql();
    println!("Select with multiple ORDER BY SQL: {}", sql);

    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains("DESC"));
    assert!(sql.contains("ASC"));
});

postgres_test!(select_with_in_array, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("Alice"),
        InsertPgSimple::new("Bob"),
        InsertPgSimple::new("Charlie"),
        InsertPgSimple::new("David"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(simple)
        .r#where(in_array(simple.name, ["Alice", "Bob", "Charlie"]));

    let sql = stmt.to_sql().sql();
    println!("Select with IN SQL: {}", sql);

    assert!(sql.contains("IN"));
    // Should have PostgreSQL numbered placeholders
    assert!(sql.contains("$1"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 3);
});

postgres_test!(select_with_like_pattern, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("test_one"),
        InsertPgSimple::new("test_two"),
        InsertPgSimple::new("other"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(simple)
        .r#where(like(simple.name, "%test%"));

    let sql = stmt.to_sql().sql();
    println!("Select with LIKE SQL: {}", sql);

    assert!(sql.contains("LIKE"));
    assert!(sql.contains("$1"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_null_check, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let data1 = InsertPgComplex::new("Alice", true, PgRole::User)
        .with_email("alice@example.com")
        .with_age(30);

    let stmt = db.insert(complex).values(vec![data1]);
    drizzle_exec!(stmt.execute());

    let data2 = InsertPgComplex::new("Bob", true, PgRole::User).with_age(25);
    let stmt = db.insert(complex).values(vec![data2]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex).r#where(is_null(complex.email));

    let sql = stmt.to_sql().sql();
    println!("IS NULL condition SQL: {}", sql);

    assert!(sql.contains("IS NULL"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_between, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertPgComplex::new("Young", true, PgRole::User)
            .with_email("young@example.com")
            .with_age(15),
        InsertPgComplex::new("Adult", true, PgRole::User)
            .with_email("adult@example.com")
            .with_age(30),
        InsertPgComplex::new("Senior", true, PgRole::User)
            .with_email("senior@example.com")
            .with_age(70),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(between(complex.age, 18, 65));

    let sql = stmt.to_sql().sql();
    println!("Select with BETWEEN SQL: {}", sql);

    assert!(sql.contains("BETWEEN"));
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Adult");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_enum_condition, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let data1 = InsertPgComplex::new("Alice", true, PgRole::Admin)
        .with_email("alice@example.com")
        .with_age(30);
    let data2 = InsertPgComplex::new("Bob", true, PgRole::User)
        .with_email("bob@example.com")
        .with_age(25);

    let stmt = db.insert(complex).values(vec![data1, data2]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.role, PgRole::Admin));

    let sql = stmt.to_sql().sql();
    println!("Select with enum condition SQL: {}", sql);

    assert!(sql.contains(r#""pg_complex"."role""#));
    assert!(sql.contains("$1"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

#[cfg(feature = "uuid")]
postgres_test!(select_complex_where, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let data1 = InsertPgComplex::new("Alice", true, PgRole::Admin)
        .with_email("alice@example.com")
        .with_age(30);

    let data2 = InsertPgComplex::new("Bob", true, PgRole::User)
        .with_email("bob@example.com")
        .with_age(25);

    let data3 = InsertPgComplex::new("Charlie", false, PgRole::User)
        .with_email("charlie@example.com")
        .with_age(20);

    let stmt = db.insert(complex).values(vec![data1, data2, data3]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex).r#where(and([
        eq(complex.active, true),
        or([eq(complex.role, PgRole::Admin), gt(complex.age, 21)]),
    ]));

    let sql = stmt.to_sql().sql();
    println!("Complex WHERE SQL: {}", sql);

    assert!(sql.contains("AND"));
    assert!(sql.contains("OR"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    // Should match Alice (active=true, role=Admin) and Bob (active=true, age>21)
    assert_eq!(results.len(), 2);
});

postgres_test!(select_with_aggregate_count, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertPgSimple::new("one"),
        InsertPgSimple::new("two"),
        InsertPgSimple::new("three"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(alias(count(simple.id), "count")).from(simple);

    let sql = stmt.to_sql().sql();
    println!("Select with COUNT SQL: {}", sql);

    assert!(sql.contains("COUNT"));
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_aggregate_sum, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertPgComplex::new("Alice", true, PgRole::User)
            .with_email("alice@example.com")
            .with_age(30),
        InsertPgComplex::new("Bob", true, PgRole::User)
            .with_email("bob@example.com")
            .with_age(25),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(alias(sum(complex.age), "total_age"))
        .from(complex);

    let sql = stmt.to_sql().sql();
    println!("Select with SUM SQL: {}", sql);

    assert!(sql.contains("SUM"));
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_aggregate_avg, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertPgComplex::new("Alice", true, PgRole::User)
            .with_email("alice@example.com")
            .with_age(30),
        InsertPgComplex::new("Bob", true, PgRole::User)
            .with_email("bob@example.com")
            .with_age(20),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(alias(avg(complex.score), "avg_score"))
        .from(complex);

    let sql = stmt.to_sql().sql();
    println!("Select with AVG SQL: {}", sql);

    assert!(sql.contains("AVG"));
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_aggregate_min_max, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertPgComplex::new("Alice", true, PgRole::User)
            .with_email("alice@example.com")
            .with_age(30),
        InsertPgComplex::new("Bob", true, PgRole::User)
            .with_email("bob@example.com")
            .with_age(25),
        InsertPgComplex::new("Charlie", true, PgRole::User)
            .with_email("charlie@example.com")
            .with_age(35),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((
            alias(min(complex.age), "min_age"),
            alias(max(complex.age), "max_age"),
        ))
        .from(complex);

    let sql = stmt.to_sql().sql();
    println!("Select with MIN/MAX SQL: {}", sql);

    assert!(sql.contains("MIN"));
    assert!(sql.contains("MAX"));
});

#[cfg(feature = "uuid")]
postgres_test!(select_distinct, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertPgComplex::new("Alice", true, PgRole::User)
            .with_email("alice@example.com")
            .with_age(30),
        InsertPgComplex::new("Bob", true, PgRole::User)
            .with_email("bob@example.com")
            .with_age(25),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(alias(distinct(complex.role), "role"))
        .from(complex);

    let sql = stmt.to_sql().sql();
    println!("Select DISTINCT SQL: {}", sql);

    assert!(sql.contains("DISTINCT"));
});

postgres_test!(select_with_alias, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![InsertPgSimple::new("test")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(alias(simple.name, "user_name")).from(simple);

    let sql = stmt.to_sql().sql();
    println!("Select with alias SQL: {}", sql);

    assert!(sql.contains("AS"));
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_coalesce, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let data = InsertPgComplex::new("Alice", true, PgRole::User).with_age(30);

    let stmt = db.insert(complex).values(vec![data]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(alias(
            coalesce(complex.email, "unknown@example.com"),
            "email",
        ))
        .from(complex);

    let sql = stmt.to_sql().sql();
    println!("Select with COALESCE SQL: {}", sql);

    assert!(sql.contains("COALESCE"));
});
