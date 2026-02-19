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
    #[column(references = User::id)]
    user_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
    post: Post,
}

#[derive(SQLiteFromRow)]
struct UserPostRow {
    #[column(User::id)]
    user_id: i32,
    #[column(Post::id)]
    post_id: i32,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, schema) = Drizzle::new(conn, Schema::default());
    let Schema { user, post: _ } = schema;

    let _rows: Vec<UserPostRow> = db.select(UserPostRow::Select).from(user).all().unwrap();
}
