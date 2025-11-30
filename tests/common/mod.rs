#![cfg(any(
    feature = "rusqlite",
    feature = "turso",
    feature = "libsql",
    feature = "postgres"
))]

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod rusqlite;
#[cfg(feature = "rusqlite")]
pub use rusqlite::*;

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod turso;
#[cfg(feature = "turso")]
pub use turso::setup_db;

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod libsql;
#[cfg(feature = "libsql")]
pub use libsql::*;

pub mod helpers;

use drizzle::prelude::*;

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
#[SQLiteTable]
pub struct Simple {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub name: String,
}
#[cfg(all(feature = "uuid", not(feature = "serde")))]
#[SQLiteTable]
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
#[SQLiteTable]
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
#[SQLiteTable]
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
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
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

#[cfg(not(feature = "uuid"))]
#[SQLiteTable(name = "post_categories")]
pub struct PostCategory {
    #[integer]
    pub post_id: i32,
    #[integer]
    pub category_id: i32,
}

#[cfg(feature = "uuid")]
#[SQLiteTable(name = "post_categories")]
pub struct PostCategory {
    #[blob]
    pub post_id: Uuid,
    #[integer]
    pub category_id: i32,
}

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema)]
pub struct SimpleComplexSchema {
    pub simple: Simple,
    pub complex: Complex,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLiteSchema)]
pub struct SimpleComplexSchema {
    pub simple: Simple,
    pub complex: Complex,
}

#[derive(SQLiteSchema, Debug)]
pub struct SimpleSchema {
    pub simple: Simple,
}

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema, Debug)]
pub struct ComplexSchema {
    pub complex: Complex,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLiteSchema)]
pub struct ComplexSchema {
    pub complex: Complex,
}

#[derive(SQLiteSchema)]
pub struct PostSchema {
    pub post: Post,
}

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[cfg(not(feature = "uuid"))]
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
    pub post_category: PostCategory,
}

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema)]
pub struct FullBlogSchema {
    pub simple: Simple,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
}

#[cfg(not(feature = "uuid"))]
#[derive(SQLiteSchema)]
pub struct FullBlogSchema {
    pub simple: Simple,
    pub complex: Complex,
    pub post: Post,
    pub category: Category,
    pub post_category: PostCategory,
}

// ============================================================================
// PostgreSQL-specific types and schemas
// ============================================================================

#[cfg(feature = "postgres")]
pub mod pg {
    use drizzle::prelude::*;

    #[cfg(feature = "uuid")]
    use uuid::Uuid;

    // JSON struct types for testing JSON serialization features
    #[cfg(feature = "serde")]
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
    pub struct PgUserMetadata {
        pub preferences: Vec<String>,
        pub last_login: Option<String>,
        pub theme: String,
    }

    #[cfg(feature = "serde")]
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
    pub struct PgUserConfig {
        pub notifications: bool,
        pub language: String,
        pub settings: std::collections::HashMap<String, String>,
    }

    /// PostgreSQL enum for user roles
    #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
    pub enum PgRole {
        #[default]
        User,
        Admin,
        Moderator,
    }

    /// PostgreSQL enum for post status
    #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
    pub enum PostStatus {
        #[default]
        Draft,
        Published,
        Archived,
    }

    /// PostgreSQL enum for priority levels
    #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
    pub enum Priority {
        Low = 1,
        #[default]
        Medium = 5,
        High = 10,
    }

    // Simple table for basic testing
    #[PostgresTable(name = "pg_simple")]
    pub struct PgSimple {
        #[serial(primary)]
        pub id: i32,
        #[text]
        pub name: String,
    }

    // Complex table with various PostgreSQL-specific types
    #[cfg(all(feature = "uuid", not(feature = "serde")))]
    #[PostgresTable(name = "pg_complex")]
    pub struct PgComplex {
        #[uuid(primary, default_fn = Uuid::new_v4)]
        pub id: Uuid,
        #[text]
        pub name: String,
        #[text]
        pub email: Option<String>,
        #[integer]
        pub age: Option<i32>,
        #[double_precision]
        pub score: Option<f64>,
        #[boolean]
        pub active: bool,
        #[text(enum)]
        pub role: PgRole,
        #[text]
        pub description: Option<String>,
        #[bytea]
        pub data_blob: Option<Vec<u8>>,
        #[text]
        pub created_at: Option<String>,
    }

