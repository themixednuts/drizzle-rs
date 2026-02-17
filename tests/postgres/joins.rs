//! PostgreSQL JOIN tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

#[cfg(feature = "uuid")]
use crate::common::schema::postgres::{
    Category, Complex, ComplexPostSchema, FullBlogSchema, InsertCategory, InsertComplex,
    InsertPost, InsertPostCategory, Post, Role,
};
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[cfg(feature = "uuid")]
use std::array;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow, Default)]
struct AuthorPostResult {
    #[column(Complex::name)]
    author_name: String,
    #[column(Post::title)]
    post_title: String,
    #[column(Post::content)]
    post_content: Option<String>,
}

#[cfg(feature = "uuid")]
postgres_test!(auto_fk_join, ComplexPostSchema, {
    let ComplexPostSchema { complex, post, .. } = schema;

    let [id1, id2, id3]: [Uuid; 3] = array::from_fn(|_| Uuid::new_v4());

    let authors = vec![
        InsertComplex::new("alice", true, Role::User)
            .with_id(id1)
            .with_email("alice@example.com"),
        InsertComplex::new("bob", true, Role::User)
            .with_id(id2)
            .with_email("bob@example.com"),
        InsertComplex::new("charlie", true, Role::User)
            .with_id(id3)
            .with_email("charlie@example.com"),
    ];

    drizzle_exec!(db.insert(complex).values(authors) => execute);

    let posts = vec![
        InsertPost::new("Alice's First Post", true)
            .with_content("Content by Alice")
            .with_author_id(id1),
        InsertPost::new("Bob's Adventure", true)
            .with_content("Travel blog by Bob")
            .with_author_id(id2),
        InsertPost::new("Alice's Second Post", true)
            .with_content("More content by Alice")
            .with_author_id(id1),
    ];

    drizzle_exec!(db.insert(post).values(posts) => execute);

    let join_results: Vec<AuthorPostResult> = drizzle_exec!(
        db.select(AuthorPostResult::default())
            .from(complex)
            .join(post)
            .order_by([asc(complex.name), asc(post.title)])
            => all_as
    );

    assert_eq!(join_results.len(), 3);

    assert_eq!(join_results[0].author_name, "alice");
    assert_eq!(join_results[0].post_title, "Alice's First Post");
    assert_eq!(
        join_results[0].post_content,
        Some("Content by Alice".to_string())
    );

    assert_eq!(join_results[1].author_name, "alice");
    assert_eq!(join_results[1].post_title, "Alice's Second Post");

    assert_eq!(join_results[2].author_name, "bob");
    assert_eq!(join_results[2].post_title, "Bob's Adventure");

    let filtered_results: Vec<AuthorPostResult> = drizzle_exec!(
        db.select(AuthorPostResult::default())
            .from(complex)
            .join(post)
            .r#where(eq(complex.name, "alice"))
            => all_as
    );

    assert_eq!(filtered_results.len(), 2);
    filtered_results.iter().for_each(|r| {
        assert_eq!(r.author_name, "alice");
    });
});

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow, Default)]
struct PostCategoryResult {
    #[column(Post::title)]
    post_title: String,
    #[column(Category::name)]
    category_name: String,
}

#[cfg(feature = "uuid")]
postgres_test!(chained_fk_join, FullBlogSchema, {
    let FullBlogSchema {
        post,
        category,
        post_category,
        ..
    } = schema;

    let [post_id1, post_id2]: [Uuid; 2] = array::from_fn(|_| Uuid::new_v4());

    let posts = vec![
        InsertPost::new("Rust Guide", true)
            .with_id(post_id1)
            .with_content("Learn Rust"),
        InsertPost::new("Go Guide", true)
            .with_id(post_id2)
            .with_content("Learn Go"),
    ];
    drizzle_exec!(db.insert(post).values(posts) => execute);

    let categories = vec![
        InsertCategory::new("Programming"),
        InsertCategory::new("Tutorial"),
    ];
    drizzle_exec!(db.insert(category).values(categories) => execute);

    let links = vec![
        InsertPostCategory::new(post_id1, 1),
        InsertPostCategory::new(post_id1, 2),
        InsertPostCategory::new(post_id2, 1),
    ];
    drizzle_exec!(db.insert(post_category).values(links) => execute);

    // Chained auto-FK: post -> post_category (forward FK) -> category (reverse FK)
    let results: Vec<PostCategoryResult> = drizzle_exec!(
        db.select(PostCategoryResult::default())
            .from(post)
            .join(post_category)
            .join(category)
            .order_by([asc(post.title), asc(category.name)])
            => all_as
    );

    // Go Guide -> Programming = 1 row
    // Rust Guide -> Programming, Tutorial = 2 rows
    // Total = 3
    assert_eq!(results.len(), 3);

    assert_eq!(results[0].post_title, "Go Guide");
    assert_eq!(results[0].category_name, "Programming");

    assert_eq!(results[1].post_title, "Rust Guide");
    assert_eq!(results[1].category_name, "Programming");

    assert_eq!(results[2].post_title, "Rust Guide");
    assert_eq!(results[2].category_name, "Tutorial");
});
