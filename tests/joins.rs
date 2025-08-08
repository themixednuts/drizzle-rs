use common::{
    Category, Complex, InsertCategory, InsertComplex, InsertPost, InsertPostCategory, Post,
    PostCategory, setup_db,
};
use drizzle_core::{OrderBy, sql};
use drizzle_rs::prelude::*;
use rusqlite::Row;

mod common;

#[derive(Debug)]
struct AuthorPostResult {
    author_name: String,
    post_title: String,
    post_content: Option<String>,
}

impl TryFrom<&Row<'_>> for AuthorPostResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            author_name: row.get(0)?,
            post_title: row.get(1)?,
            post_content: row.get(2)?,
        })
    }
}

#[derive(Debug)]
struct PostCategoryResult {
    post_title: String,
    category_name: String,
    category_description: Option<String>,
}

impl TryFrom<&Row<'_>> for PostCategoryResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            post_title: row.get(0)?,
            category_name: row.get(1)?,
            category_description: row.get(2)?,
        })
    }
}

#[derive(Debug)]
struct PostResult {
    id: i32,
    title: String,
    content: Option<String>,
    published: bool,
}

impl TryFrom<&Row<'_>> for PostResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            published: row.get(3)?,
        })
    }
}

#[test]
fn simple_inner_join() {
    let db = setup_db();
    let (drizzle, (complex, post, ..)) = drizzle!(db, [Complex, Post, PostCategory, Category]);

    // Insert test data: authors and their posts
    #[cfg(not(feature = "uuid"))]
    let authors = vec![
        InsertComplex::default()
            .with_name("alice")
            .with_email("alice@example.com".to_string()),
        InsertComplex::default()
            .with_name("bob")
            .with_email("bob@example.com".to_string()),
        InsertComplex::default()
            .with_name("charlie")
            .with_email("charlie@example.com".to_string()), // No posts
    ];

    #[cfg(feature = "uuid")]
    let authors = vec![
        InsertComplex::default()
            .with_id(uuid::Uuid::new_v4())
            .with_name("alice")
            .with_email("alice@example.com".to_string()),
        InsertComplex::default()
            .with_id(uuid::Uuid::new_v4())
            .with_name("bob")
            .with_email("bob@example.com".to_string()),
        InsertComplex::default()
            .with_id(uuid::Uuid::new_v4())
            .with_name("charlie")
            .with_email("charlie@example.com".to_string()), // No posts
    ];

    let author_result = drizzle.insert(complex).values(authors).execute().unwrap();
    assert_eq!(author_result, 3);

    let posts = vec![
        InsertPost::default()
            .with_title("Alice's First Post")
            .with_content("Content by Alice".to_string())
            .with_author_id(1),
        InsertPost::default()
            .with_title("Bob's Adventure")
            .with_content("Travel blog by Bob".to_string())
            .with_author_id(2),
        InsertPost::default()
            .with_title("Alice's Second Post")
            .with_content("More content by Alice".to_string())
            .with_author_id(1),
    ];

    let post_result = drizzle.insert(post).values(posts).execute().unwrap();
    assert_eq!(post_result, 3);

    // Test inner join: only authors with posts should appear
    let join_results: Vec<AuthorPostResult> = drizzle
        .select(columns![Complex::name, Post::title, Post::content])
        .from(complex)
        .inner_join(post, eq(Complex::id, Post::author_id))
        .order_by(sql![
            (Complex::name, OrderBy::Asc),
            (Post::title, OrderBy::Asc)
        ])
        .all()
        .unwrap();

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
    let alice_posts: Vec<AuthorPostResult> = drizzle
        .select(columns![Complex::name, Post::title, Post::content])
        .from(complex)
        .inner_join(post, eq(Complex::id, Post::author_id))
        .r#where(eq(Complex::name, "alice"))
        .all()
        .unwrap();

    assert_eq!(alice_posts.len(), 2);
    alice_posts.iter().for_each(|result| {
        assert_eq!(result.author_name, "alice");
    });
}

#[test]
fn many_to_many_join() {
    let db = setup_db();
    let (drizzle, (complex, post, postcategory, category)) =
        drizzle!(db, [Complex, Post, PostCategory, Category]);

    // Insert test data: posts and categories with many-to-many relationship
    let posts = vec![
        InsertPost::default()
            .with_title("Tech Tutorial")
            .with_content("Learn programming".to_string())
            .with_published(true),
        InsertPost::default()
            .with_title("Life Hacks")
            .with_content("Productivity tips".to_string())
            .with_published(true),
        InsertPost::default()
            .with_title("Draft Post")
            .with_content("Not published yet".to_string())
            .with_published(false),
    ];

    let post_result = drizzle.insert(post).values(posts).execute().unwrap();
    assert_eq!(post_result, 3);

    let categories = vec![
        InsertCategory::default()
            .with_name("Technology")
            .with_description("Tech related posts".to_string()),
        InsertCategory::default()
            .with_name("Lifestyle")
            .with_description("Life tips and tricks".to_string()),
        InsertCategory::default()
            .with_name("Tutorial")
            .with_description("How-to guides".to_string()),
    ];

    let category_result = drizzle
        .insert(category)
        .values(categories)
        .execute()
        .unwrap();
    assert_eq!(category_result, 3);

    // Create many-to-many relationships
    let post_categories = vec![
        InsertPostCategory::default()
            .with_post_id(1)
            .with_category_id(1), // Tech Tutorial -> Technology
        InsertPostCategory::default()
            .with_post_id(1)
            .with_category_id(3), // Tech Tutorial -> Tutorial
        InsertPostCategory::default()
            .with_post_id(2)
            .with_category_id(2), // Life Hacks -> Lifestyle
        InsertPostCategory::default()
            .with_post_id(3)
            .with_category_id(1), // Draft Post -> Technology
    ];

    let junction_result = drizzle
        .insert(postcategory)
        .values(post_categories)
        .execute()
        .unwrap();
    assert_eq!(junction_result, 4);

    // Test many-to-many join: posts with their categories
    let join_smt = drizzle
        .select(columns![Post::title, Category::name, Category::description])
        .from(post)
        .join(postcategory, eq(Post::id, PostCategory::post_id))
        .join(category, eq(PostCategory::category_id, Category::id))
        .order_by(sql![
            (Post::title, OrderBy::Asc),
            (Category::name, OrderBy::Asc)
        ]);
    let sql = join_smt.to_sql().sql();

    println!("{sql:?}");

    let join_results: Vec<PostCategoryResult> = join_smt.all().unwrap();

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
    let published_results: Vec<PostCategoryResult> = drizzle
        .select(columns![Post::title, Category::name, Category::description])
        .from(post)
        .join(postcategory, eq(Post::id, PostCategory::post_id))
        .join(category, eq(PostCategory::category_id, Category::id))
        .r#where(eq(Post::published, true))
        .order_by(sql![
            (Post::title, OrderBy::Asc),
            (Category::name, OrderBy::Asc)
        ])
        .all()
        .unwrap();

    // Should exclude Draft Post (published = false)
    assert_eq!(published_results.len(), 3);

    // Verify no draft posts in results
    published_results.iter().for_each(|result| {
        assert_ne!(result.post_title, "Draft Post");
    });
}