    #[cfg(all(feature = "uuid", feature = "serde"))]
    #[PostgresTable(name = "pg_complex")]
    pub struct PgComplex {
        #[uuid(primary, default_fn = Uuid::new_v4)]
        pub id: Uuid,
        #[text]
        pub name: String,
        #[text]
        pub email: Option<String>,
        #[integer]
        pub age: Option<i32>,
        #[double_precision]
        pub score: Option<f64>,
        #[boolean]
        pub active: bool,
        #[text(enum)]
        pub role: PgRole,
        #[text]
        pub description: Option<String>,
        #[text(json)]
        pub metadata: Option<String>,
        #[text(json)]
        pub config: Option<String>,
        #[bytea]
        pub data_blob: Option<Vec<u8>>,
        #[text]
        pub created_at: Option<String>,
    }

    #[cfg(all(not(feature = "uuid"), feature = "serde"))]
    #[PostgresTable(name = "pg_complex")]
    pub struct PgComplex {
        #[serial(primary)]
        pub id: i32,
        #[text]
        pub name: String,
        #[text]
        pub email: Option<String>,
        #[integer]
        pub age: Option<i32>,
        #[double_precision]
        pub score: Option<f64>,
        #[boolean]
        pub active: bool,
        #[text(enum)]
        pub role: PgRole,
        #[text]
        pub description: Option<String>,
        #[text(json)]
        pub metadata: Option<String>,
        #[text(json)]
        pub config: Option<String>,
        #[bytea]
        pub data_blob: Option<Vec<u8>>,
        #[text]
        pub created_at: Option<String>,
    }

    #[cfg(all(not(feature = "uuid"), not(feature = "serde")))]
    #[PostgresTable(name = "pg_complex")]
    pub struct PgComplex {
        #[serial(primary)]
        pub id: i32,
        #[text]
        pub name: String,
        #[text]
        pub email: Option<String>,
        #[integer]
        pub age: Option<i32>,
        #[double_precision]
        pub score: Option<f64>,
        #[boolean]
        pub active: bool,
        #[text(enum)]
        pub role: PgRole,
        #[text]
        pub description: Option<String>,
        #[bytea]
        pub data_blob: Option<Vec<u8>>,
        #[text]
        pub created_at: Option<String>,
    }

    // Posts table for join testing
    #[cfg(not(feature = "uuid"))]
    #[PostgresTable(name = "pg_posts")]
    pub struct PgPost {
        #[serial(primary)]
        pub id: i32,
        #[text]
        pub title: String,
        #[text]
        pub content: Option<String>,
        #[integer(references = PgComplex::id)]
        pub author_id: Option<i32>,
        #[boolean]
        pub published: bool,
        #[text]
        pub tags: Option<String>,
        #[text]
        pub created_at: Option<String>,
    }

    #[cfg(feature = "uuid")]
    #[PostgresTable(name = "pg_posts")]
    pub struct PgPost {
        #[uuid(primary, default_fn = Uuid::new_v4)]
        pub id: Uuid,
        #[text]
        pub title: String,
        #[text]
        pub content: Option<String>,
        #[uuid(references = PgComplex::id)]
        pub author_id: Option<Uuid>,
        #[boolean]
        pub published: bool,
        #[text]
        pub tags: Option<String>,
        #[text]
        pub created_at: Option<String>,
    }

    // Categories table
    #[PostgresTable(name = "pg_categories")]
    pub struct PgCategory {
        #[serial(primary)]
        pub id: i32,
        #[text]
        pub name: String,
        #[text]
        pub description: Option<String>,
    }

    // Junction table for many-to-many
    #[cfg(not(feature = "uuid"))]
    #[PostgresTable(name = "pg_post_categories")]
    pub struct PgPostCategory {
        #[integer]
        pub post_id: i32,
        #[integer]
        pub category_id: i32,
    }

