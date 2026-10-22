//! PostgreSQL array operator tests
//!
//! Tests for PostgreSQL-specific array operators (@>, <@, &&), executed
//! against `text[]` and `int4[]` columns.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use drizzle::core::expr::count;
use drizzle::postgres::expr::{PgArray, array_contained, array_contains, array_overlaps};
use drizzle::postgres::prelude::*;

#[PostgresTable(NAME = "pg_array_ops_posts")]
struct ArrayPost {
    #[column(serial, primary)]
    id: i32,
    tags: Vec<String>,
    scores: Vec<i32>,
}

#[derive(PostgresSchema)]
struct ArrayPostSchema {
    posts: ArrayPost,
}

macro_rules! seed {
    ($db:expr, $posts:expr) => {
        $db.insert($posts).values([
            InsertArrayPost::new(vec!["rust".to_string(), "sql".to_string()], vec![1, 2, 3]),
            InsertArrayPost::new(vec!["python".to_string()], vec![4]),
        ])
    };
}

#[drizzle::test]
fn array_contains_matches_rows(db: &mut TestDb<ArrayPostSchema>) {
    let ArrayPostSchema { posts } = schema;
    seed!(db, posts).execute();

    let stmt = db
        .select(count(posts.id))
        .from(posts)
        .r#where(array_contains(posts.tags, PgArray(vec!["rust"])));
    assert!(stmt.to_sql().sql().contains("@>"));
    let n: i64 = stmt.get();
    assert_eq!(n, 1);
}

#[drizzle::test]
fn array_contained_matches_rows(db: &mut TestDb<ArrayPostSchema>) {
    let ArrayPostSchema { posts } = schema;
    seed!(db, posts).execute();

    let stmt = db
        .select(count(posts.id))
        .from(posts)
        .r#where(array_contained(posts.scores, PgArray(vec![1, 2, 3, 4])));
    assert!(stmt.to_sql().sql().contains("<@"));
    let n: i64 = stmt.get();
    assert_eq!(n, 2);
}

#[drizzle::test]
fn array_overlaps_matches_rows(db: &mut TestDb<ArrayPostSchema>) {
    let ArrayPostSchema { posts } = schema;
    seed!(db, posts).execute();

    let stmt = db
        .select(count(posts.id))
        .from(posts)
        .r#where(array_overlaps(posts.tags, PgArray(vec!["python", "go"])));
    assert!(stmt.to_sql().sql().contains("&&"));
    let n: i64 = stmt.get();
    assert_eq!(n, 1);
}

// Method syntax via the ArrayExprExt trait, and column-to-column operands.
#[drizzle::test]
fn array_ops_method_syntax(db: &mut TestDb<ArrayPostSchema>) {
    use drizzle::postgres::expr::ArrayExprExt;

    let ArrayPostSchema { posts } = schema;
    seed!(db, posts).execute();

    let n: i64 = db
        .select(count(posts.id))
        .from(posts)
        .r#where(posts.tags.array_contains(PgArray(vec!["sql"])))
        .get();
    assert_eq!(n, 1);

    let n: i64 = db
        .select(count(posts.id))
        .from(posts)
        .r#where(posts.scores.array_contained(PgArray(vec![4, 5])))
        .get();
    assert_eq!(n, 1);

    let n: i64 = db
        .select(count(posts.id))
        .from(posts)
        .r#where(posts.tags.array_overlaps(posts.tags))
        .get();
    assert_eq!(n, 2);
}
