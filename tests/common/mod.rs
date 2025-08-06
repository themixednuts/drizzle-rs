use drizzle_rs::core::IsInSchema;
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

// Define a test schema that includes all our tables
pub struct TestSchema;

// Implement IsInSchema for all our test types
impl IsInSchema<TestSchema> for Simple {}
impl IsInSchema<TestSchema> for Complex {}
impl IsInSchema<TestSchema> for Post {}
impl IsInSchema<TestSchema> for Category {}
impl IsInSchema<TestSchema> for PostCategory {}

pub fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    create_tables(&conn);
    conn
}

fn create_tables(conn: &Connection) {
    // Simple table
    conn.execute(
        "CREATE TABLE simple (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
        [],
    )
    .expect("Failed to create simple table");

    // Complex table with all field types and constraints
    #[cfg(feature = "uuid")]
    conn.execute(
        "CREATE TABLE complex (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER,
            score REAL,
            active BOOLEAN DEFAULT 1,
            description TEXT,
            metadata TEXT,
            config BLOB,
            data_blob BLOB,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name, email)
        )",
        [],
    )
    .expect("Failed to create complex table");

    #[cfg(not(feature = "uuid"))]
    conn.execute(
        "CREATE TABLE complex (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER,
            score REAL,
            active BOOLEAN DEFAULT 1,
            description TEXT,
            metadata TEXT,
            config BLOB,
            data_blob BLOB,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name, email)
        )",
        [],
    )
    .expect("Failed to create complex table");

    // Posts table for joins
    conn.execute(
        "CREATE TABLE posts (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT,
            author_id INTEGER,
            published BOOLEAN DEFAULT 0,
            tags TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (author_id) REFERENCES complex (id)
        )",
        [],
    )
    .expect("Failed to create posts table");

    // Categories for many-to-many testing
    conn.execute(
        "CREATE TABLE categories (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT
        )",
        [],
    )
    .expect("Failed to create categories table");

    // Junction table
    conn.execute(
        "CREATE TABLE post_categories (
            post_id INTEGER,
            category_id INTEGER,
            PRIMARY KEY (post_id, category_id),
            FOREIGN KEY (post_id) REFERENCES posts (id),
            FOREIGN KEY (category_id) REFERENCES categories (id)
        )",
        [],
    )
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
    #[text(primary)]
    pub id: uuid::Uuid,

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
