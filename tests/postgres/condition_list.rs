//! Tuple conjunctions and the `all` / `any` combinators on PostgreSQL.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_postgres::values::PostgresValue;

#[drizzle::test]
fn condition_tuple_renders_flat_conjunction(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where((gt(simple.id, 1), lt(simple.id, 3), neq(simple.name, "zed")))
        .to_sql()
        .sql();

    assert!(
        sql.contains(
            r#"WHERE ("simple"."id" > $1 AND "simple"."id" < $2 AND "simple"."name" <> $3)"#
        ),
        "unexpected SQL: {sql}"
    );
}

#[drizzle::test]
fn condition_tuple_where_matches_chained_and(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([
            InsertSimple::new("alice"),
            InsertSimple::new("bob"),
            InsertSimple::new("carol"),
        ])
        .execute();

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((neq(simple.name, "alice"), neq(simple.name, "carol")))
        .all();

    assert_eq!(1, results.len());
    assert_eq!("bob", results[0].name);
}

#[drizzle::test]
fn condition_any_renders_flat_disjunction(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([
            InsertSimple::new("alice"),
            InsertSimple::new("bob"),
            InsertSimple::new("carol"),
        ])
        .execute();

    let sql = db
        .select(())
        .from(simple)
        .r#where(any((eq(simple.name, "alice"), eq(simple.name, "carol"))))
        .to_sql()
        .sql();
    assert!(
        sql.contains(r#"WHERE ("simple"."name" = $1 OR "simple"."name" = $2)"#),
        "unexpected SQL: {sql}"
    );

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(any((eq(simple.name, "alice"), eq(simple.name, "carol"))))
        .all();
    assert_eq!(2, results.len());
}

#[drizzle::test]
fn condition_tuples_nest_inside_or(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where(or(
            (gt(simple.id, 1), lt(simple.id, 3)),
            (eq(simple.name, "alice"), gte(simple.id, 1)),
        ))
        .to_sql()
        .sql();

    assert!(
        sql.contains(
            r#"WHERE (("simple"."id" > $1 AND "simple"."id" < $2) OR ("simple"."name" = $3 AND "simple"."id" >= $4))"#
        ),
        "unexpected SQL: {sql}"
    );
}

#[drizzle::test]
fn condition_boolean_column_tuple(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    // Two `boolean` columns in one tuple: the conjunction, not a row value.
    let sql = db
        .select(complex.name)
        .from(complex)
        .r#where((complex.active, complex.active))
        .to_sql()
        .sql();

    assert!(
        sql.contains(r#"WHERE ("complex"."active" AND "complex"."active")"#),
        "unexpected SQL: {sql}"
    );
}

#[drizzle::test]
fn condition_option_elements_are_skipped(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("alice"), InsertSimple::new("bob")])
        .execute();

    let unfiltered: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((
            gte(simple.id, 1),
            None::<_>.map(|n: &str| eq(simple.name, n)),
        ))
        .all();
    assert_eq!(2, unfiltered.len());

    let filtered: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((gte(simple.id, 1), Some("bob").map(|n| eq(simple.name, n))))
        .all();
    assert_eq!(1, filtered.len());
}

type PgCond<'a> =
    SQLExpr<'a, PostgresValue<'a>, drizzle::postgres::types::Boolean, NonNull, Scalar>;

#[drizzle::test]
fn condition_all_none_conjunction_is_true(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("alice"), InsertSimple::new("bob")])
        .execute();

    let sql = db
        .select(())
        .from(simple)
        .r#where((None::<PgCond<'_>>, None::<PgCond<'_>>))
        .to_sql()
        .sql();
    assert!(sql.contains("WHERE TRUE"), "unexpected SQL: {sql}");

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((None::<PgCond<'_>>, None::<PgCond<'_>>))
        .all();
    assert_eq!(2, results.len());
}

#[drizzle::test]
fn condition_all_none_disjunction_is_false(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("alice")])
        .execute();

    let sql = db
        .select(())
        .from(simple)
        .r#where(any((None::<PgCond<'_>>, None::<PgCond<'_>>)))
        .to_sql()
        .sql();
    assert!(sql.contains("WHERE FALSE"), "unexpected SQL: {sql}");

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(any((None::<PgCond<'_>>, None::<PgCond<'_>>)))
        .all();
    assert!(results.is_empty());
}

#[drizzle::test]
fn condition_tuple_in_having(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([InsertSimple::new("alice"), InsertSimple::new("alice")])
        .execute();

    let sql = db
        .select((simple.name, count(simple.id)))
        .from(simple)
        .group_by(simple.name)
        .having((gt(count(simple.id), 0i64), lt(count(simple.id), 10i64)))
        .to_sql()
        .sql();

    assert!(sql.contains("HAVING (COUNT"), "unexpected SQL: {sql}");
    assert!(sql.contains("AND COUNT"), "unexpected SQL: {sql}");
}

// A join's ON condition is bounded on `ToSQL`, not `Expr`, so a bare tuple
// there would render as a column list. `all(...)` is the spelling that works.
#[drizzle::test]
fn condition_all_in_join_on(db: &mut TestDb<PostCategorySchema>) {
    let PostCategorySchema { post, category, .. } = schema;

    let sql = db
        .select(post.title)
        .from(post)
        .inner_join((
            category,
            all((eq(category.name, "rust"), neq(post.title, "zed"))),
        ))
        .to_sql()
        .sql();

    assert!(
        sql.contains(r#"."name" = $1 AND "#) && sql.contains(r#"."title" <> $2)"#),
        "unexpected SQL: {sql}"
    );
    assert!(sql.contains("ON ("), "unexpected SQL: {sql}");
}
