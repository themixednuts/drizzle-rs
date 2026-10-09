#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    feature = "query",
    feature = "uuid"
))]

//! Relational Query API on SQLite.
//!
//! The portable scenarios live in `crate::common::relational`; this file keeps
//! two UUID-keyed scenarios so that non-integer keys stay covered.

use drizzle::core::asc;
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;

use crate::common::schema::sqlite::{
    Category, Complex, InsertCategory, InsertComplex, InsertPost, InsertPostCategory, Post,
    PostCategory, Role, SelectCategory, SelectComplex, SelectPost,
};

crate::common::query::shared_relational_query_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    drizzle::sqlite::types::Integer,
    drizzle::sqlite::connection::SQLiteTransactionType::Deferred
);
crate::common::query::shared_view_query_suite!(sqlite, SQLiteTable, SQLiteView, SQLiteSchema);
crate::common::relational::shared_relational_api_suite!(
    sqlite,
    SQLiteTable,
    SQLiteView,
    SQLiteSchema,
    drizzle::sqlite::types::Integer,
    drizzle::sqlite::connection::SQLiteTransactionType::Deferred
);

#[derive(SQLiteSchema)]
struct ComplexPostQuerySchema {
    complex: Complex,
    post: Post,
}

#[derive(SQLiteSchema)]
struct M2MQuerySchema {
    complex: Complex,
    post: Post,
    category: Category,
    post_category: PostCategory,
}

// -- Reverse relation: Complex -> Posts (Many) --
#[drizzle::test]
fn query_reverse_relation_many(db: &mut TestDb<ComplexPostQuerySchema>) {
    let ComplexPostQuerySchema { complex, post } = schema;

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

#[SQLiteTable(NAME = "query_mutual_users")]
struct Member {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    #[column(REFERENCES = Squad::id)]
    current_squad_id: Option<i32>,
}

#[SQLiteTable(NAME = "query_mutual_squads")]
struct Squad {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    #[column(REFERENCES = Member::id)]
    owner_id: Option<i32>,
}

#[derive(SQLiteSchema)]
struct MutualQuerySchema {
    members: Member,
    squads: Squad,
}

/// Members and squads have keys to each other: a member's current squad, a
/// squad's owner. Every relation loads, nested through the cycle too.
#[drizzle::test]
fn relations_load_between_tables_keyed_to_each_other(db: &mut TestDb<MutualQuerySchema>) {
    let MutualQuerySchema { members, squads } = schema;
    db.insert(members)
        .values([
            InsertMember::new("Ada").with_id(1),
            InsertMember::new("Bob").with_id(2),
        ])
        .execute();
    db.insert(squads)
        .value(InsertSquad::new("Core").with_id(7).with_owner_id(1))
        .execute();
    db.update(members)
        .set(UpdateMember::default().with_current_squad_id(7))
        .r#where(true)
        .execute();

    let ada = db
        .query(members)
        .with(members.current_squad().with(squads.owner()))
        .with(members.owner_squads())
        .r#where(eq(members.id, 1))
        .find_first()
        .expect("Ada");
    let squad = ada.current_squad.as_ref().expect("Ada's squad");
    assert_eq!(squad.name, "Core");
    assert_eq!(squad.owner.as_ref().expect("an owner").name, "Ada");
    assert_eq!(ada.owner_squads.len(), 1);

    let core = db
        .query(squads)
        .with(squads.current_squad_members())
        .find_first()
        .expect("Core");
    let mut names: Vec<_> = core
        .current_squad_members
        .iter()
        .map(|member| member.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Ada", "Bob"]);
}
