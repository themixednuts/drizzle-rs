// Shared schema, helpers, and macros for SQLite benchmarks.
// Included by both `sqlite.rs` and `sqlite_libsql.rs` via `include!`.

use drizzle::core::expr::{count, eq};
use drizzle::sqlite::prelude::*;

#[SQLiteTable(name = "bench_users")]
pub struct User {
    #[column(primary)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[SQLiteTable(name = "bench_posts")]
pub struct Post {
    #[column(primary)]
    pub id: i32,
    pub title: String,
    pub body: String,
    pub author_id: i32,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub user: User,
}

#[derive(SQLiteSchema)]
pub struct BlogSchema {
    pub user: User,
    pub post: Post,
}

macro_rules! users {
    ($n:expr) => {
        (0..$n).map(|i| InsertUser::new(format!("User {}", i), format!("user{}@x.dev", i)))
    };
}

macro_rules! posts {
    ($n:expr, $authors:expr) => {
        (0..$n).map(|i| {
            InsertPost::new(
                format!("Post {}", i),
                format!("Body {}", i),
                (i % $authors) + 1,
            )
        })
    };
}
