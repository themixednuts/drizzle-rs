use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    active: bool,
}

#[SQLiteTable(NAME = "posts")]
struct Post {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = User::id)]
    author_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
    post: Post,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { user, post }) = Drizzle::new(conn);
    // Every post has an author; filtering it would leave a post without one.
    let _ = db
        .query(post)
        .with(post.author().r#where(eq(user.active, true)));
}
