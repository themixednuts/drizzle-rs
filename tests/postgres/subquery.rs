//! PostgreSQL subquery tests.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::{InsertSimple, SimpleSchema};
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSubqueryResult {
    id: i32,
    name: String,
}

postgres_test!(test_typed_scalar_subquery, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema {
        simple: subquery_simple,
    } = SimpleSchema::new();

    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let min_id = builder
        .select(min(subquery_simple.id))
        .from(subquery_simple);
    let results: Vec<PgSubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(gt(simple.id, min_id))
            => all
    );

    assert_eq!(2, results.len());
    assert!(results.iter().any(|r| r.name == "bob"));
    assert!(results.iter().any(|r| r.name == "charlie"));
});

postgres_test!(test_typed_in_subquery_single_column, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema {
        simple: subquery_simple,
    } = SimpleSchema::new();

    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];
    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    let only_bob_id = builder
        .select(subquery_simple.id)
        .from(subquery_simple)
        .r#where(eq(subquery_simple.name, "bob"));

    let results: Vec<PgSubqueryResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(in_subquery(simple.id, only_bob_id))
            => all
    );

    assert_eq!(1, results.len());
    assert_eq!("bob", results[0].name);
});

postgres_test!(
    test_typed_in_subquery_multi_column_row_value,
    SimpleSchema,
    {
        let SimpleSchema { simple } = schema;
        let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
        let SimpleSchema {
            simple: subquery_simple,
        } = SimpleSchema::new();

        let test_data = vec![
            InsertSimple::new("alice"),
            InsertSimple::new("bob"),
            InsertSimple::new("charlie"),
        ];
        drizzle_exec!(db.insert(simple).values(test_data) => execute);

        let bob_row = builder
            .select((subquery_simple.id, subquery_simple.name))
            .from(subquery_simple)
            .r#where(eq(subquery_simple.name, "bob"));

        let results: Vec<PgSubqueryResult> = drizzle_exec!(
            db.select((simple.id, simple.name))
                .from(simple)
                .r#where(in_subquery((simple.id, simple.name), bob_row))
                => all
        );

        assert_eq!(1, results.len());
        assert_eq!("bob", results[0].name);
    }
);

postgres_test!(test_with_subquery_parenthesization, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema {
        simple: subquery_simple,
    } = SimpleSchema::new();

    struct FilteredIdsTag;
    impl drizzle::core::Tag for FilteredIdsTag {
        const NAME: &'static str = "filtered_ids";
    }

    let filtered_ids = builder
        .select(subquery_simple.id)
        .from(subquery_simple)
        .r#where(gt(subquery_simple.id, 1))
        .into_cte::<FilteredIdsTag>();

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);
    let cmp_sql = db
        .select(simple.id)
        .from(simple)
        .r#where(gt(simple.id, with_subquery))
        .to_sql()
        .sql();
    assert!(
        cmp_sql.contains(r#""simple"."id" >(WITH filtered_ids AS"#),
        "sql: {cmp_sql}"
    );

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);
    let in_sql = db
        .select(simple.id)
        .from(simple)
        .r#where(in_subquery(simple.id, with_subquery))
        .to_sql()
        .sql();
    assert!(
        in_sql.contains(r#""simple"."id" IN (WITH filtered_ids AS"#),
        "sql: {in_sql}"
    );

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);
    let func_sql = db.select(avg(with_subquery)).from(simple).to_sql().sql();
    assert!(
        func_sql.contains(r#"AVG ((WITH filtered_ids AS"#),
        "sql: {func_sql}"
    );
});
