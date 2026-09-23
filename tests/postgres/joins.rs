//! PostgreSQL JOIN tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

#[cfg(feature = "uuid")]
use crate::common::schema::postgres::{
    Category, Complex, ComplexPostSchema, FullBlogSchema, InsertCategory, InsertComplex,
    InsertPost, InsertPostCategory, Post, Role,
};
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;

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
#[drizzle::test]
fn auto_fk_join(db: &mut TestDb<ComplexPostSchema>) {
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

    db.insert(complex).values(authors).execute();

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

    db.insert(post).values(posts).execute();

    let join_results: Vec<AuthorPostResult> = db
        .select(AuthorPostResult::default())
        .from(complex)
        .join(post)
        .order_by([asc(complex.name), asc(post.title)])
        .all();

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

    let filtered_results: Vec<AuthorPostResult> = db
        .select(AuthorPostResult::default())
        .from(complex)
        .join(post)
        .r#where(eq(complex.name, "alice"))
        .all();

    assert_eq!(filtered_results.len(), 2);
    filtered_results.iter().for_each(|r| {
        assert_eq!(r.author_name, "alice");
    });
}

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow, Default)]
struct PostCategoryResult {
    #[column(Post::title)]
    post_title: String,
    #[column(Category::name)]
    category_name: String,
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn chained_fk_join(db: &mut TestDb<FullBlogSchema>) {
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
    db.insert(post).values(posts).execute();

    let categories = vec![
        InsertCategory::new("Programming"),
        InsertCategory::new("Tutorial"),
    ];
    db.insert(category).values(categories).execute();

    let links = vec![
        InsertPostCategory::new(post_id1, 1),
        InsertPostCategory::new(post_id1, 2),
        InsertPostCategory::new(post_id2, 1),
    ];
    db.insert(post_category).values(links).execute();

    // Chained auto-FK: post -> post_category (forward FK) -> category (reverse FK)
    let results: Vec<PostCategoryResult> = db
        .select(PostCategoryResult::default())
        .from(post)
        .join(post_category)
        .join(category)
        .order_by([asc(post.title), asc(category.name)])
        .all();

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
}

#[PostgresTable(NAME = "join_using_accounts")]
struct JoinUsingAccount {
    #[column(PRIMARY)]
    account_id: i32,
    owner: String,
}

#[PostgresTable(NAME = "join_using_orders")]
struct JoinUsingOrder {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = JoinUsingAccount::account_id)]
    account_id: i32,
    total: i32,
}

#[derive(PostgresSchema)]
struct JoinUsingSchema {
    accounts: JoinUsingAccount,
    orders: JoinUsingOrder,
}

#[drizzle::test]
fn join_using_matches_same_named_columns(db: &mut TestDb<JoinUsingSchema>) {
    let JoinUsingSchema { accounts, orders } = schema;
    db.insert(accounts)
        .values([
            InsertJoinUsingAccount::new(1, "alice"),
            InsertJoinUsingAccount::new(2, "bob"),
            InsertJoinUsingAccount::new(3, "cleo"),
        ])
        .execute();
    db.insert(orders)
        .values([
            InsertJoinUsingOrder::new(1, 1, 10),
            InsertJoinUsingOrder::new(2, 1, 15),
            InsertJoinUsingOrder::new(3, 2, 7),
        ])
        .execute();

    let stmt = db
        .select((accounts.owner, orders.total))
        .from(accounts)
        .inner_join_using(orders, SQL::ident("account_id"))
        .order_by([asc(orders.id)]);
    assert!(
        stmt.to_sql()
            .sql()
            .contains(r#"INNER JOIN "join_using_orders" USING ("account_id")"#),
        "{}",
        stmt.to_sql().sql()
    );
    let rows: Vec<(String, i32)> = stmt.all();
    assert_eq!(
        rows,
        [
            ("alice".to_string(), 10),
            ("alice".to_string(), 15),
            ("bob".to_string(), 7),
        ]
    );

    // LEFT JOIN ... USING keeps accounts without orders; nullability is
    // tracked on the joined table model.
    let rows: Vec<(SelectJoinUsingAccount, Option<SelectJoinUsingOrder>)> = db
        .select(())
        .from(accounts)
        .left_join_using(orders, SQL::ident("account_id"))
        .order_by([asc(accounts.account_id), asc(orders.id)])
        .all();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].0.owner, "alice");
    assert_eq!(rows[0].1.as_ref().map(|order| order.total), Some(10));
    assert_eq!(rows[3].0.owner, "cleo");
    assert!(rows[3].1.is_none());
}
