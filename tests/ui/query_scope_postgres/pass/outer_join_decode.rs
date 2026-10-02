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

#[derive(PostgresFromRow)]
struct UserPost {
    #[column(Users::name)]
    name: String,
    #[column(Posts::title)]
    title: Option<String>,
}

#[derive(PostgresFromRow)]
struct NameTitle {
    name: String,
    title: Option<String>,
}

#[allow(clippy::type_complexity)]
fn check(mut db: Drizzle<Schema>, Schema { users, posts }: Schema) {
    let _: drizzle::Result<Vec<(SelectUsers, SelectPosts)>> =
        db.select(()).from(users).inner_join(posts).all();
    let _: drizzle::Result<Vec<(SelectUsers, Option<SelectPosts>)>> =
        db.select(()).from(users).left_join(posts).all();
    let _: drizzle::Result<Vec<(Option<SelectUsers>, SelectPosts)>> =
        db.select(()).from(users).right_join(posts).all();
    let _: drizzle::Result<Vec<(Option<SelectUsers>, Option<SelectPosts>)>> =
        db.select(()).from(users).full_join(posts).all();

    let _: drizzle::Result<Vec<(String, String)>> =
        db.select((users.name, posts.title)).from(users).inner_join(posts).all();
    let _: drizzle::Result<Vec<(String, Option<String>, Option<String>)>> = db
        .select((users.name, posts.title, posts.content))
        .from(users)
        .left_join(posts)
        .all();
    let _: drizzle::Result<(Option<String>, Option<String>, String)> = db
        .select((users.name, users.email, posts.title))
        .from(users)
        .right_join(posts)
        .get();
    let _: drizzle::Result<Vec<(Option<String>, Option<String>)>> =
        db.select((users.name, posts.title)).from(users).full_join(posts).all();

    let _: drizzle::Result<Vec<UserPost>> =
        db.select(UserPost::Select).from(users).left_join(posts).all();
    let _: drizzle::Result<Vec<NameTitle>> =
        db.select((users.name, posts.title)).from(users).left_join(posts).all();
}

fn main() {
    let _ = check;
}
