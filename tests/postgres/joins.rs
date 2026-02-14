//! PostgreSQL JOIN tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

#[cfg(feature = "uuid")]
use crate::common::schema::postgres::{
    Complex, ComplexPostSchema, InsertComplex, InsertPost, Post, Role,
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

    drizzle_exec!(db.insert(complex).values(authors).execute());

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

    drizzle_exec!(db.insert(post).values(posts).execute());

    let join_results: Vec<AuthorPostResult> = drizzle_exec!(
        db.select(AuthorPostResult::default())
            .from(complex)
            .join(post)
            .order_by([OrderBy::asc(complex.name), OrderBy::asc(post.title)])
            .all()
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
            .all()
    );

    assert_eq!(filtered_results.len(), 2);
    filtered_results.iter().for_each(|r| {
        assert_eq!(r.author_name, "alice");
    });
});
