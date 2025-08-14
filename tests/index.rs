#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod common;
use drizzle_rs::prelude::*;

// Define a simple table for testing indexes
#[SQLiteTable]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    email: String,
    #[text]
    username: String,
}

#[SQLiteIndex(unique)]
struct UserEmailUsernameIdx(User::email, User::username);

#[SQLiteIndex]
struct UserIdx(User::id);

#[test]
fn test_index() {
    let idx = UserIdx::new();
    let sql = idx.to_sql().sql();

    assert_eq!(sql, r#"CREATE INDEX "user_idx" ON "User" (id)"#);
}

#[test]
fn test_unique_index() {
    let idx = UserEmailUsernameIdx::new();
    let sql = idx.to_sql().sql();

    assert_eq!(
        sql,
        r#"CREATE UNIQUE INDEX "user_email_username_idx" ON "User" (email, username)"#
    );
}
