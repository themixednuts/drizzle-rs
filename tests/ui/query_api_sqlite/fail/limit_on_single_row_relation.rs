use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
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
    let (db, Schema { post, .. }) = Drizzle::new(conn);
    // `author` loads one row; a LIMIT or OFFSET could only drop or skip it.
    let _ = db.query(post).with(post.author().limit(1).offset(1));
}
