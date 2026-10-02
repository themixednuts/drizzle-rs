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

fn check(mut db: Drizzle<mysql::Conn, Schema>, Schema { users, posts }: Schema) {
    // RIGHT JOIN makes the left-hand `users` columns nullable.
    let _row: drizzle::Result<(String, String)> = db
        .select((users.name, posts.title))
        .from(users)
        .right_join((posts, eq(posts.author_id, users.id)))
        .get();
}

fn main() {
    let _ = check;
}
