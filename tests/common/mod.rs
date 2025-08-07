use drizzle_rs::prelude::*;
use rusqlite::Connection;

// JSON struct types for testing JSON serialization features
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct UserMetadata {
    pub preferences: Vec<String>,
    pub last_login: Option<String>,
    pub theme: String,
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct UserConfig {
    pub notifications: bool,
    pub language: String,
    pub settings: std::collections::HashMap<String, String>,
}

pub fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    create_tables(&conn);
    conn
}

fn create_tables(conn: &Connection) {
    // Simple table
    conn.execute(Simple::SQL, [])
        .expect("Failed to create simple table");

    conn.execute(Complex::SQL, [])
        .expect("Failed to create complex table");

    // Posts table for joins
    conn.execute(Post::SQL, [])
        .expect("Failed to create posts table");

    // Categories for many-to-many testing
    conn.execute(Category::SQL, [])
        .expect("Failed to create categories table");

    // Junction table
    conn.execute(PostCategory::SQL, [])
        .expect("Failed to create post_categories table");
}

// Simple type for basic testing
#[SQLiteTable(name = "simple")]
pub struct Simple {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub name: String,
}

// Complex type testing all features
#[SQLiteTable(name = "complex")]
pub struct Complex {
    // Use UUID as primary key (requires uuid feature)
    #[cfg(feature = "uuid")]
    #[text(primary, default_fn = Uuid::new_v4().into )]
    pub id: String,
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

    // Text field for regular text storage
    #[text]
    pub description: Option<String>,

    // JSON stored as text (serde feature)
    #[cfg(feature = "serde")]
    #[text(json)]
    pub metadata: Option<UserMetadata>,

    // JSON stored as blob (serde feature)
    #[cfg(feature = "serde")]
    #[blob(json)]
    pub config: Option<UserConfig>,

    // Raw blob storage
    #[blob]
    pub data_blob: Option<Vec<u8>>,

    #[text]
    pub created_at: Option<String>,
}

#[SQLiteTable(name = "posts")]
pub struct Post {
    #[integer(primary)]
    pub id: i32,
    #[text]
    pub title: String,
    #[text]
    pub content: Option<String>,
    #[integer]
    pub author_id: Option<i32>,
    #[boolean]
    pub published: bool,
    #[cfg(feature = "serde")]
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
