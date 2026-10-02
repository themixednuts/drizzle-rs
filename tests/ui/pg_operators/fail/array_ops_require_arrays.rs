use drizzle::postgres::expr::{PgArray, array_contains, array_overlaps};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Post {
    #[column(primary)]
    id: i32,
    title: String,
    tags: Vec<String>,
    scores: Vec<i32>,
}

fn main() {
    let post = Post::default();
    // `text @> ...`: the left operand is not an array.
    let _ = array_contains(post.title, PgArray(vec!["rust"]));
    // `text[] @> text`: the right operand must be an array too.
    let _ = array_contains(post.tags, "rust");
    // `text[] && int4[]`: element types differ.
    let _ = array_overlaps(post.tags, post.scores);
}
