//! Tuple conjunctions and the `all` / `any` combinators.

#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use crate::common::schema::sqlite::{
    FullBlogSchema, InsertCategory, InsertPost, InsertSimple, SelectSimple, SimpleSchema,
};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[drizzle::test]
fn test_tuple_renders_flat_conjunction(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where((gt(simple.id, 1), lt(simple.id, 3), neq(simple.name, "zed")))
        .to_sql();

    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE ("simple"."id" > ? AND "simple"."id" < ? AND "simple"."name" <> ?)"#
    );
}

#[drizzle::test]
fn test_tuple_where_matches_chained_and(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((gt(simple.id, 1), lt(simple.id, 3)))
        .all();

    assert_eq!(1, results.len());
    assert_eq!("bob", results[0].name);
}

#[drizzle::test]
fn test_all_and_any_render_flat(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let conjunction = db
        .select(())
        .from(simple)
        .r#where(all((gt(simple.id, 1), lt(simple.id, 3))))
        .to_sql();
    assert_eq!(
        conjunction.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE ("simple"."id" > ? AND "simple"."id" < ?)"#
    );

    let disjunction = db
        .select(())
        .from(simple)
        .r#where(any((
            eq(simple.name, "alice"),
            eq(simple.name, "bob"),
            eq(simple.name, "carol"),
        )))
        .to_sql();
    assert_eq!(
        disjunction.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE ("simple"."name" = ? OR "simple"."name" = ? OR "simple"."name" = ?)"#
    );
}

#[drizzle::test]
fn test_any_matches_each_alternative(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(any((eq(simple.name, "alice"), eq(simple.name, "carol"))))
        .all();

    assert_eq!(2, results.len());
}

#[drizzle::test]
fn test_tuples_nest_inside_or(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where(or(
            (gt(simple.id, 1), lt(simple.id, 3)),
            (eq(simple.name, "alice"), gte(simple.id, 1)),
        ))
        .to_sql();

    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE (("simple"."id" > ? AND "simple"."id" < ?) OR ("simple"."name" = ? AND "simple"."id" >= ?))"#
    );
}

#[drizzle::test]
fn test_nested_tuples_group(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where(((gt(simple.id, 1), lt(simple.id, 3)), eq(simple.name, "bob")))
        .to_sql();

    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE (("simple"."id" > ? AND "simple"."id" < ?) AND "simple"."name" = ?)"#
    );
}

#[drizzle::test]
fn test_option_elements_are_skipped(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let present = db
        .select(())
        .from(simple)
        .r#where((gt(simple.id, 1), Some(eq(simple.name, "bob"))))
        .to_sql();
    assert_eq!(
        present.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE ("simple"."id" > ? AND "simple"."name" = ?)"#
    );

    let absent = db
        .select(())
        .from(simple)
        .r#where((
            gt(simple.id, 1),
            None::<SQLExpr<'_, SQLiteValue<'_>, drizzle::sqlite::types::Integer, NonNull, Scalar>>,
        ))
        .to_sql();
    assert_eq!(
        absent.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE ("simple"."id" > ?)"#
    );
}

#[drizzle::test]
fn test_optional_filter_round_trip(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    let unfiltered: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((
            gte(simple.id, 1),
            None::<_>.map(|n: &str| eq(simple.name, n)),
        ))
        .all();
    assert_eq!(3, unfiltered.len());

    let filtered: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((gte(simple.id, 1), Some("bob").map(|n| eq(simple.name, n))))
        .all();
    assert_eq!(1, filtered.len());
}

#[drizzle::test]
fn test_all_none_conjunction_is_true(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    type Cond<'a> = SQLExpr<'a, SQLiteValue<'a>, drizzle::sqlite::types::Integer, NonNull, Scalar>;

    let sql = db
        .select(())
        .from(simple)
        .r#where((None::<Cond<'_>>, None::<Cond<'_>>))
        .to_sql();
    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE TRUE"#
    );

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((None::<Cond<'_>>, None::<Cond<'_>>))
        .all();
    assert_eq!(3, results.len());
}

#[drizzle::test]
fn test_all_none_disjunction_is_false(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    type Cond<'a> = SQLExpr<'a, SQLiteValue<'a>, drizzle::sqlite::types::Integer, NonNull, Scalar>;

    let sql = db
        .select(())
        .from(simple)
        .r#where(any((None::<Cond<'_>>, None::<Cond<'_>>)))
        .to_sql();
    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE FALSE"#
    );

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(any((None::<Cond<'_>>, None::<Cond<'_>>)))
        .all();
    assert!(results.is_empty());
}

#[drizzle::test]
fn test_tuple_in_having(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    let sql = db
        .select((simple.name, count(simple.id)))
        .from(simple)
        .group_by(simple.name)
        .having((gt(count(simple.id), 0i64), lt(count(simple.id), 10i64)))
        .to_sql();

    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."name", COUNT ("simple"."id") FROM "simple" GROUP BY "simple"."name" HAVING (COUNT ("simple"."id")> ? AND COUNT ("simple"."id")< ?)"#
    );
}

// A join's ON condition is bounded on `ToSQL`, not `Expr`, so a bare tuple
// there would render as a column list. `all(...)` is the spelling that works.
#[drizzle::test]
fn test_all_in_join_on(db: &mut TestDb<FullBlogSchema>) {
    let FullBlogSchema { post, category, .. } = schema;

    db.insert(category)
        .values([InsertCategory::new("rust").with_id(1)])
        .execute();
    db.insert(post)
        .values([InsertPost::new("hello", false)])
        .execute();

    let sql = db
        .select(post.title)
        .from(post)
        .inner_join((
            category,
            all((eq(category.name, "rust"), neq(post.title, "zed"))),
        ))
        .to_sql();

    assert!(
        sql.sql()
            .contains(r#"ON ("categories"."name" = ? AND "posts"."title" <> ?)"#),
        "unexpected SQL: {}",
        sql.sql()
    );

    let titles: Vec<String> = db
        .select(post.title)
        .from(post)
        .inner_join((
            category,
            all((eq(category.name, "rust"), neq(post.title, "zed"))),
        ))
        .all();
    assert_eq!(vec!["hello".to_owned()], titles);
}

#[drizzle::test]
fn test_tuple_under_not(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(not((gt(simple.id, 1), lt(simple.id, 3))))
        .all();

    assert_eq!(2, results.len());
}

#[drizzle::test]
fn test_eight_element_tuple(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    // Three rows: ids 1..=3 named alice, bob, carol.
    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where((
            gte(simple.id, 1),
            gte(simple.id, 1),
            gte(simple.id, 1),
            gte(simple.id, 1),
            gte(simple.id, 1),
            gte(simple.id, 1),
            gte(simple.id, 1),
            eq(simple.name, "bob"),
        ))
        .all();

    assert_eq!(1, results.len());
}
