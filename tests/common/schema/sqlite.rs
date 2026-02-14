use drizzle::sqlite::prelude::*;

#[cfg(feature = "uuid")]
use uuid::Uuid;

// JSON struct types for testing JSON serialization features
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UserMetadata {
    pub preferences: Vec<String>,
    pub last_login: Option<String>,
    pub theme: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[SQLiteTable]
pub struct Simple {
    #[column(PRIMARY)]
    pub id: i32,
    pub name: String,
}
#[cfg(all(feature = "uuid", not(feature = "serde")))]
#[SQLiteTable]
pub struct Complex {
    #[column(PRIMARY, DEFAULT_FN = Uuid::new_v4)]
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(ENUM)]
    pub role: Role,

    // Text field for regular text storage
    pub description: Option<String>,

    // Raw blob storage
    pub data_blob: Option<Vec<u8>>,

    pub created_at: Option<String>,
}

#[cfg(all(feature = "uuid", feature = "serde"))]
#[SQLiteTable]
pub struct Complex {
    #[column(PRIMARY, DEFAULT_FN = Uuid::new_v4)]
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(ENUM)]
    pub role: Role,

    // Text field for regular text storage
    pub description: Option<String>,

    // JSON stored as text (serde feature)
    #[column(JSON)]
    pub metadata: Option<UserMetadata>,

    // JSON stored as blob (serde feature)
    #[column(JSONB)]
    pub config: Option<UserConfig>,

    // Raw blob storage
    pub data_blob: Option<Vec<u8>>,

    pub created_at: Option<String>,
}
#[cfg(all(not(feature = "uuid"), feature = "serde"))]
#[SQLiteTable]
pub struct Complex {
    #[column(PRIMARY)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(ENUM)]
    pub role: Role,

    // Text field for regular text storage
    pub description: Option<String>,

    // JSON stored as text (serde feature)
    #[column(JSON)]
    pub metadata: Option<UserMetadata>,

    // JSON stored as blob (serde feature)
    #[column(JSONB)]
    pub config: Option<UserConfig>,

    // Raw blob storage
    pub data_blob: Option<Vec<u8>>,

    pub created_at: Option<String>,
}

#[cfg(all(not(feature = "uuid"), not(feature = "serde")))]
#[SQLiteTable]
pub struct Complex {
    #[column(PRIMARY)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(ENUM)]
    pub role: Role,

    // Text field for regular text storage
    pub description: Option<String>,

    // Raw blob storage
    pub data_blob: Option<Vec<u8>>,

    pub created_at: Option<String>,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable(NAME = "posts")]
pub struct Post {
    #[column(PRIMARY)]
    pub id: i32,
    pub title: String,
    pub content: Option<String>,
    #[column(REFERENCES = Complex::id)]
    pub author_id: Option<i64>,
    pub published: bool,
    pub tags: Option<String>,
    pub created_at: Option<String>,
}

#[cfg(feature = "uuid")]
#[SQLiteTable(NAME = "posts")]
pub struct Post {
    #[column(PRIMARY, DEFAULT_FN = Uuid::new_v4)]
    pub id: Uuid,
    pub title: String,
    pub content: Option<String>,
    #[column(REFERENCES = Complex::id)]
    pub author_id: Option<Uuid>,
    pub published: bool,
    pub tags: Option<String>,
    pub created_at: Option<String>,
}

#[SQLiteTable(NAME = "categories")]
pub struct Category {
    #[column(PRIMARY)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable(NAME = "post_categories")]
pub struct PostCategory {
    #[column(REFERENCES = Post::id)]
    pub post_id: i32,
    #[column(REFERENCES = Category::id)]
    pub category_id: i32,
}

#[cfg(feature = "uuid")]
#[SQLiteTable(NAME = "post_categories")]
pub struct PostCategory {
    #[column(REFERENCES = Post::id)]
    pub post_id: Uuid,
    #[column(REFERENCES = Category::id)]
    pub category_id: i32,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable(NAME = "comments")]
pub struct Comment {
    #[column(PRIMARY)]
    pub id: i32,
    pub body: String,
    #[column(REFERENCES = Post::id)]
    pub post_id: i32,
}

#[cfg(feature = "uuid")]
#[SQLiteTable(NAME = "comments")]
pub struct Comment {
    #[column(PRIMARY)]
    pub id: i32,
    pub body: String,
    #[column(REFERENCES = Post::id)]
    pub post_id: Uuid,
}

#[derive(SQLiteSchema)]
pub struct SimpleComplexSchema {
    pub simple: Simple,
    pub complex: Complex,
}

#[derive(SQLiteSchema, Debug)]
pub struct SimpleSchema {
    pub simple: Simple,
}

#[derive(SQLiteSchema)]
pub struct ComplexSchema {
    pub complex: Complex,
}

#[derive(SQLiteSchema)]
pub struct PostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[derive(SQLiteSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[derive(SQLiteSchema)]
pub struct CategorySchema {
    pub category: Category,
}

#[derive(SQLiteSchema)]
pub struct PostCategorySchema {
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
}

#[derive(SQLiteSchema)]
pub struct FullBlogSchema {
    pub simple: Simple,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
    pub comment: Comment,
}
