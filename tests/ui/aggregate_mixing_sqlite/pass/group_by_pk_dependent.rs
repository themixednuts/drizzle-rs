use drizzle::core::expr::{alias, count, eq};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
    email: String,
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
struct PkRow {
    id: i32,
    name: String,
    email: String,
    total: i64,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, post }) = Drizzle::new(conn, Schema::default());

    // GROUP BY the table's primary key functionally determines every other
    // column of that table (SQL:1999), so scalar columns pass without being
    // listed in GROUP BY.
    let _: drizzle::Result<Vec<PkRow>> = db
        .select((user.id, user.name, user.email, alias(count(()), "total")))
        .from(user)
        .group_by(user.id)
        .all();

    // Same shape across a join: base-table scalars + aggregates over the
    // joined table, grouped by the base table's primary key.
    let _: drizzle::Result<Vec<PkRow>> = db
        .select((
            user.id,
            user.name,
            user.email,
            alias(count(post.id), "total"),
        ))
        .from(user)
        .left_join((post, eq(user.id, post.user_id)))
        .group_by(user.id)
        .all();
}
