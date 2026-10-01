use drizzle::core::expr::eq;
use drizzle::mysql::mysql_sync::Drizzle;
use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Users {
    #[column(PRIMARY)]
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
    #[column(VARCHAR(255))]
    email: Option<String>,
}

#[MySQLTable]
struct Posts {
    #[column(PRIMARY)]
    id: u64,
    #[column(VARCHAR(255))]
    title: String,
    #[column(VARCHAR(255))]
    content: Option<String>,
    author_id: u64,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

#[derive(MySQLFromRow)]
struct UserPost {
    #[column(Users::name)]
    name: String,
    #[column(Posts::title)]
    title: String,
}

fn check(mut db: Drizzle<mysql::Conn, Schema>, Schema { users, posts }: Schema) {
    let _rows: drizzle::Result<Vec<UserPost>> = db
        .select(UserPost::Select)
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .all();
}

fn main() {
    let _ = check;
}
