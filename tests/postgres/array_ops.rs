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

/// Arrays whose elements may be NULL: `Vec<Option<T>>` is a nullable-element
/// native array, and `None` elements are SQL NULL elements.
#[PostgresTable(NAME = "pg_nullable_element_arrays")]
struct NullableElementArray {
    #[column(serial, primary)]
    id: i32,
    scores: Vec<Option<i32>>,
    tags: Option<Vec<Option<String>>>,
}

#[derive(PostgresSchema)]
struct NullableElementArraySchema {
    arrays: NullableElementArray,
}

#[drizzle::test]
fn nullable_element_arrays_round_trip(db: &mut TestDb<NullableElementArraySchema>) {
    let NullableElementArraySchema { arrays } = schema;
    db.insert(arrays)
        .values([
            InsertNullableElementArray::new(vec![Some(1), None, Some(3)])
                .with_tags(vec![Some("a".to_string()), None]),
        ])
        .execute();

    let row: SelectNullableElementArray = db.select(()).from(arrays).get().expect("a row");
    assert_eq!(row.scores, vec![Some(1), None, Some(3)]);
    assert_eq!(row.tags, Some(vec![Some("a".to_string()), None]));

    // NULL elements are SQL NULL, not a placeholder value.
    let nulls: Vec<NullElements> = result!(db.all(SQL::raw(
        r#"SELECT "scores"[2] IS NULL, "tags"[2] IS NULL FROM "pg_nullable_element_arrays""#
    )))
    .unwrap();
    assert_eq!(nulls.len(), 1);
    assert_eq!((nulls[0].0, nulls[0].1), (true, true));

    db.update(arrays)
        .set(UpdateNullableElementArray::default().with_scores(vec![None]))
        .r#where(true)
        .execute();
    let row: SelectNullableElementArray = db.select(()).from(arrays).get().expect("a row");
    assert_eq!(row.scores, vec![None]);
    assert_eq!(row.tags, Some(vec![Some("a".to_string()), None]));
}

#[derive(Debug, PostgresFromRow)]
struct NullElements(bool, bool);

/// A `json`/`jsonb` column of `Vec<Option<T>>` is one JSON document: `None`
/// elements are JSON `null`, and a `None` column is SQL NULL.
#[cfg(feature = "serde")]
#[PostgresTable(NAME = "pg_nullable_element_json_arrays")]
struct NullableElementJsonArray {
    #[column(serial, primary)]
    id: i32,
    #[column(json)]
    scores: Vec<Option<i32>>,
    #[column(jsonb)]
    tags: Option<Vec<Option<String>>>,
}

#[cfg(feature = "serde")]
#[derive(PostgresSchema)]
struct NullableElementJsonArraySchema {
    arrays: NullableElementJsonArray,
}

#[cfg(feature = "serde")]
#[derive(Debug, PostgresFromRow)]
struct JsonArrayShape(String, bool);

#[cfg(feature = "serde")]
#[drizzle::test]
fn nullable_element_json_arrays_round_trip(db: &mut TestDb<NullableElementJsonArraySchema>) {
    let NullableElementJsonArraySchema { arrays } = schema;
    db.insert(arrays)
        .values([InsertNullableElementJsonArray::new(vec![Some(1), None])])
        .execute();

    let row: SelectNullableElementJsonArray = db.select(()).from(arrays).get().expect("a row");
    assert_eq!(row.scores, vec![Some(1), None]);
    assert_eq!(row.tags, None);

    let shape: Vec<JsonArrayShape> = result!(db.all(SQL::raw(
        r#"SELECT "scores"::text, "tags" IS NULL FROM "pg_nullable_element_json_arrays""#
    )))
    .unwrap();
    assert_eq!(shape.len(), 1);
    assert_eq!((shape[0].0.as_str(), shape[0].1), ("[1,null]", true));
}
