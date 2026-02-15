use drizzle::postgres::prelude::*;

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

/// PostgreSQL enum for user roles
#[derive(PostgresEnum, Default, Copy, Clone, PartialEq, Debug)]
pub enum Role {
    #[default]
    User,
    Admin,
    Moderator,
}

/// PostgreSQL enum for post status
#[derive(PostgresEnum, Default, Copy, Clone, PartialEq, Debug)]
pub enum PostStatus {
    #[default]
    Draft,
    Published,
    Archived,
}

/// PostgreSQL enum for priority levels
#[derive(PostgresEnum, Default, Copy, Clone, PartialEq, Debug)]
pub enum Priority {
    Low = 1,
    #[default]
    Medium = 5,
    High = 10,
}

// Simple table for basic testing
#[PostgresTable]
pub struct Simple {
    #[column(serial, primary)]
    pub id: i32,
    pub name: String,
}

// Complex table with various PostgreSQL-specific types
#[cfg(all(feature = "uuid", not(feature = "serde")))]
#[PostgresTable]
pub struct Complex {
    #[column(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(enum)]
    pub role: Role,
    pub description: Option<String>,
    pub data_blob: Option<Vec<u8>>,
    pub created_at: Option<String>,
}

#[cfg(all(feature = "uuid", feature = "serde"))]
#[PostgresTable]
pub struct Complex {
    #[column(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(enum)]
    pub role: Role,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub config: Option<String>,
    pub data_blob: Option<Vec<u8>>,
    pub created_at: Option<String>,
}

#[cfg(all(not(feature = "uuid"), feature = "serde"))]
#[PostgresTable]
pub struct Complex {
    #[column(serial, primary)]
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(enum)]
    pub role: Role,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub config: Option<String>,
    pub data_blob: Option<Vec<u8>>,
    pub created_at: Option<String>,
}

#[cfg(all(not(feature = "uuid"), not(feature = "serde")))]
#[PostgresTable]
pub struct Complex {
    #[column(serial, primary)]
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub score: Option<f64>,
    pub active: bool,
    #[column(enum)]
    pub role: Role,
    pub description: Option<String>,
    pub data_blob: Option<Vec<u8>>,
    pub created_at: Option<String>,
}

// Posts table for join testing
#[cfg(not(feature = "uuid"))]
#[PostgresTable]
pub struct Post {
    #[column(serial, primary)]
    pub id: i32,
    pub title: String,
    pub content: Option<String>,
    #[column(references = Complex::id)]
    pub author_id: Option<i32>,
    pub published: bool,
    pub tags: Option<String>,
    pub created_at: Option<String>,
}

#[cfg(feature = "uuid")]
#[PostgresTable]
pub struct Post {
    #[column(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    pub title: String,
    pub content: Option<String>,
    #[column(references = Complex::id)]
    pub author_id: Option<Uuid>,
    pub published: bool,
    pub tags: Option<String>,
    pub created_at: Option<String>,
}

// Categories table
#[PostgresTable]
pub struct Category {
    #[column(serial, primary)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

// Junction table for many-to-many
#[cfg(not(feature = "uuid"))]
#[PostgresTable]
pub struct PostCategory {
    #[column(references = Post::id)]
    pub post_id: i32,
    #[column(references = Category::id)]
    pub category_id: i32,
}

#[cfg(feature = "uuid")]
#[PostgresTable]
pub struct PostCategory {
    #[column(references = Post::id)]
    pub post_id: Uuid,
    #[column(references = Category::id)]
    pub category_id: i32,
}

// Comments table for chained FK join testing
#[cfg(not(feature = "uuid"))]
#[PostgresTable]
pub struct Comment {
    #[column(serial, primary)]
    pub id: i32,
    pub body: String,
    #[column(references = Post::id)]
    pub post_id: i32,
}

#[cfg(feature = "uuid")]
#[PostgresTable]
pub struct Comment {
    #[column(serial, primary)]
    pub id: i32,
    pub body: String,
    #[column(references = Post::id)]
    pub post_id: Uuid,
}

// Table with native PostgreSQL enum
#[PostgresTable]
pub struct Task {
    #[column(serial, primary)]
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    #[column(enum)]
    pub priority: Priority,
    #[column(enum)]
    pub status: PostStatus,
}

// Index definitions
#[PostgresIndex(unique)]
pub struct SimpleNameIdx(Simple::name);

#[PostgresIndex]
pub struct ComplexEmailIdx(Complex::email);

#[PostgresIndex]
pub struct PostAuthorIdx(Post::author_id);

// Schema definitions
#[derive(PostgresSchema)]
pub struct SimpleSchema {
    pub simple: Simple,
}

#[derive(PostgresSchema)]
pub struct ComplexSchema {
    pub role: Role,
    pub complex: Complex,
}

#[derive(PostgresSchema)]
pub struct SimpleComplexSchema {
    pub role: Role,
    pub simple: Simple,
    pub complex: Complex,
}

#[derive(PostgresSchema)]
pub struct ComplexPostSchema {
    pub role: Role,
    pub complex: Complex,
    pub post: Post,
}

#[derive(PostgresSchema)]
pub struct CategorySchema {
    pub category: Category,
}

#[derive(PostgresSchema)]
pub struct PostCategorySchema {
    pub role: Role,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
}

#[derive(PostgresSchema)]
pub struct TaskSchema {
    pub priority: Priority,
    pub status: PostStatus,
    pub task: Task,
}

#[derive(PostgresSchema)]
pub struct FullBlogSchema {
    pub role: Role,
    pub simple: Simple,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
    pub comment: Comment,
}

// Schema with indexes
#[derive(PostgresSchema)]
pub struct SimpleWithIndexSchema {
    pub simple: Simple,
    pub simple_name_idx: SimpleNameIdx,
}

#[derive(PostgresSchema)]
pub struct ComplexWithIndexSchema {
    pub role: Role,
    pub complex: Complex,
    pub complex_email_idx: ComplexEmailIdx,
}

// Helper table for testing UUID columns
#[cfg(feature = "uuid")]
#[PostgresTable(name = "pg_uuid_text")]
pub struct UuidText {
    #[column(serial, primary)]
    pub id: i32,
    pub uuid_col: Uuid, // Now maps to native UUID (not TEXT)
}

#[cfg(feature = "uuid")]
#[derive(PostgresSchema)]
pub struct UuidTextSchema {
    pub uuid_text: UuidText,
}
