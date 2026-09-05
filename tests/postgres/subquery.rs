//! PostgreSQL-specific subquery behavior (CTE-backed subqueries). Portable
//! subquery cases live in `crate::common::subquery`.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::SimpleSchema;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;

#[drizzle::test]
fn test_with_subquery_parenthesization(db: &mut TestDb<SimpleSchema>) {
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
        cmp_sql.contains(r#""simple"."id" >(WITH "filtered_ids" AS"#),
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
        in_sql.contains(r#""simple"."id" IN (WITH "filtered_ids" AS"#),
        "sql: {in_sql}"
    );

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);
    let func_sql = db.select(avg(with_subquery)).from(simple).to_sql().sql();
    assert!(
        func_sql.contains(r#"AVG ((WITH "filtered_ids" AS"#),
        "sql: {func_sql}"
    );
}
