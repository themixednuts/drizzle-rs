//! PostgreSQL SELECT query tests
//!
//! Tests for SELECT statement generation and execution with PostgreSQL-specific syntax.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_core::OrderBy;
#[cfg(feature = "uuid")]
use drizzle_core::types::Double;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[derive(Debug, PostgresFromRow)]
struct PgCountResult {
    count: i64,
}

#[derive(Debug, PostgresFromRow)]
struct PgSumResult {
    total_age: i64,
}

#[derive(Debug, PostgresFromRow)]
struct PgAvgResult {
    avg_age: f64,
}

#[derive(Debug, PostgresFromRow)]
struct PgMinMaxResult {
    min_age: i32,
    max_age: i32,
}

#[derive(Debug, PostgresFromRow)]
struct PgAliasResult {
    user_name: String,
}

#[derive(Debug, PostgresFromRow)]
struct PgCoalesceResult {
    email: String,
}

#[allow(dead_code)]
#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
}

postgres_test!(simple_select_with_conditions, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alpha"),
        InsertSimple::new("beta"),
        InsertSimple::new("gamma"),
        InsertSimple::new("delta"),
    ];

    let stmt = db.insert(simple).values(test_data);
    drizzle_exec!(stmt => execute);

    // Test WHERE condition
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "beta"));

    let where_results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(where_results.len(), 1);
    assert_eq!(where_results[0].name, "beta");

    // Test ORDER BY with LIMIT
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2);

    let ordered_results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

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

    let offset_results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);

    assert_eq!(offset_results.len(), 2);
    assert_eq!(offset_results[0].name, "delta");
    assert_eq!(offset_results[1].name, "gamma");
});

postgres_test!(select_all_columns, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values(vec![InsertSimple::new("test")])
            => execute
    );

    let sql = db.select(()).from(simple).to_sql().sql();
    assert_eq!(
        sql,
        r#"SELECT "simple"."id", "simple"."name" FROM "simple""#
    );
});

postgres_test!(select_with_where, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values(vec![InsertSimple::new("test"), InsertSimple::new("other")])
            => execute
    );

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "test"));

    assert_eq!(
        stmt.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE "simple"."name" = $1"#
    );

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

postgres_test!(select_with_order_by, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values(vec![
                InsertSimple::new("zebra"),
                InsertSimple::new("alpha"),
                InsertSimple::new("beta"),
            ])
            => execute
    );

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2);

    assert_eq!(
        stmt.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" ORDER BY "simple"."name" ASC LIMIT 2"#
    );

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alpha");
    assert_eq!(results[1].name, "beta");
});

postgres_test!(select_with_limit, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values(vec![
                InsertSimple::new("one"),
                InsertSimple::new("two"),
                InsertSimple::new("three"),
            ])
            => execute
    );

    let stmt = db.select((simple.id, simple.name)).from(simple).limit(2);

    assert_eq!(
        stmt.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" LIMIT 2"#
    );

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);
});

postgres_test!(select_with_offset, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("one"),
                InsertSimple::new("two"),
                InsertSimple::new("three"),
                InsertSimple::new("four"),
            ])
            => execute
    );

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .order_by([OrderBy::asc(simple.name)])
        .limit(2)
        .offset(1);

    assert_eq!(
        stmt.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" ORDER BY "simple"."name" ASC LIMIT 2 OFFSET 1"#
    );

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);
    // After ordering: four, one, three, two - offset 1 skips "four"
    assert_eq!(results[0].name, "one");
    assert_eq!(results[1].name, "three");
});

postgres_test!(cte_after_join, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = [
        InsertSimple::new("alpha"),
        InsertSimple::new("beta"),
        InsertSimple::new("gamma"),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let results: Vec<PgSimpleResult> = {
        let simple_alias = Simple::alias("simple_alias");
        let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
        let join_cond = eq(simple.id, simple_alias.id);
        let joined_simple: drizzle_postgres::builder::CTEView<'static, _, _> = builder
            .select((simple.id, simple.name))
            .from(simple)
            .join((simple_alias, join_cond))
            .into_cte("joined_simple");
        let joined_alias = joined_simple.table;

        drizzle_exec!(
            db.with(&joined_simple)
                .select((joined_alias.id, joined_alias.name))
                .from(&joined_simple)
                .order_by([OrderBy::asc(joined_alias.id)])
                .all()
        )
    };

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "alpha");
    assert_eq!(results[2].name, "gamma");
});

