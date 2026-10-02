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
    // `posts.title` is NOT NULL in the table, but NULL after a LEFT JOIN miss.
    let _rows: drizzle::Result<Vec<(String, String)>> =
        db.select((users.name, posts.title)).from(users).left_join(posts).all();
}

fn main() {
    let _ = check;
}
