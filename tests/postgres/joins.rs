//! PostgreSQL JOIN tests

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;
use drizzle_core::OrderBy;

#[test]
fn test_inner_join_sql_generation() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .inner_join(post, eq(complex.id, post.author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Inner join SQL: {}", sql_string);

    assert!(sql_string.contains("INNER JOIN"));
    assert!(sql_string.contains(r#""pg_complex""#));
    assert!(sql_string.contains(r#""pg_posts""#));
}

#[test]
fn test_left_join_sql_generation() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .left_join(post, eq(complex.id, post.author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Left join SQL: {}", sql_string);

    assert!(sql_string.contains("LEFT JOIN"));
}

#[test]
fn test_right_join_sql_generation() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .right_join(post, eq(complex.id, post.author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Right join SQL: {}", sql_string);

    assert!(sql_string.contains("RIGHT JOIN"));
}

#[test]
fn test_full_join_sql_generation() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .full_join(post, eq(complex.id, post.author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Full join SQL: {}", sql_string);

    assert!(sql_string.contains("FULL JOIN"));
}

#[test]
fn test_join_with_where_clause() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .inner_join(post, eq(complex.id, post.author_id))
        .r#where(eq(post.published, true));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Join with WHERE SQL: {}", sql_string);

    assert!(sql_string.contains("JOIN"));
    assert!(sql_string.contains("WHERE"));
}

#[test]
fn test_join_with_complex_on_condition() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .inner_join(
            post,
            and([eq(complex.id, post.author_id), eq(post.published, true)]),
        );

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Join with complex ON condition SQL: {}", sql_string);

    assert!(sql_string.contains("JOIN"));
    assert!(sql_string.contains("AND"));
}

#[test]
fn test_multiple_joins() {
    let PgFullBlogSchema {
        post,
        category,
        post_category,
        ..
    } = PgFullBlogSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((post.title, category.name))
        .from(post)
        .inner_join(post_category, eq(post.id, post_category.post_id))
        .inner_join(category, eq(post_category.category_id, category.id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Multiple joins SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_posts""#));
    assert!(sql_string.contains(r#""pg_post_categories""#));
    assert!(sql_string.contains(r#""pg_categories""#));
}

#[test]
fn test_join_select_all_columns() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select(())
        .from(complex)
        .inner_join(post, eq(complex.id, post.author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Join select all SQL: {}", sql_string);

    assert!(sql_string.contains("SELECT"));
    assert!(sql_string.contains("JOIN"));
}

#[test]
fn test_join_with_order_by() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .inner_join(post, eq(complex.id, post.author_id))
        .order_by([OrderBy::asc(complex.name), OrderBy::asc(post.title)]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Join with ORDER BY SQL: {}", sql_string);

    assert!(sql_string.contains("JOIN"));
    assert!(sql_string.contains("ORDER BY"));
}

#[test]
fn test_join_with_aggregation() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, alias(count(post.id), "post_count")))
        .from(complex)
        .inner_join(post, eq(complex.id, post.author_id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Join with aggregation SQL: {}", sql_string);

    assert!(sql_string.contains("JOIN"));
    assert!(sql_string.contains("COUNT"));
}

#[test]
fn test_left_join_with_null_check() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .left_join(post, eq(complex.id, post.author_id))
        .r#where(is_null(post.id));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Left join with NULL check SQL: {}", sql_string);

    assert!(sql_string.contains("LEFT JOIN"));
    assert!(sql_string.contains("IS NULL"));
}

#[test]
fn test_join_with_enum_filter() {
    let PgComplexPostSchema { complex, post, .. } = PgComplexPostSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((complex.name, post.title))
        .from(complex)
        .inner_join(post, eq(complex.id, post.author_id))
        .r#where(eq(complex.role, PgRole::Admin));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Join with enum filter SQL: {}", sql_string);

    assert!(sql_string.contains("JOIN"));
    assert!(sql_string.contains(r#""pg_complex"."role""#));
}

#[test]
fn test_self_join_scenario() {
    let PgSimpleSchema { simple } = PgSimpleSchema::new();

    // Self-join scenario - select from same table with different alias
    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(gt(simple.id, 0));

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Simple query (self-join scenario base) SQL: {}", sql_string);

    assert!(sql_string.contains(r#""pg_simple""#));
}

#[test]
fn test_many_to_many_join() {
    let PgFullBlogSchema {
        post,
        category,
        post_category,
        ..
    } = PgFullBlogSchema::new();

    let query = drizzle::postgres::QueryBuilder::new::<()>()
        .select((post.title, category.name))
        .from(post)
        .join(post_category, eq(post.id, post_category.post_id))
        .join(category, eq(post_category.category_id, category.id))
        .order_by([OrderBy::asc(post.title), OrderBy::asc(category.name)]);

    let sql = query.to_sql();
    let sql_string = sql.sql();

    println!("Many-to-many join SQL: {}", sql_string);

    assert!(sql_string.contains("JOIN"));
    assert!(sql_string.contains(r#""pg_posts""#));
    assert!(sql_string.contains(r#""pg_categories""#));
}
