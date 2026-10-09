#![cfg(all(
    any(feature = "postgres-sync", feature = "tokio-postgres"),
    feature = "query",
    feature = "uuid"
))]

//! Relational Query API on PostgreSQL.
//!
//! The portable scenarios live in `crate::common::relational`; this file keeps
//! the schema-qualification metadata checks and two UUID-keyed scenarios so
//! that non-integer keys stay covered.

use drizzle::core::asc;
use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;
use uuid::Uuid;

use crate::common::schema::postgres::{
    Category, Complex, InsertCategory, InsertComplex, InsertPost, InsertPostCategory, Post,
    PostCategory, Role, SelectCategory, SelectComplex, SelectPost,
};

crate::common::query::shared_relational_query_suite!(
    postgres,
    PostgresTable,
    PostgresSchema,
    drizzle::postgres::types::Int4,
    drizzle_postgres::common::PostgresTransactionType::default()
);
crate::common::query::shared_view_query_suite!(
    postgres,
    PostgresTable,
    PostgresView,
    PostgresSchema
);
crate::common::relational::shared_relational_api_suite!(
    postgres,
    PostgresTable,
    PostgresView,
    PostgresSchema,
    drizzle::postgres::types::Int4,
    drizzle_postgres::common::PostgresTransactionType::default()
);

#[PostgresTable(TEMPORARY, NAME = "query_temp_metadata")]
struct QueryTempMetadata {
    #[column(PRIMARY)]
    id: i32,
}

#[PostgresTable(SCHEMA = "tenant", NAME = "query_tenant_metadata")]
struct QueryTenantMetadata {
    #[column(PRIMARY)]
    id: i32,
}

#[test]
fn query_table_metadata_only_qualifies_explicit_postgres_schemas() {
    assert_eq!(
        <QueryTempMetadata as drizzle::core::query::QueryTable>::TABLE.schema,
        None
    );
    assert_eq!(
        <QueryTenantMetadata as drizzle::core::query::QueryTable>::TABLE.schema,
        Some("tenant")
    );
}

#[PostgresView(DEFINITION = "SELECT id, title, author_id FROM post")]
struct PostView {
    id: Uuid,
    title: String,
    author_id: Option<Uuid>,
}

#[PostgresView(EXISTING, NAME = "qualified_post_view", SCHEMA = "analytics")]
struct QualifiedPostView {
    id: Uuid,
    title: String,
    author_id: Option<Uuid>,
}

#[test]
fn query_view_schema_metadata_only_qualifies_explicit_schemas() {
    assert_eq!(
        <PostView as drizzle::core::query::QueryTable>::TABLE_SCHEMA,
        None
    );
    assert_eq!(
        <QualifiedPostView as drizzle::core::query::QueryTable>::TABLE_SCHEMA,
        Some("analytics")
    );
}

#[derive(PostgresSchema)]
struct ComplexPostQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
}

#[derive(PostgresSchema)]
struct M2MQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
    category: Category,
    post_category: PostCategory,
}

// -- Reverse relation: Complex -> Posts (Many) --
#[drizzle::test]
fn query_reverse_relation_many(db: &mut TestDb<ComplexPostQuerySchema>) {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    // Insert users

    db.insert(complex)
        .values([
            InsertComplex::new("Alice", true, Role::User),
            InsertComplex::new("Bob", true, Role::User),
        ])
        .execute();

    let all_users: Vec<SelectComplex> = db.select(()).from(complex).all();
    let alice_id = all_users.iter().find(|u| u.name == "Alice").unwrap().id;
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Insert posts

    db.insert(post)
        .values([
            InsertPost::new("Alice Post 1", true).with_author_id(alice_id),
            InsertPost::new("Alice Post 2", true).with_author_id(alice_id),
            InsertPost::new("Bob Post 1", true).with_author_id(bob_id),
        ])
        .execute();

    // Query users with their posts
    let users = db.query(complex).with(complex.author_posts()).find_many();

    assert_eq!(users.len(), 2);

    // Alice has 2 posts
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(alice.author_posts.len(), 2);
    assert_eq!(alice.author_posts[0].title, "Alice Post 1");
    assert_eq!(alice.author_posts[1].title, "Alice Post 2");

    // Bob has 1 post
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(bob.author_posts.len(), 1);
    assert_eq!(bob.author_posts[0].title, "Bob Post 1");
}

