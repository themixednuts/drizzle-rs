#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle_macros::drizzle_test;
use drizzle_rs::prelude::*;
mod common;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
use crate::common::{Complex, InsertComplex, InsertPost, Post, SelectPost};
#[cfg(not(feature = "uuid"))]
use crate::common::{FullBlogSchema, InsertPost, Post, SelectPost};

#[test]
fn test_foreign_key_generation() {
    // Test that foreign keys are generated correctly
    let post_instance = Post::default();
    let post_sql = post_instance.sql().sql();

    println!("Post table SQL: {}", post_sql);

    assert!(post_sql.contains("CREATE TABLE"));
    assert!(post_sql.contains("posts"));

    // Check for foreign key constraint
    assert!(
        post_sql.contains("REFERENCES"),
        "Post table should contain REFERENCES for foreign key"
    );
    assert!(
        post_sql.contains("complex"),
        "Post table should reference users table"
    );
    assert!(
        post_sql.contains("(id)"),
        "Post table should reference id column"
    );
}

#[cfg(feature = "uuid")]
#[derive(SQLSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[cfg(feature = "uuid")]
drizzle_test!(test_foreign_key_impl, ComplexPostSchema, {
    let ComplexPostSchema { complex, post } = schema;

    let id = Uuid::new_v4();

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("John", false, common::Role::User).with_id(id)])
            .execute()
    );

    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("test", true).with_author_id(id)])
            .execute()
    );

    let row: SelectPost = drizzle_exec!(
        db.select(())
            .from(post)
            .r#where(eq(post.author_id, id))
            .get()
    );

    assert_eq!(row.author_id, Some(id));
});
