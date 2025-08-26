#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use common::{Category, InsertCategory, InsertPost, InsertPostCategory, Post};
#[cfg(feature = "uuid")]
use common::{Complex, InsertComplex};
use drizzle_macros::drizzle_test;
use drizzle::prelude::*;

use std::array;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(not(feature = "uuid"))]
use crate::common::FullBlogSchema;
#[cfg(feature = "uuid")]
use crate::common::{ComplexPostSchema, FullBlogSchema};

mod common;

#[cfg(feature = "uuid")]
#[derive(Debug, FromRow, Default)]
struct AuthorPostResult {
    #[column(Complex::name)]
    author_name: String,
    #[column(Post::title)]
    post_title: String,
    #[column(Post::content)]
    post_content: Option<String>,
}

#[derive(Debug, FromRow, Default)]
struct PostCategoryResult {
    #[column(Post::title)]
    post_title: String,
    #[column(Category::name)]
    category_name: String,
    #[column(Category::description)]
    category_description: Option<String>,
}

#[cfg(feature = "uuid")]
drizzle_test!(simple_inner_join, ComplexPostSchema, {
    let ComplexPostSchema { complex, post } = schema;

    #[cfg(not(feature = "uuid"))]
    let (id1, id2, id3) = (1, 2, 3);
    #[cfg(feature = "uuid")]
    let [id1, id2, id3]: [Uuid; 3] = array::from_fn(|_| Uuid::new_v4());

    #[cfg(feature = "uuid")]
    let authors = vec![
        InsertComplex::new("alice", true, common::Role::User)
            .with_id(id1)
            .with_email("alice@example.com"),
        InsertComplex::new("bob", true, common::Role::User)
            .with_id(id2)
            .with_email("bob@example.com"),
        InsertComplex::new("charlie", true, common::Role::User)
            .with_id(id3)
            .with_email("charlie@example.com"),
    ];

    let author_result = drizzle_exec!(db.insert(complex).values(authors).execute());
    assert_eq!(author_result, 3);

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

    let post_result = drizzle_exec!(db.insert(post).values(posts).execute());
    assert_eq!(post_result, 3);

    // Test inner join: only authors with posts should appear
    let join_results: Vec<AuthorPostResult> = drizzle_exec!(
        db.select(AuthorPostResult::default())
            .from(complex)
            .inner_join(post, eq(complex.id, post.author_id))
            .order_by([OrderBy::asc(complex.name), OrderBy::asc(post.title)])
            .all()
    );

    dbg!(&join_results);

    // Should have 3 results (Alice: 2 posts, Bob: 1 post) - Charlie excluded because no posts
    assert_eq!(join_results.len(), 3);

    // Verify Alice's posts
    assert_eq!(join_results[0].author_name, "alice");
    assert_eq!(join_results[0].post_title, "Alice's First Post");
    assert_eq!(
        join_results[0].post_content,
        Some("Content by Alice".to_string())
    );

    assert_eq!(join_results[1].author_name, "alice");
    assert_eq!(join_results[1].post_title, "Alice's Second Post");
    assert_eq!(
        join_results[1].post_content,
        Some("More content by Alice".to_string())
    );

    // Verify Bob's post
    assert_eq!(join_results[2].author_name, "bob");
    assert_eq!(join_results[2].post_title, "Bob's Adventure");
    assert_eq!(
        join_results[2].post_content,
        Some("Travel blog by Bob".to_string())
    );

    // Verify that we can filter join results
    let alice_posts: Vec<AuthorPostResult> = drizzle_exec!(
        db.select(AuthorPostResult::default())
            .from(complex)
            .inner_join(post, eq(complex.id, post.author_id))
            .r#where(eq(complex.name, "alice"))
            .all()
    );

    assert_eq!(alice_posts.len(), 2);
    alice_posts.iter().for_each(|result| {
        assert_eq!(result.author_name, "alice");
    });
});

drizzle_test!(many_to_many_join, FullBlogSchema, {
    let FullBlogSchema {
        category,
        post,
        post_category,
        ..
    } = schema;

    // Insert test data: posts and categories with many-to-many relationship
    let posts = vec![
        InsertPost::new("Tech Tutorial", true).with_content("Learn programming"),
        InsertPost::new("Life Hacks", true).with_content("Productivity tips"),
        InsertPost::new("Draft Post", false).with_content("Not published yet"),
    ];

    let post_result = drizzle_exec!(db.insert(post).values(posts).execute());
    assert_eq!(post_result, 3);

    let categories = vec![
        InsertCategory::new("Technology").with_description("Tech related posts"),
        InsertCategory::new("Lifestyle").with_description("Life tips and tricks"),
        InsertCategory::new("Tutorial").with_description("How-to guides"),
    ];

    let category_result = drizzle_exec!(db.insert(category).values(categories).execute());
    assert_eq!(category_result, 3);

    // Create many-to-many relationships
    let post_categories = vec![
        InsertPostCategory::new(1, 1),
        InsertPostCategory::new(1, 3),
        InsertPostCategory::new(2, 2),
        InsertPostCategory::new(3, 1),
    ];

    let junction_result = drizzle_exec!(db.insert(post_category).values(post_categories).execute());
    assert_eq!(junction_result, 4);

    // Test many-to-many join: posts with their categories
    let join_smt = db
        .select(PostCategoryResult::default())
        .from(post)
        .join(post_category, eq(post.id, post_category.post_id))
        .join(category, eq(post_category.category_id, category.id))
        .order_by([OrderBy::asc(post.title), OrderBy::asc(category.name)]);
    let sql = join_smt.to_sql().sql();

    println!("{sql:?}");

    let join_results: Vec<PostCategoryResult> = drizzle_exec!(join_smt.all());

    // Should have 4 results (each post-category relationship)
    assert_eq!(join_results.len(), 4);

    // Verify Draft Post -> Technology
    assert_eq!(join_results[0].post_title, "Draft Post");
    assert_eq!(join_results[0].category_name, "Technology");
    assert_eq!(
        join_results[0].category_description,
        Some("Tech related posts".to_string())
    );

    // Verify Life Hacks -> Lifestyle
    assert_eq!(join_results[1].post_title, "Life Hacks");
    assert_eq!(join_results[1].category_name, "Lifestyle");
    assert_eq!(
        join_results[1].category_description,
        Some("Life tips and tricks".to_string())
    );

    // Verify Tech Tutorial -> Technology
    assert_eq!(join_results[2].post_title, "Tech Tutorial");
    assert_eq!(join_results[2].category_name, "Technology");

    // Verify Tech Tutorial -> Tutorial
    assert_eq!(join_results[3].post_title, "Tech Tutorial");
    assert_eq!(join_results[3].category_name, "Tutorial");

    // Test filtering: only published posts
    let published_results: Vec<PostCategoryResult> = drizzle_exec!(
        db.select(PostCategoryResult::default())
            .from(post)
            .join(post_category, eq(post.id, post_category.post_id))
            .join(category, eq(post_category.category_id, category.id))
            .r#where(eq(post.published, true))
            .order_by([OrderBy::asc(post.title), OrderBy::asc(category.name)])
            .all()
    );

    // Should exclude Draft Post (published = false)
    assert_eq!(published_results.len(), 3);

    // Verify no draft posts in results
    published_results.iter().for_each(|result| {
        assert_ne!(result.post_title, "Draft Post");
    });
});
