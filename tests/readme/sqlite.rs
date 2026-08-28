use drizzle::sqlite::{prelude::*, rusqlite::Drizzle};

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

pub fn database() -> drizzle::Result<(Drizzle<Schema>, Schema)> {
    let connection = rusqlite::Connection::open_in_memory()?;
    let (
        db,
        schema @ Schema {
            users,
            posts,
            comments,
        },
    ) = Drizzle::new(connection, Schema::new());

    db.create()?;
    db.insert(users)
        .values([
            InsertUsers::new("Alex Smith", 26).with_email("alex@example.com"),
            InsertUsers::new("Alice", 30).with_email("alice@example.com"),
            InsertUsers::new("Bob", 17).with_email("bob@example.com"),
        ])
        .execute()?;
    db.insert(posts)
        .values([
            InsertPosts::new("First post", 1).with_content("Hello"),
            InsertPosts::new("Second post", 1).with_content("More"),
        ])
        .execute()?;
    db.insert(comments)
        .value(InsertComments::new("Nice post", 1))
        .execute()?;

    Ok((db, schema))
}
