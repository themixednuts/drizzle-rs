#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{Complex, InsertComplex, InsertPost, Post};
use crate::common::schema::sqlite::{InsertSimple, Role, Simple};
use drizzle::core::conditions::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

use crate::common::schema::sqlite::SimpleSchema;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{ComplexPostSchema, ComplexSchema};

#[allow(dead_code)]
#[derive(SQLiteFromRow, Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[derive(SQLiteFromRow, Debug)]
struct JoinResult {
    user_name: String,
    post_title: String,
}

#[derive(SQLiteFromRow, Debug)]
struct NamePair {
    name1: String,
    name2: String,
}

sqlite_test!(basic_table_alias, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test basic table alias
    let s = Simple::alias("s");
    let stmt = db.select((s.id, s.name)).from(s).r#where(eq(s.name, "bob"));
    println!("Basic alias SQL: {}", stmt.to_sql());
    let results: Vec<SimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "bob");
});

sqlite_test!(table_alias_with_conditions, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("test1"),
        InsertSimple::new("test2"),
        InsertSimple::new("test3"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Test alias with WHERE conditions
    let s_alias = Simple::alias("s_alias");
    let stmt = db
        .select((s_alias.id, s_alias.name))
        .from(s_alias)
        .r#where(and([gt(s_alias.id, 1), neq(s_alias.name, "test3")]));
    println!("Alias with conditions SQL: {}", stmt.to_sql());
    let results: Vec<SimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test2");
});

#[cfg(feature = "uuid")]
sqlite_test!(self_join_with_aliases, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Insert test data with same email domain
    let test_data = vec![
        InsertComplex::new("user1", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("test@example.com"),
        InsertComplex::new("user2", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("test@example.com"),
        InsertComplex::new("user3", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("other@domain.com"),
    ];

    drizzle_exec!(db.insert(complex).values(test_data).execute());

    // Self-join using aliases to find users with same email
    let c1 = Complex::alias("c1");
    let c2 = Complex::alias("c2");

    let stmt = db
        .select((c1.name.alias("name1"), c2.name.alias("name2")))
        .from(c1)
        .inner_join(c2, eq(c1.email, c2.email))
        .r#where(neq(c1.id, c2.id));
    println!("Self-join with aliases SQL: {}", stmt.to_sql());
    let results: Vec<NamePair> = drizzle_exec!(stmt.all());

    // Should find the pair of users with same email
    assert_eq!(results.len(), 2); // Both directions of the join

    // Verify both users are in the results
    let names: Vec<String> = results
        .iter()
        .flat_map(|pair| vec![pair.name1.clone(), pair.name2.clone()])
        .collect();
    assert!(names.contains(&"user1".to_string()));
    assert!(names.contains(&"user2".to_string()));
});

#[cfg(feature = "uuid")]
sqlite_test!(multiple_table_aliases_join, ComplexPostSchema, {
    let ComplexPostSchema { complex, post } = schema;

    // Insert test users
    let user_id1 = uuid::Uuid::new_v4();
    let user_id2 = uuid::Uuid::new_v4();

    let users = vec![
        InsertComplex::new("author1", true, Role::User).with_id(user_id1),
        InsertComplex::new("author2", true, Role::User).with_id(user_id2),
    ];

    drizzle_exec!(db.insert(complex).values(users).execute());

    // Insert test posts
    let posts = vec![
        InsertPost::new("First Post", true).with_author_id(user_id1),
        InsertPost::new("Second Post", true).with_author_id(user_id2),
        InsertPost::new("Third Post", false).with_author_id(user_id1),
    ];

    drizzle_exec!(db.insert(post).values(posts).execute());

    // Join with aliases
    let u = Complex::alias("u");
    let p = Post::alias("p");

    let stmt = db
        .select((u.name.alias("user_name"), p.title.alias("post_title")))
        .from(u)
        .inner_join(p, eq(u.id, p.author_id))
        .r#where(eq(p.published, true))
        .order_by([OrderBy::asc(u.name)]);
    println!("Multiple table aliases join SQL: {}", stmt.to_sql());
    let results: Vec<JoinResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].user_name, "author1");
    assert_eq!(results[0].post_title, "First Post");
    assert_eq!(results[1].user_name, "author2");
    assert_eq!(results[1].post_title, "Second Post");
});

sqlite_test!(alias_with_original_table_comparison, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![InsertSimple::new("original"), InsertSimple::new("aliased")];

    drizzle_exec!(db.insert(simple).values(test_data).execute());

    // Query using original table reference
    let original_results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "original"))
            .all()
    );

    // Query using table alias
    let s_alias = Simple::alias("s_alias");
    let alias_stmt = db
        .select((s_alias.id, s_alias.name))
        .from(s_alias)
        .r#where(eq(s_alias.name, "aliased"));
    println!("Original vs alias comparison SQL: {}", alias_stmt.to_sql());
    let alias_results: Vec<SimpleResult> = drizzle_exec!(alias_stmt.all());

    assert_eq!(original_results.len(), 1);
    assert_eq!(original_results[0].name, "original");

    assert_eq!(alias_results.len(), 1);
    assert_eq!(alias_results[0].name, "aliased");
});
