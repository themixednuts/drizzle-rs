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
    title: Option<String>,
}

#[allow(clippy::type_complexity)]
fn check(mut db: Drizzle<mysql::Conn, Schema>, Schema { users, posts }: Schema) {
    let _: drizzle::Result<Vec<(SelectUsers, SelectPosts)>> = db
        .select(())
        .from(users)
        .inner_join((posts, eq(posts.author_id, users.id)))
        .all();
    let _: drizzle::Result<Vec<(SelectUsers, Option<SelectPosts>)>> = db
        .select(())
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .all();
    let _: drizzle::Result<Vec<(Option<SelectUsers>, SelectPosts)>> = db
        .select(())
        .from(users)
        .right_join((posts, eq(posts.author_id, users.id)))
        .all();

    let _: drizzle::Result<Vec<(String, Option<String>, Option<String>)>> = db
        .select((users.name, posts.title, posts.content))
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .all();
    let _: drizzle::Result<(Option<String>, Option<String>, String)> = db
        .select((users.name, users.email, posts.title))
        .from(users)
        .right_join((posts, eq(posts.author_id, users.id)))
        .get();

    let _: drizzle::Result<Vec<UserPost>> = db
        .select(UserPost::Select)
        .from(users)
        .left_join((posts, eq(posts.author_id, users.id)))
        .all();
}

fn main() {
    let _ = check;
}
