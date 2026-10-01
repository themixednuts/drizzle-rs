use drizzle::core::expr::count;
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
    title: Option<String>,
    #[column(Posts::content)]
    content: Option<String>,
}

#[derive(SQLiteFromRow)]
struct NameTitle {
    name: String,
    title: Option<String>,
}

#[derive(SQLiteFromRow)]
struct UserLite {
    id: i32,
    name: String,
    email: Option<String>,
    age: i32,
}

#[allow(clippy::type_complexity)]
fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { users, posts }) = Drizzle::new(conn);

    // Single-table `SELECT *` still decodes into any row type.
    let _: drizzle::Result<Vec<SelectUsers>> = db.select(()).from(users).all();
    let _: drizzle::Result<Vec<UserLite>> = db.select(()).from(users).all();

    // Joined `SELECT *` keeps the inferred shape; outer-joined halves are `Option`.
    let _: drizzle::Result<Vec<(SelectUsers, SelectPosts)>> =
        db.select(()).from(users).inner_join(posts).all();
    let _: drizzle::Result<Vec<(SelectUsers, Option<SelectPosts>)>> =
        db.select(()).from(users).left_join(posts).all();
    let _: drizzle::Result<Vec<(Option<SelectUsers>, SelectPosts)>> =
        db.select(()).from(users).right_join(posts).all();
    let _: drizzle::Result<Vec<(Option<SelectUsers>, Option<SelectPosts>)>> =
        db.select(()).from(users).full_join(posts).all();
    // Widening a non-null half to `Option` is always allowed.
    let _: drizzle::Result<Vec<(SelectUsers, Option<SelectPosts>)>> =
        db.select(()).from(users).inner_join(posts).all();

    // Explicit columns: outer-joined columns decode as `Option`, including
    // columns that were already nullable (no `Option<Option<_>>`).
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
    let _: drizzle::Result<Vec<(Option<i32>, i32)>> = db
        .select((posts.id, users.id))
        .from(users)
        .left_join(posts)
        .all();

    // Typed expressions are not tied to a source and keep their type.
    let _: drizzle::Result<Vec<(String, i64)>> = db
        .select((users.name, count(posts.id)))
        .from(users)
        .left_join(posts)
        .group_by(users.id)
        .all();

    // FromRow targets follow the same rules.
    let _: drizzle::Result<Vec<UserPost>> =
        db.select(UserPost::Select).from(users).left_join(posts).all();
    let _: drizzle::Result<Vec<NameTitle>> =
        db.select((users.name, posts.title)).from(users).left_join(posts).all();
}
