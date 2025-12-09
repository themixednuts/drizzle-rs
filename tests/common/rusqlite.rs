#![cfg(feature = "rusqlite")]

use crate::common::schema::sqlite::{Category, Complex, Post, PostCategory, Role, Simple};
use ::rusqlite::Connection;
use rand::seq::IndexedRandom;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "rusqlite")]
pub fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    create_tables(&conn);
    // seed(&conn, 10, rand::random_range(0..=1000));
    conn
}

#[cfg(feature = "rusqlite")]
fn create_tables(conn: &Connection) {
    // Simple table

    use drizzle_core::SQLSchema;

    conn.execute(Simple::new().sql().sql().as_str(), [])
        .expect("Failed to create simple table");

    conn.execute(Complex::new().sql().sql().as_str(), [])
        .expect("Failed to create complex table");

    // Posts table for joins
    conn.execute(Post::new().sql().sql().as_str(), [])
        .expect("Failed to create posts table");

    // Categories for many-to-many testing
    conn.execute(Category::new().sql().sql().as_str(), [])
        .expect("Failed to create categories table");

    // Junction table
    conn.execute(PostCategory::new().sql().sql().as_str(), [])
        .expect("Failed to create post_categories table");
}

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub fn seed(conn: &Connection, rows: usize, rng_seed: u64) {
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // --- Simple table ---
    let simple_names = [
        "John", "Alice", "Thomas", "Appa", "Sarah", "Mike", "Laura", "Ethan",
    ];
    for _ in 0..rows {
        let name = simple_names.choose(&mut rng).unwrap();
        #[cfg(feature = "rusqlite")]
        conn.execute("INSERT INTO simple (name) VALUES (?1)", [&name])
            .expect("Failed to insert into simple");
    }

    // --- Complex table ---
    #[cfg(feature = "uuid")]
    let mut complex_ids: Vec<Uuid> = Vec::new();
    #[cfg(not(feature = "uuid"))]
    let mut complex_ids: Vec<i64> = Vec::new();

    for i in 0..rows {
        #[cfg(feature = "uuid")]
        let id = Uuid::new_v4();
        #[cfg(not(feature = "uuid"))]
        let id = i as i64 + 1;

        complex_ids.push(id);

        let name = format!("User{}", rng.random_range(1..=1000));
        let email: Option<String> = if rng.random_bool(0.7) {
            Some(format!("{}@example.com", name.to_lowercase()))
        } else {
            None
        };
        let age = if rng.random_bool(0.5) {
            Some(rng.random_range(18..=70))
        } else {
            None
        };
        let score = if rng.random_bool(0.5) {
            Some(rng.random_range(0.0..100.0))
        } else {
            None
        };
        let active = rng.random_bool(0.5);
        let bytes = vec![
            rng.random_range(0u8..=255),
            rng.random_range(0u8..=255),
            rng.random_range(0u8..=255),
            rng.random_range(0u8..=255),
        ];

        #[cfg(all(feature = "uuid", feature = "rusqlite"))]
        conn.execute(
            r#"
            INSERT INTO complex (
                id, name, email, age, score, active, role, description, data_blob, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            rusqlite::params![
                id.as_bytes(),
                name,
                email,
                age,
                score,
                active,
                if rng.random_bool(0.3) {
                    Role::Admin
                } else {
                    Role::User
                },
                "Generated user",
                bytes,
                "2025-08-11T12:00:00Z",
            ],
        )
        .unwrap();

        let bytes: Vec<u8> = vec![rng.random_range(0..=255), rng.random_range(0..=255)];

        #[cfg(all(not(feature = "uuid"), feature = "rusqlite"))]
        conn.execute(
            r#"
            INSERT INTO complex (
                id, name, email, age, score, active, description, data_blob, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            rusqlite::params![
                id,
                name,
                email,
                age,
                score,
                active,
                Some("Generated user"),
                Some(bytes),
                Some("2025-08-11T12:00:00Z"),
            ],
        )
        .unwrap();
    }

    // --- Categories ---
    let categories = [
        ("Technology", "Tech related posts"),
        ("Lifestyle", "Lifestyle related posts"),
        ("Travel", "Travel experiences"),
        ("Food", "Food and recipes"),
    ];
    for &(name, desc) in &categories {
        #[cfg(feature = "rusqlite")]
        conn.execute(
            "INSERT INTO categories (name, description) VALUES (?1, ?2)",
            rusqlite::params![name, desc],
        )
        .unwrap();
    }

    // --- Posts ---
    for i in 0..rows {
        let title = format!("Post {}", i + 1);
        let content: Option<String> = if rng.random_bool(0.8) {
            Some(format!("This is the content of post {}", i + 1))
        } else {
            None
        };
        let published = rng.random_bool(0.5);
        let created_at = format!("2025-08-11T{:02}:00:00Z", rng.random_range(0..24));

        #[cfg(all(feature = "uuid", feature = "rusqlite"))]
        {
            let author_id = complex_ids.choose(&mut rng).unwrap();
            conn.execute(
                "INSERT INTO posts (title, content, author_id, published, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![title, content, Some(author_id.as_bytes().to_vec()), published, created_at],
            ).unwrap();
        }

        #[cfg(all(not(feature = "uuid"), feature = "rusqlite"))]
        {
            let author_id = complex_ids.choose(&mut rng).unwrap();
            conn.execute(
                "INSERT INTO posts (title, content, author_id, published, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![title, content, Some(author_id), published, created_at],
            ).unwrap();
        }
    }

    // --- PostCategories (junction table) ---
    let category_ids: Vec<i32> = (1..=(categories.len() as i32)).collect();
    for post_id in 1..=rows as i32 {
        let num_cats = rng.random_range(1..=category_ids.len());
        for &cat_id in category_ids.choose_multiple(&mut rng, num_cats) {
            #[cfg(feature = "rusqlite")]
            conn.execute(
                "INSERT INTO post_categories (post_id, category_id) VALUES (?1, ?2)",
                rusqlite::params![post_id, cat_id],
            )
            .unwrap();
        }
    }
}