postgres_test!(cte_after_order_limit_offset, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let test_data = [
        InsertSimple::new("one"),
        InsertSimple::new("two"),
        InsertSimple::new("three"),
        InsertSimple::new("four"),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let results: Vec<PgSimpleResult> = {
        let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
        let paged_simple: drizzle_postgres::builder::CTEView<'static, _, _> = builder
            .select((simple.id, simple.name))
            .from(simple)
            .order_by([OrderBy::asc(simple.id)])
            .limit(2)
            .offset(1)
            .into_cte("paged_simple");
        let paged_alias = paged_simple.table;

        drizzle_exec!(
            db.with(&paged_simple)
                .select((paged_alias.id, paged_alias.name))
                .from(&paged_simple)
                .order_by([OrderBy::asc(paged_alias.id)])
                .all()
        )
    };

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 2);
    assert_eq!(results[1].id, 3);
});

// Validate that the generated Select model can be used directly
postgres_test!(select_with_generated_model, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values(vec![InsertSimple::new("sel_a"), InsertSimple::new("sel_b")]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(())
        .from(simple)
        .order_by([OrderBy::asc(simple.id)]);

    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "sel_a");
    assert_eq!(results[1].name, "sel_b");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_multiple_order_by, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values(vec![
                InsertComplex::new("Alice", true, Role::User)
                    .with_email("alice@example.com")
                    .with_age(30),
                InsertComplex::new("Bob", true, Role::User)
                    .with_email("bob@example.com")
                    .with_age(25),
                InsertComplex::new("Charlie", true, Role::User)
                    .with_email("charlie@example.com")
                    .with_age(30),
            ])
            => execute
    );

    let stmt = db
        .select((complex.id, complex.name, complex.email, complex.age))
        .from(complex)
        .order_by([OrderBy::desc(complex.age), OrderBy::asc(complex.name)]);

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 3);
    // age DESC, name ASC: Alice(30), Charlie(30), Bob(25)
    assert_eq!(results[0].name, "Alice");
    assert_eq!(results[1].name, "Charlie");
    assert_eq!(results[2].name, "Bob");
});

postgres_test!(select_with_in_array, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
        InsertSimple::new("David"),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(())
        .from(simple)
        .r#where(in_array(simple.name, ["Alice", "Bob", "Charlie"]));

    let sql = stmt.to_sql().sql();

    assert!(sql.contains("IN"));
    // Should have PostgreSQL numbered placeholders
    assert!(sql.contains("$1"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 3);
});

