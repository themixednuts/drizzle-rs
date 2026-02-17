#[cfg(feature = "turso")]
mod schema;

#[cfg(feature = "turso")]
#[tokio::main]
async fn main() {
    use drizzle::sqlite::prelude::*;
    use turso::Builder;
    #[cfg(feature = "uuid")]
    use uuid::Uuid;

    use crate::schema::{InsertPosts, InsertUsers, Posts, Schema, SelectPosts, SelectUsers, Users};

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
        .get_as()
        .await
        .expect("select users on posts.user_id");

    assert_eq!(row.id, id);
    assert_eq!(row.name, "Alex Smith");
    assert_eq!(row.age, 26);
    #[cfg(feature = "uuid")]
    assert!(!row.post_id.is_nil());
    #[cfg(not(feature = "uuid"))]
    assert_eq!(row.post_id, 1);

    // Clone the Drizzle handle and move it into a spawned task.
    // The clone is cheap — the underlying connection is shared via Arc.
    let db_clone = db.clone();
    let handle = tokio::spawn(async move {
        db_clone
            .insert(users)
            .values([InsertUsers::new("Bob", 25u64)])
            .execute()
            .await
            .expect("insert user from spawned task");
    });

    handle.await.expect("spawned task completed");

    let all_users: Vec<SelectUsers> = db.select(()).from(users).all().await.expect("select users");

    assert_eq!(all_users.len(), 2);
    println!("After spawn: {:?}", all_users);
}

#[cfg(not(feature = "turso"))]
fn main() {
    println!("turso feature not enabled — run with: cargo run --example turso --features turso");
}
