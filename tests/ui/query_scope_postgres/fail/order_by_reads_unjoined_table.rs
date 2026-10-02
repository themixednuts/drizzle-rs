use drizzle::core::expr::asc;
use drizzle::postgres::prelude::*;
use drizzle::postgres::sync::Drizzle;

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
    email: Option<String>,
    age: i32,
}

#[PostgresTable]
struct Posts {
    #[column(primary)]
    id: i32,
    title: String,
    content: Option<String>,
    #[column(references = Users::id)]
    author_id: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

fn check(mut db: Drizzle<Schema>, Schema { users, posts }: Schema) {
    // `posts` is never joined.
    let _rows: drizzle::Result<Vec<i32>> =
        db.select(users.id).from(users).order_by(asc(posts.title)).all();
}

fn main() {
    let _ = check;
}
