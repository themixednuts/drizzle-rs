mod schema;
#[cfg(feature = "turso")]
use drizzle::sqlite::prelude::*;

#[cfg(feature = "turso")]
use turso::Builder;

#[cfg(all(feature = "turso", feature = "uuid"))]
use uuid::Uuid;

#[cfg(feature = "turso")]
use crate::schema::{InsertPosts, InsertUsers, Posts, Schema, SelectPosts, SelectUsers, Users};

#[cfg(feature = "turso")]
#[tokio::main]
async fn main() {
    use drizzle::core::expr::eq;

    let db_builder = Builder::new_local(":memory:")
        .build()
        .await
        .expect("create in-memory database");
    let conn = db_builder.connect().expect("connect to database");
    let (db, Schema { users, posts }) = drizzle::sqlite::turso::Drizzle::new(conn, Schema::new());
    db.create().await.expect("create tables");

    #[cfg(feature = "uuid")]
    let id = Uuid::new_v4();
    #[cfg(not(feature = "uuid"))]
    let id = 1;

    db.insert(users)
        .values([InsertUsers::new("Alex Smith", 26u64).with_id(id)])
        .execute()
        .await
        .expect("insert user");

    db.insert(posts)
        .values([InsertPosts::new(id).with_context("just testing")])
        .execute()
        .await
        .expect("insert post");

    let user_rows: Vec<SelectUsers> = db.select(()).from(users).all().await.expect("select users");
    let post_rows: Vec<SelectPosts> = db.select(()).from(posts).all().await.expect("select posts");

    assert_eq!(user_rows.len(), 1);
    assert_eq!(user_rows[0].name, "Alex Smith");
    assert_eq!(post_rows.len(), 1);
    assert_eq!(post_rows[0].context, Some("just testing".to_string()));

    println!("Users: {:?}", user_rows);
    println!("Posts: {:?}", post_rows);

    #[derive(SQLiteFromRow, Default, Debug)]
    struct JoinedResult {
        #[cfg(feature = "uuid")]
        #[column(Users::id)]
        id: Uuid,
        #[cfg(not(feature = "uuid"))]
        #[column(Users::id)]
        id: i64,
        #[cfg(feature = "uuid")]
        #[column(Posts::id)]
        post_id: Uuid,
        #[cfg(not(feature = "uuid"))]
        #[column(Posts::id)]
        post_id: i64,
        name: String,
        age: u64,
    }

    let row: JoinedResult = db
        .select(JoinedResult::default())
        .from(users)
        .left_join(posts)
        .get()
        .await
        .expect("select users on posts.user_id");

    assert_eq!(row.id, id);
    assert_eq!(row.name, "Alex Smith");
    assert_eq!(row.age, 26);
    #[cfg(feature = "uuid")]
    assert!(!row.post_id.is_nil());
    #[cfg(not(feature = "uuid"))]
    assert_eq!(row.post_id, 1);
}

#[cfg(not(feature = "turso"))]
fn main() {
    println!("turso feature not enabled");
}
