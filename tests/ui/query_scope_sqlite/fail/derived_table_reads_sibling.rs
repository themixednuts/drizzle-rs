use drizzle::core::expr::*;
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
    #[column(references = Users::id)]
    author_id: i32,
    views: i32,
}

#[SQLiteTable]
struct Tags {
    #[column(primary)]
    id: i32,
    #[column(references = Posts::id)]
    post_id: i32,
    label: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    posts: Posts,
    tags: Tags,
}

fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let (db, Schema { users, posts, tags }) = Drizzle::new(conn);
    let qb = drizzle::sqlite::builder::QueryBuilder::new::<Schema>();
    let _ = (&users, &posts, &tags, &qb);
    // A derived table in FROM cannot read the sources joined beside it.
    struct RecentPosts;
    impl drizzle::core::Tag for RecentPosts {
        const NAME: &'static str = "recent_posts";
    }
    let recent = qb
        .select((posts.id, posts.author_id))
        .from(posts)
        .r#where(eq(posts.author_id, users.id))
        .alias(RecentPosts);
    let _rows: drizzle::Result<Vec<i32>> = db
        .select(users.id)
        .from(users)
        .join((recent, eq(users.id, users.id)))
        .all();
}
