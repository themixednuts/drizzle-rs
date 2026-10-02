use drizzle::core::expr::asc;
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

fn check(mut db: Drizzle<mysql::Conn, Schema>, Schema { users, posts }: Schema) {
    // `posts` is never joined.
    let _rows: drizzle::Result<Vec<u64>> =
        db.select(users.id).from(users).order_by(asc(posts.title)).all();
}

fn main() {
    let _ = check;
}
