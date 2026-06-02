mod schema;

use drizzle::core::expr::*;
use drizzle::migrations::Tracking;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

use schema::{
    InsertComments, InsertPosts, InsertUsers, Posts, QueryUsersPosts, Schema, SelectUsers,
    UpdateUsers, Users,
};

fn main() -> drizzle::Result<()> {
    let conn = rusqlite::Connection::open_in_memory().expect("open sqlite");
    let (
        db,
        Schema {
            users,
            posts,
            comments,
        },
    ) = Drizzle::new(conn, Schema::new());

    let migrations = drizzle::include_migrations!("./drizzle");
    db.migrate(&migrations, Tracking::SQLITE)?;

    seed(&db, users, posts, comments)?;

    println!("--- select ---");
    let rows: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .r#where(gt(users.age, 25))
        .order_by(asc(users.name))
        .all()?;
    for u in &rows {
        println!("{} ({})", u.name, u.age);
    }

    println!("--- insert ---");
    db.insert(users)
        .value(InsertUsers::new("Sam", 22).with_email("sam@example.com"))
        .execute()?;

    println!("--- update ---");
    db.update(users)
        .set(UpdateUsers::default().with_age(27))
        .r#where(eq(users.id, 1))
        .execute()?;

    println!("--- join ---");
    #[derive(SQLiteFromRow)]
    #[from(Users)]
    struct UserPost {
        name: String,
        #[column(Posts::title)]
        post_title: Option<String>,
    }
    for row in db
        .select(UserPost::Select)
        .from(users)
        .left_join((posts, eq(users.id, posts.author_id)))
        .all()?
    {
        println!(
            "{} | {}",
            row.name,
            row.post_title.as_deref().unwrap_or("(no post)")
        );
    }

    println!("--- relations ---");
    let loaded = db.query(users).with(users.posts()).find_many()?;
    for u in &loaded {
        println!("{}: {} posts", u.name, u.posts().len());
    }

    Ok(())
}

fn seed(
    db: &Drizzle<Schema>,
    users: Users,
    posts: Posts,
    comments: schema::Comments,
) -> drizzle::Result<()> {
    db.insert(users)
        .values([
            InsertUsers::new("Alex Smith", 26).with_email("alex@example.com"),
            InsertUsers::new("Jordan Lee", 30).with_email("jordan@example.com"),
            InsertUsers::new("Alice", 28).with_email("alice@example.com"),
            InsertUsers::new("Bob", 32).with_email("bob@example.com"),
        ])
        .execute()?;

    db.insert(posts)
        .values([
            InsertPosts::new("Hello", 1).with_content("first post"),
            InsertPosts::new("World", 1).with_content("second post"),
        ])
        .execute()?;

    db.insert(comments)
        .value(InsertComments::new("nice post", 1))
        .execute()?;

    Ok(())
}
