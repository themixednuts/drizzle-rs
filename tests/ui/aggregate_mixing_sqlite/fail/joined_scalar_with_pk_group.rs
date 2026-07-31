use drizzle::core::expr::{alias, count, eq};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

#[SQLiteTable]
struct Post {
    #[column(primary)]
    id: i32,
    user_id: i32,
    title: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
    post: Post,
}

#[derive(SQLiteFromRow)]
struct BadRow {
    name: String,
    title: String,
    total: i64,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, post }) = Drizzle::new(conn, Schema::default());

    // GROUP BY user's primary key covers user's columns, but `post.title`
    // belongs to the joined table and is NOT functionally dependent on
    // user.id — should fail.
    let _: drizzle::Result<Vec<BadRow>> = db
        .select((user.name, post.title, alias(count(()), "total")))
        .from(user)
        .left_join((post, eq(user.id, post.user_id)))
        .group_by(user.id)
        .all();
}
