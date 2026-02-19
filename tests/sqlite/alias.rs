#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{Complex, InsertComplex, InsertPost, Post};
use crate::common::schema::sqlite::{InsertSimple, Simple};
use drizzle::core::expr::*;
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

#[cfg(feature = "uuid")]
#[derive(SQLiteFromRow, Debug)]
struct NamePair {
    name1: String,
    name2: String,
}

struct AliasS;
impl drizzle::core::Tag for AliasS {
    const NAME: &'static str = "s";
}

struct AliasSimple;
impl drizzle::core::Tag for AliasSimple {
    const NAME: &'static str = "s_alias";
}

#[cfg(feature = "uuid")]
struct AliasC1;
#[cfg(feature = "uuid")]
impl drizzle::core::Tag for AliasC1 {
    const NAME: &'static str = "c1";
}

#[cfg(feature = "uuid")]
struct AliasC2;
#[cfg(feature = "uuid")]
impl drizzle::core::Tag for AliasC2 {
    const NAME: &'static str = "c2";
}

#[cfg(feature = "uuid")]
struct AliasU;
#[cfg(feature = "uuid")]
impl drizzle::core::Tag for AliasU {
    const NAME: &'static str = "u";
}

#[cfg(feature = "uuid")]
struct AliasP;
#[cfg(feature = "uuid")]
impl drizzle::core::Tag for AliasP {
    const NAME: &'static str = "p";
}

sqlite_test!(basic_table_alias, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let test_data = vec![
        InsertSimple::new("alice"),
        InsertSimple::new("bob"),
        InsertSimple::new("charlie"),
    ];

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test basic table alias
    let s = Simple::alias::<AliasS>();
    let stmt = db
        .select(SimpleResult::Select)
        .from(s)
        .r#where(eq(s.name, "bob"));
    let results: Vec<SimpleResult> = drizzle_exec!(stmt => all);

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

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Test alias with WHERE conditions
    let s_alias = Simple::alias::<AliasSimple>();
    let stmt = db
        .select((s_alias.id, s_alias.name))
        .from(s_alias)
        .r#where(and([gt(s_alias.id, 1), neq(s_alias.name, "test3")]));
    let results: Vec<SimpleResult> = drizzle_exec!(stmt => all);

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

    drizzle_exec!(db.insert(complex).values(test_data) => execute);

    // Self-join using aliases to find users with same email
    let c1 = Complex::alias::<AliasC1>();
    let c2 = Complex::alias::<AliasC2>();

    let stmt = db
        .select((c1.name.alias("name1"), c2.name.alias("name2")))
        .from(c1)
        .inner_join((c2, eq(c1.email, c2.email)))
        .r#where(neq(c1.id, c2.id));
    let results: Vec<NamePair> = drizzle_exec!(stmt => all);

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

    drizzle_exec!(db.insert(complex).values(users) => execute);

    // Insert test posts
    let posts = vec![
        InsertPost::new("First Post", true).with_author_id(user_id1),
        InsertPost::new("Second Post", true).with_author_id(user_id2),
        InsertPost::new("Third Post", false).with_author_id(user_id1),
    ];

    drizzle_exec!(db.insert(post).values(posts) => execute);

    // Join with aliases
    let u = Complex::alias::<AliasU>();
    let p = Post::alias::<AliasP>();

    let stmt = db
        .select((u.name.alias("user_name"), p.title.alias("post_title")))
        .from(u)
        .inner_join((p, eq(u.id, p.author_id)))
        .r#where(eq(p.published, true))
        .order_by([asc(u.name)]);
    let results: Vec<JoinResult> = drizzle_exec!(stmt => all);

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

    drizzle_exec!(db.insert(simple).values(test_data) => execute);

    // Query using original table reference
    let original_results: Vec<SimpleResult> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "original"))
            => all
    );

    // Query using table alias
    let s_alias = Simple::alias::<AliasSimple>();
    let alias_stmt = db
        .select((s_alias.id, s_alias.name))
        .from(s_alias)
        .r#where(eq(s_alias.name, "aliased"));
    let alias_results: Vec<SimpleResult> = drizzle_exec!(alias_stmt => all);

    assert_eq!(original_results.len(), 1);
    assert_eq!(original_results[0].name, "original");

    assert_eq!(alias_results.len(), 1);
    assert_eq!(alias_results[0].name, "aliased");
});

sqlite_test!(tagged_alias_forwards_alias_metadata, SimpleSchema, {
    let tagged = Simple::alias::<AliasSimple>();
    let base = Simple::new();

    assert_eq!(tagged.name(), "s_alias");
    assert!(!std::ptr::eq(tagged.columns(), base.columns()));
    assert!(!std::ptr::eq(
        tagged.sqlite_columns(),
        base.sqlite_columns()
    ));
});

sqlite_test!(runtime_alias_named_uses_base_metadata, SimpleSchema, {
    let runtime =
        <Simple as SQLTable<'static, SQLiteSchemaType, SQLiteValue<'static>>>::alias_named(
            "runtime_simple",
        );
    let base = Simple::new();

    assert_eq!(runtime.name(), "runtime_simple");
    assert!(std::ptr::eq(runtime.columns(), base.columns()));
    assert!(std::ptr::eq(
        runtime.sqlite_columns(),
        base.sqlite_columns()
    ));
});
