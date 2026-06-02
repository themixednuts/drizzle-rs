use drizzle::sqlite::prelude::*;

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