postgres_test!(select_with_like_pattern, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values(vec![
        InsertSimple::new("test_one"),
        InsertSimple::new("test_two"),
        InsertSimple::new("other"),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(())
        .from(simple)
        .r#where(like(simple.name, "%test%"));

    let sql = stmt.to_sql().sql();

    assert!(sql.contains("LIKE"));
    assert!(sql.contains("$1"));

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 2);
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_null_check, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let data1 = InsertComplex::new("Alice", true, Role::User)
        .with_email("alice@example.com")
        .with_age(30);

    let stmt = db.insert(complex).values(vec![data1]);
    drizzle_exec!(stmt => execute);

    let data2 = InsertComplex::new("Bob", true, Role::User).with_age(25);
    let stmt = db.insert(complex).values(vec![data2]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(complex).r#where(is_null(complex.email));

    let sql = stmt.to_sql().sql();

    assert!(sql.contains("IS NULL"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_between, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values(vec![
        InsertComplex::new("Young", true, Role::User)
            .with_email("young@example.com")
            .with_age(15),
        InsertComplex::new("Adult", true, Role::User)
            .with_email("adult@example.com")
            .with_age(30),
        InsertComplex::new("Senior", true, Role::User)
            .with_email("senior@example.com")
            .with_age(70),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(())
        .from(complex)
        .r#where(between(complex.age, 18, 65));

    let sql = stmt.to_sql().sql();

    assert!(sql.contains("BETWEEN"));
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Adult");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_enum_condition, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let data1 = InsertComplex::new("Alice", true, Role::Admin)
        .with_email("alice@example.com")
        .with_age(30);
    let data2 = InsertComplex::new("Bob", true, Role::User)
        .with_email("bob@example.com")
        .with_age(25);

    let stmt = db.insert(complex).values(vec![data1, data2]);
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.role, Role::Admin));

    let sql = stmt.to_sql().sql();

    assert!(sql.contains(r#""complex"."role""#));
    assert!(sql.contains("$1"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

#[cfg(feature = "uuid")]
postgres_test!(select_complex_where, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let data1 = InsertComplex::new("Alice", true, Role::Admin)
        .with_email("alice@example.com")
        .with_age(30);

    let data2 = InsertComplex::new("Bob", true, Role::User)
        .with_email("bob@example.com")
        .with_age(25);

    let data3 = InsertComplex::new("Charlie", false, Role::User)
        .with_email("charlie@example.com")
        .with_age(20);

    let stmt = db.insert(complex).values(vec![data1, data2, data3]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(complex).r#where(and([
        eq(complex.active, true),
        or([eq(complex.role, Role::Admin), gt(complex.age, 21)]),
    ]));

    let sql = stmt.to_sql().sql();

    assert!(sql.contains("AND"));
    assert!(sql.contains("OR"));

    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);
    // Should match Alice (active=true, role=Admin) and Bob (active=true, age>21)
    assert_eq!(results.len(), 2);
});

postgres_test!(select_with_aggregate_count, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values(vec![
                InsertSimple::new("one"),
                InsertSimple::new("two"),
                InsertSimple::new("three"),
            ])
            => execute
    );

    let stmt = db.select(alias(count(simple.id), "count")).from(simple);

    let result: PgCountResult = drizzle_exec!(stmt => get);
    assert_eq!(result.count, 3);
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_aggregate_sum, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values(vec![
                InsertComplex::new("Alice", true, Role::User)
                    .with_email("alice@example.com")
                    .with_age(30),
                InsertComplex::new("Bob", true, Role::User)
                    .with_email("bob@example.com")
                    .with_age(25),
            ])
            => execute
    );

    let stmt = db
        .select(alias(sum(complex.age), "total_age"))
        .from(complex);

    let result: PgSumResult = drizzle_exec!(stmt => get);
    assert_eq!(result.total_age, 55);
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_aggregate_avg, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values(vec![
                InsertComplex::new("Alice", true, Role::User)
                    .with_email("alice@example.com")
                    .with_age(30),
                InsertComplex::new("Bob", true, Role::User)
                    .with_email("bob@example.com")
                    .with_age(20),
            ])
            => execute
    );

    let stmt = db
        .select(alias(
            cast::<_, _, Double>(avg(complex.age), "float8"),
            "avg_age",
        ))
        .from(complex);

    let result: PgAvgResult = drizzle_exec!(stmt => get);
    assert!((result.avg_age - 25.0).abs() < 0.01);
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_aggregate_min_max, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values(vec![
                InsertComplex::new("Alice", true, Role::User)
                    .with_email("alice@example.com")
                    .with_age(30),
                InsertComplex::new("Bob", true, Role::User)
                    .with_email("bob@example.com")
                    .with_age(25),
                InsertComplex::new("Charlie", true, Role::User)
                    .with_email("charlie@example.com")
                    .with_age(35),
            ])
            => execute
    );

    let stmt = db
        .select((
            alias(min(complex.age), "min_age"),
            alias(max(complex.age), "max_age"),
        ))
        .from(complex);

    let result: PgMinMaxResult = drizzle_exec!(stmt => get);
    assert_eq!(result.min_age, 25);
    assert_eq!(result.max_age, 35);
});

#[cfg(feature = "uuid")]
postgres_test!(select_distinct, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values(vec![
                InsertComplex::new("Alice", true, Role::User)
                    .with_email("alice@example.com")
                    .with_age(30),
                InsertComplex::new("Bob", true, Role::Admin)
                    .with_email("bob@example.com")
                    .with_age(25),
                InsertComplex::new("Charlie", true, Role::User)
                    .with_email("charlie@example.com")
                    .with_age(35),
            ])
            => execute
    );

    #[allow(dead_code)]
    #[derive(Debug, PostgresFromRow)]
    struct PgDistinctRoleResult {
        role: Role,
    }
    let results: Vec<PgDistinctRoleResult> = drizzle_exec!(
        db.select(alias(distinct(complex.role), "role"))
            .from(complex)
            => all
    );
    assert_eq!(results.len(), 2);
});

postgres_test!(select_with_alias, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple)
            .values(vec![InsertSimple::new("test")])
            => execute
    );

    let stmt = db.select(alias(simple.name, "user_name")).from(simple);

    let result: PgAliasResult = drizzle_exec!(stmt => get);
    assert_eq!(result.user_name, "test");
});

#[cfg(feature = "uuid")]
postgres_test!(select_with_coalesce, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values(vec![
                InsertComplex::new("Alice", true, Role::User).with_age(30),
            ])
            => execute
    );

    let stmt = db
        .select(alias(
            coalesce(complex.email, "unknown@example.com"),
            "email",
        ))
        .from(complex);

    let result: PgCoalesceResult = drizzle_exec!(stmt => get);
    assert_eq!(result.email, "unknown@example.com");
});
