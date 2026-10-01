use drizzle::core::expr::*;
use drizzle::core::{SQLColumn, ToSQL, asc, desc};
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
pub struct Users {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub age: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub title: String,
    pub content: Option<String>,
    #[column(references = Users::id)]
    pub author_id: i64,
}

#[SQLiteTable]
pub struct Comments {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub body: String,
    #[column(references = Posts::id)]
    pub post_id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
    pub comments: Comments,
}

#[allow(dead_code)]
#[derive(SQLiteFromRow, Debug)]
#[from(Users)]
struct UserWithPost {
    name: String,
    #[column(Posts::title)]
    title: Option<String>,
}

fn main() -> drizzle::Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    let (db, Schema { users, posts, comments }) = Drizzle::new(conn);
    db.create()?;

    // Scene: insert
    db.insert(users)
        .values([
            InsertUsers::new("Ada", 36).with_email("ada@lovelace.dev"),
            InsertUsers::new("Linus", 28).with_email("linus@kernel.org"),
        ])
        .execute()?;
    db.insert(posts)
        .values([
            InsertPosts::new("Hello, drizzle", 1).with_content("first!"),
            InsertPosts::new("Zero-cost SQL", 1).with_content("..."),
        ])
        .execute()?;
    db.insert(comments).value(InsertComments::new("nice", 1)).execute()?;

    // Scene: select builder
    let q = db
        .select((users.name, users.email))
        .from(users)
        .r#where(gt(users.age, 21))
        .order_by(desc(users.age))
        .limit(10);
    println!("SELECT  => {}", q.to_sql().sql());
    let adults: Vec<(String, Option<String>)> = q.all()?;
    println!("         {adults:?}");

    // Scene: aggregates / group by
    let q = db
        .select((users.name, alias(count(posts.id), "posts")))
        .from(users)
        .left_join(posts)
        .group_by(users.name)
        .having(gt(count(posts.id), 0))
        .order_by(asc(users.name));
    println!("GROUP   => {}", q.to_sql().sql());
    let per_user: Vec<(String, i64)> = q.all()?;
    println!("         {per_user:?}");

    // Scene: left join -> Option
    let q = db.select(UserWithPost::Select).from(users).left_join(posts);
    println!("JOIN    => {}", q.to_sql().sql());
    let rows: Vec<UserWithPost> = q.all()?;
    println!("         {rows:?}");

    // Scene: dynamic filters
    for search in [Some("Ada"), None] {
        let by_name = search.map(|n| eq(users.name, n));
        let q = db
            .select(())
            .from(users)
            .r#where((gt(users.age, 18), by_name));
        println!("DYN {search:?} => {}", q.to_sql().sql());
        let _: Vec<SelectUsers> = q.all()?;
    }

    // Scene: update / delete
    let q = db
        .update(users)
        .set(UpdateUsers::default().with_age(37))
        .r#where(eq(users.name, "Ada"));
    println!("UPDATE  => {}", q.to_sql().sql());
    q.execute()?;

    // Scene: relational query API
    let feed = db
        .query(users)
        .with(users.posts().with(posts.comments()))
        .find_many()?;
    for u in &feed {
        println!("QUERY   => {} has {} posts", u.name, u.posts.len());
        for p in &u.posts {
            println!("           {:?} ({} comments)", p.title, p.comments.len());
        }
    }

    // Scene: prepared + typed placeholder
    let name = users.name.placeholder("name");
    let find = db
        .select(())
        .from(users)
        .r#where(eq(users.name, name))
        .prepare();
    let ada: Vec<SelectUsers> = find.all(db.conn(), [name.bind("Ada")])?;
    println!("PREP    => {} row(s)", ada.len());

    // Scene: same question, flat rows vs nested tree
    let flat: Vec<(SelectUsers, Option<SelectPosts>)> =
        db.select(()).from(users).left_join(posts).all()?;
    println!("FLAT    => {} rows", flat.len());

    // Scene: query API — filter, order, pick columns
    let ada = db
        .query(users)
        .with(users.posts())
        .r#where(eq(users.name, "Ada"))
        .find_first()?;
    if let Some(ada) = ada {
        println!("FIRST   => {} / {} posts", ada.name, ada.posts.len());
    }
    let slim = db
        .query(users)
        .columns(users.columns().name().email())
        .order_by(asc(users.name))
        .find_many()?;
    println!("COLUMNS => {:?}", slim.iter().map(|u| u.name.clone()).collect::<Vec<_>>());
    let authored = db.query(posts).with(posts.author()).find_many()?;
    println!("AUTHOR  => {} by {}", authored[0].title, authored[0].author.name);

    Ok(())
}
