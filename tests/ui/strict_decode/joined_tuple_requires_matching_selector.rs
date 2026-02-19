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
    title: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    user: User,
    post: Post,
}

#[derive(SQLiteFromRow)]
#[from(User)]
struct UserRow {
    id: i32,
    name: String,
}

#[derive(SQLiteFromRow)]
#[from(Post)]
struct PostRow {
    id: i32,
    user_id: i32,
    title: String,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, schema) = Drizzle::new(conn, Schema::default());
    let Schema { user, post } = schema;

    let _rows: Vec<(UserRow, PostRow)> = db
        .select(UserRow::Select)
        .from(user)
        .left_join(post)
        .all()
        .unwrap();
}
