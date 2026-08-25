#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::sqlite::prelude::*;

// Define a simple table for testing indexes
#[SQLiteTable]
struct User {
    #[column(PRIMARY)]
    id: i32,
    email: String,
    username: String,
}

#[SQLiteIndex(unique)]
struct UserEmailUsernameIdx(User::email, User::username);

#[SQLiteIndex]
struct UserIdx(User::id);

#[SQLiteIndex(unique, where = "username <> ''")]
struct ActiveUserIdx(User::username);

#[derive(SQLiteSchema)]
struct PartialIndexSchema {
    user: User,
    active_user_idx: ActiveUserIdx,
}

#[test]
fn test_index() {
    let idx = UserIdx::new();
    let sql = idx.to_sql().sql();

    // DDL-based SQL format uses backticks and includes semicolon
    assert_eq!(sql, "CREATE INDEX `user_idx` ON `user`(`id`);");
}

#[test]
fn test_unique_index() {
    let idx = UserEmailUsernameIdx::new();
    let sql = idx.to_sql().sql();

    // DDL-based SQL format uses backticks and includes semicolon
    assert_eq!(
        sql,
        "CREATE UNIQUE INDEX `user_email_username_idx` ON `user`(`email`, `username`);"
    );
}

#[test]
fn test_partial_index() {
    let sql = ActiveUserIdx::new().to_sql().sql();

    assert_eq!(
        sql,
        "CREATE UNIQUE INDEX `active_user_idx` ON `user`(`username`) WHERE username <> '';"
    );
    assert_eq!(
        ActiveUserIdx::ddl_sql(),
        "CREATE UNIQUE INDEX \"active_user_idx\" ON \"user\" (\"username\") WHERE username <> ''"
    );
}

#[drizzle::test]
fn partial_unique_index_is_a_complete_conflict_target(db: &mut TestDb<PartialIndexSchema>) {
    let PartialIndexSchema {
        user,
        active_user_idx,
    } = schema;

    db.insert(user)
        .values([InsertUser::new("first@example.com", "active")])
        .execute();
    db.insert(user)
        .values([InsertUser::new("second@example.com", "active")])
        .on_conflict(active_user_idx)
        .do_nothing()
        .execute();

    let rows: Vec<SelectUser> = db.select(()).from(user).all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].email, "first@example.com");
}
