#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

mod rusqlite;
#[cfg(feature = "rusqlite")]
pub use rusqlite::*;

mod turso;
#[cfg(feature = "turso")]
pub use turso::setup_db;

mod libsql;
#[cfg(feature = "libsql")]
pub use libsql::*;

pub mod helpers;

use drizzle_rs::prelude::*;

#[cfg(feature = "uuid")]
use uuid::Uuid;

// JSON struct types for testing JSON serialization features
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct UserMetadata {
    pub preferences: Vec<String>,
    pub last_login: Option<String>,
    pub theme: String,
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct UserConfig {
    pub notifications: bool,
    pub language: String,
    pub settings: std::collections::HashMap<String, String>,
}
#[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
pub enum Role {
    #[default]
    User,
    Admin,
}

// Simple type for basic testing
#[SQLiteTable(name = "simple")]
pub struct Simple {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub name: String,
}
#[cfg(all(feature = "uuid", not(feature = "serde")))]
#[SQLiteTable(name = "complex")]
pub struct Complex {
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: Option<i32>,
    #[real]
    pub score: Option<f64>,
    #[boolean]
    pub active: bool,
    #[text(enum)]
    pub role: Role,

    // Text field for regular text storage
    #[text]
    pub description: Option<String>,

    // Raw blob storage
    #[blob]
    pub data_blob: Option<Vec<u8>>,

    #[text]
    pub created_at: Option<String>,
}

#[cfg(all(feature = "uuid", feature = "serde"))]
#[SQLiteTable(name = "complex")]
pub struct Complex {
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: Option<i32>,
    #[real]
    pub score: Option<f64>,
    #[boolean]
    pub active: bool,
    #[text(enum)]
    pub role: Role,

    // Text field for regular text storage
    #[text]
    pub description: Option<String>,

    // JSON stored as text (serde feature)
    #[text(json)]
    pub metadata: Option<UserMetadata>,

    // JSON stored as blob (serde feature)
    #[blob(json)]
    pub config: Option<UserConfig>,

    // Raw blob storage
    #[blob]
    pub data_blob: Option<Vec<u8>>,

    #[text]
    pub created_at: Option<String>,
}
#[cfg(all(not(feature = "uuid"), feature = "serde"))]
#[SQLiteTable(name = "complex")]
pub struct Complex {
    #[integer(primary)]
    pub id: i64,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: Option<i32>,
    #[real]
    pub score: Option<f64>,
    #[boolean]
    pub active: bool,
    #[text(enum)]
    pub role: Role,

    // Text field for regular text storage
    #[text]
    pub description: Option<String>,

    // JSON stored as text (serde feature)
    #[text(json)]
    pub metadata: Option<UserMetadata>,

    // JSON stored as blob (serde feature)
    #[blob(json)]
    pub config: Option<UserConfig>,

    // Raw blob storage
    #[blob]
    pub data_blob: Option<Vec<u8>>,

    #[text]
    pub created_at: Option<String>,
}

#[cfg(all(not(feature = "uuid"), not(feature = "serde")))]
#[SQLiteTable(name = "complex")]
pub struct Complex {
    #[integer(primary)]
    pub id: i64,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: Option<i32>,
    #[real]
    pub score: Option<f64>,
    #[boolean]
    pub active: bool,
    #[text(enum)]
    pub role: Role,

    // Text field for regular text storage
    #[text]
    pub description: Option<String>,

    // Raw blob storage
    #[blob]
    pub data_blob: Option<Vec<u8>>,

    #[text]
    pub created_at: Option<String>,
}

#[cfg(all(not(feature = "uuid"), feature = "serde"))]
#[SQLiteTable(name = "posts")]
pub struct Post {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub title: String,
    #[text]
    pub content: Option<String>,
    #[integer(references = Complex::id)]
    pub author_id: Option<i32>,
    #[boolean]
    pub published: bool,
    #[text]
    pub tags: Option<String>,
    #[text]
    pub created_at: Option<String>,
}

#[cfg(all(not(feature = "uuid"), not(feature = "serde")))]
#[SQLiteTable(name = "posts")]
pub struct Post {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub title: String,
    #[text]
    pub content: Option<String>,
    #[integer(references = Complex::id)]
    pub author_id: Option<i32>,
    #[boolean]
    pub published: bool,
    #[text]
    pub tags: Option<String>,
    #[text]
    pub created_at: Option<String>,
}

#[cfg(feature = "uuid")]
#[SQLiteTable(name = "posts")]
pub struct Post {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub title: String,
    #[text]
    pub content: Option<String>,
    #[blob(references = Complex::id)]
    pub author_id: Option<Uuid>,
    #[boolean]
    pub published: bool,
    #[text]
    pub tags: Option<String>,
    #[text]
    pub created_at: Option<String>,
}

#[SQLiteTable(name = "categories")]
pub struct Category {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub name: String,
    #[text]
    pub description: Option<String>,
}

#[SQLiteTable(name = "post_categories")]
pub struct PostCategory {
    #[integer]
    pub post_id: i32,
    #[integer]
    pub category_id: i32,
}

#[cfg(feature = "uuid")]
#[derive(SQLSchema)]
pub struct SimpleComplexSchema {
    pub simple: Simple,
    pub complex: Complex,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLSchema)]
pub struct SimpleComplexSchema {
    pub simple: Simple,
    pub complex: Complex,
}

#[derive(SQLSchema)]
pub struct SimpleSchema {
    pub simple: Simple,
}

#[cfg(feature = "uuid")]
#[derive(SQLSchema)]
pub struct ComplexSchema {
    pub complex: Complex,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLSchema)]
pub struct ComplexSchema {
    pub complex: Complex,
}

#[derive(SQLSchema)]
pub struct PostSchema {
    pub post: Post,
}

#[cfg(feature = "uuid")]
#[derive(SQLSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[derive(SQLSchema)]
pub struct CategorySchema {
    pub category: Category,
}

#[derive(SQLSchema)]
pub struct PostCategorySchema {
    pub post_category: PostCategory,
}

#[cfg(feature = "uuid")]
#[derive(SQLSchema)]
pub struct FullBlogSchema {
    pub simple: Simple,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLSchema)]
pub struct FullBlogSchema {
    pub simple: Simple,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
}
