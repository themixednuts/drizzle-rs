#![cfg(feature = "rusqlite")]

use drizzle_rs::prelude::*;
use rusqlite::Connection;
use std::time::Instant;

#[SQLiteTable(name = "users")]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[text]
    email: String,
}

#[derive(Clone)]
pub struct Schema;

const CREATE_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT NOT NULL
    )
"#;

fn setup_raw_rusqlite() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(CREATE_TABLE_SQL, []).unwrap();

    // Insert test data
    for i in 0..1000 {
        conn.execute(
            "INSERT INTO users (name, email) VALUES (?1, ?2)",
            [format!("User {}", i), format!("user{}@example.com", i)],
        )
        .unwrap();
    }

    conn
}

fn setup_drizzle() -> (drizzle_rs::sqlite::Drizzle<Schema>, User) {
    let conn = Connection::open_in_memory().unwrap();
    let (db, users) = drizzle!(conn, User, Schema);
    db.execute(users.sql()).unwrap();

    // Insert test data
    let data: Vec<_> = (0..1000)
        .map(|i| InsertUser::new(format!("User {}", i), format!("user{}@example.com", i)))
        .collect();
    db.insert(users).values(data).execute().unwrap();

    (db, users)
}

#[test]
fn test_raw_rusqlite_select() {
    let conn = setup_raw_rusqlite();

    let start = Instant::now();

    // Perform the query multiple times to amplify the performance difference
    for _ in 0..100 {
        let mut stmt = conn
            .prepare(r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#)
            .unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap();

        let _results: Vec<_> = rows.collect();
    }

    let duration = start.elapsed();
    println!("Raw rusqlite took: {:?}", duration);
}

#[test]
fn test_drizzle_rs_select() {
    let (db, users) = setup_drizzle();

    let start = Instant::now();

    // Perform the query multiple times to amplify the performance difference
    for _ in 0..100 {
        let _results: Vec<SelectUser> = db.select(()).from(users).all().unwrap();
    }

    let duration = start.elapsed();
    println!("Drizzle-rs took: {:?}", duration);
}

#[test]
fn test_drizzle_rs_prepared_select() {
    let (db, users) = setup_drizzle();
    let prepared = db.select(()).from(users).prepare().into_owned();

    let start = Instant::now();

    // Perform the query multiple times to amplify the performance difference
    for _ in 0..100 {
        let _results: Vec<SelectUser> = prepared.all(db.conn(), []).unwrap();
    }

    let duration = start.elapsed();
    println!("Drizzle-rs prepared took: {:?}", duration);
}