    #[cfg(feature = "uuid")]
    #[PostgresTable(name = "pg_post_categories")]
    pub struct PgPostCategory {
        #[uuid]
        pub post_id: Uuid,
        #[integer]
        pub category_id: i32,
    }

    // Table with native PostgreSQL enum
    #[PostgresTable(name = "pg_tasks")]
    pub struct PgTask {
        #[serial(primary)]
        pub id: i32,
        #[text]
        pub title: String,
        #[text]
        pub description: Option<String>,
        #[r#enum(Priority)]
        pub priority: Priority,
        #[r#enum(PostStatus)]
        pub status: PostStatus,
    }

    // Index definitions
    #[PostgresIndex(unique)]
    pub struct PgSimpleNameIdx(PgSimple::name);

    #[PostgresIndex]
    pub struct PgComplexEmailIdx(PgComplex::email);

    #[PostgresIndex]
    pub struct PgPostAuthorIdx(PgPost::author_id);

    // Schema definitions
    #[derive(PostgresSchema)]
    pub struct PgSimpleSchema {
        pub simple: PgSimple,
    }

    #[cfg(feature = "uuid")]
    #[derive(PostgresSchema)]
    pub struct PgComplexSchema {
        pub role: PgRole,
        pub complex: PgComplex,
    }

    #[cfg(not(feature = "uuid"))]
    #[derive(PostgresSchema)]
    pub struct PgComplexSchema {
        pub role: PgRole,
        pub complex: PgComplex,
    }

    #[derive(PostgresSchema)]
    pub struct PgSimpleComplexSchema {
        pub role: PgRole,
        pub simple: PgSimple,
        pub complex: PgComplex,
    }

    #[cfg(feature = "uuid")]
    #[derive(PostgresSchema)]
    pub struct PgComplexPostSchema {
        pub role: PgRole,
        pub complex: PgComplex,
        pub post: PgPost,
    }

    #[cfg(not(feature = "uuid"))]
    #[derive(PostgresSchema)]
    pub struct PgComplexPostSchema {
        pub role: PgRole,
        pub complex: PgComplex,
        pub post: PgPost,
    }

    #[derive(PostgresSchema)]
    pub struct PgCategorySchema {
        pub category: PgCategory,
    }

    #[derive(PostgresSchema)]
    pub struct PgPostCategorySchema {
        pub post_category: PgPostCategory,
    }

    #[derive(PostgresSchema)]
    pub struct PgTaskSchema {
        pub priority: Priority,
        pub status: PostStatus,
        pub task: PgTask,
    }

    #[cfg(feature = "uuid")]
    #[derive(PostgresSchema)]
    pub struct PgFullBlogSchema {
        pub role: PgRole,
        pub simple: PgSimple,
        pub complex: PgComplex,
        pub post: PgPost,
        pub category: PgCategory,
        pub post_category: PgPostCategory,
    }

    #[cfg(not(feature = "uuid"))]
    #[derive(PostgresSchema)]
    pub struct PgFullBlogSchema {
        pub role: PgRole,
        pub simple: PgSimple,
        pub complex: PgComplex,
        pub post: PgPost,
        pub category: PgCategory,
        pub post_category: PgPostCategory,
    }

    // Schema with indexes
    #[derive(PostgresSchema)]
    pub struct PgSimpleWithIndexSchema {
        pub simple: PgSimple,
        pub simple_name_idx: PgSimpleNameIdx,
    }

    #[cfg(feature = "uuid")]
    #[derive(PostgresSchema)]
    pub struct PgComplexWithIndexSchema {
        pub role: PgRole,
        pub complex: PgComplex,
        pub complex_email_idx: PgComplexEmailIdx,
    }

    #[cfg(not(feature = "uuid"))]
    #[derive(PostgresSchema)]
    pub struct PgComplexWithIndexSchema {
        pub role: PgRole,
        pub complex: PgComplex,
        pub complex_email_idx: PgComplexEmailIdx,
    }
}