// -- basic m2m: post.categories returns categories through junction --
#[drizzle::test]
fn query_many_to_many_basic(db: &mut TestDb<M2MQuerySchema>) {
    let M2MQuerySchema {
        complex,
        post,
        category,
        post_category,
        ..
    } = schema;

    // Insert author

    db.insert(complex)
        .values([InsertComplex::new("Alice", true, Role::User)])
        .execute();
    let all_users: Vec<SelectComplex> = db.select(()).from(complex).all();
    let alice_id = all_users[0].id;

    // Insert post

    db.insert(post)
        .values([InsertPost::new("My Post", true).with_author_id(alice_id)])
        .execute();
    let all_posts: Vec<SelectPost> = db.select(()).from(post).all();
    let post_id = all_posts[0].id;

    // Insert categories

    db.insert(category)
        .values([InsertCategory::new("Tech"), InsertCategory::new("Science")])
        .execute();
    let all_cats: Vec<SelectCategory> = db.select(()).from(category).all();

    // Link post to both categories

    db.insert(post_category)
        .values([
            InsertPostCategory::new(post_id, all_cats[0].id),
            InsertPostCategory::new(post_id, all_cats[1].id),
        ])
        .execute();

    // Query posts with their categories through the junction
    let posts = db.query(post).with(post.categories()).find_many();

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].title, "My Post");
    assert_eq!(posts[0].categories.len(), 2);
    let cat_names: Vec<&str> = posts[0]
        .categories
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(cat_names.contains(&"Tech"));
    assert!(cat_names.contains(&"Science"));
}

#[PostgresTable(NAME = "query_wide_parents")]
struct WideParent {
    #[column(PRIMARY)]
    id: i32,
}

/// More columns than one `json_build_object` call takes (100 arguments, so
/// 50 key-value pairs).
#[PostgresTable(NAME = "query_wide_children")]
struct WideChild {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = WideParent::id)]
    wide_parent_id: i32,
    c1: Option<i32>,
    c2: Option<i32>,
    c3: Option<i32>,
    c4: Option<i32>,
    c5: Option<i32>,
    c6: Option<i32>,
    c7: Option<i32>,
    c8: Option<i32>,
    c9: Option<i32>,
    c10: Option<i32>,
    c11: Option<i32>,
    c12: Option<i32>,
    c13: Option<i32>,
    c14: Option<i32>,
    c15: Option<i32>,
    c16: Option<i32>,
    c17: Option<i32>,
    c18: Option<i32>,
    c19: Option<i32>,
    c20: Option<i32>,
    c21: Option<i32>,
    c22: Option<i32>,
    c23: Option<i32>,
    c24: Option<i32>,
    c25: Option<i32>,
    c26: Option<i32>,
    c27: Option<i32>,
    c28: Option<i32>,
    c29: Option<i32>,
    c30: Option<i32>,
    c31: Option<i32>,
    c32: Option<i32>,
    c33: Option<i32>,
    c34: Option<i32>,
    c35: Option<i32>,
    c36: Option<i32>,
    c37: Option<i32>,
    c38: Option<i32>,
    c39: Option<i32>,
    c40: Option<i32>,
    c41: Option<i32>,
    c42: Option<i32>,
    c43: Option<i32>,
    c44: Option<i32>,
    c45: Option<i32>,
    c46: Option<i32>,
    c47: Option<i32>,
    c48: Option<i32>,
    c49: Option<i32>,
    c50: Option<i32>,
    c51: Option<i32>,
    c52: Option<i32>,
    c53: Option<i32>,
    c54: Option<i32>,
    c55: Option<i32>,
}

#[derive(PostgresSchema)]
struct WideSchema {
    wide_parent: WideParent,
    wide_child: WideChild,
}

#[drizzle::test]
fn relations_load_rows_wider_than_one_json_object_call(db: &mut TestDb<WideSchema>) {
    let WideSchema {
        wide_parent,
        wide_child,
    } = schema;
    db.insert(wide_parent)
        .values([InsertWideParent::new(1)])
        .execute();
    db.insert(wide_child)
        .values([InsertWideChild::new(10, 1)
            .with_c1(1)
            .with_c50(50)
            .with_c55(55)])
        .execute();

    let parents = db
        .query(wide_parent)
        .with(wide_parent.wide_children())
        .find_many();
    assert_eq!(parents.len(), 1);
    let child = &parents[0].wide_children[0];
    assert_eq!(
        (child.id, child.c1, child.c50, child.c51, child.c55),
        (10, Some(1), Some(50), None, Some(55))
    );
}
