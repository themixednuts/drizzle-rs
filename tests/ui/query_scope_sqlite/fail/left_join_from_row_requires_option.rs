use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
    email: Option<String>,
    age: i32,
}

#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    title: String,
    content: Option<String>,
    #[column(references = Users::id)]
    author_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

#[derive(SQLiteFromRow)]
struct UserPost {
    #[column(Users::name)]
    name: String,
    #[column(Posts::title)]
    title: String,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { users, posts }) = Drizzle::new(conn);

    let _rows: drizzle::Result<Vec<UserPost>> =
        db.select(UserPost::Select).from(users).left_join(posts).all();
}
