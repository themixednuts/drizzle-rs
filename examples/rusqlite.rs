mod schema;
use drizzle::sqlite::prelude::*;

#[cfg(feature = "rusqlite")]
use rusqlite::Connection;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "rusqlite")]
use crate::schema::{InsertPosts, InsertUsers, Posts, Schema, SelectPosts, SelectUsers, Users};

fn main() {
    #[cfg(not(feature = "rusqlite"))]
    {
        println!("rusqlite feature not enabled");
        return;
    }

    #[cfg(feature = "rusqlite")]
    {
        use drizzle::core::expr::eq;

        let conn = Connection::open_in_memory().expect("open connection");
        let (db, Schema { users, posts }) =
            drizzle::sqlite::rusqlite::Drizzle::new(conn, Schema::new());
        // create tables do not have IF NOT EXISTS so we can support migrations in the furture, so only do this on a fresh db
        db.create().expect("create tables");

        #[cfg(feature = "uuid")]
        let id = Uuid::new_v4();
        #[cfg(not(feature = "uuid"))]
        let id = 1;

        db.insert(users)
            .values([InsertUsers::new("Alex Smith", 26u64).with_id(id)])
            .execute()
            .expect("insert user");

        db.insert(posts)
            .values([InsertPosts::new(id).with_context("just testing")])
            .execute()
            .expect("insert post");

        let user_rows: Vec<SelectUsers> = db.select(()).from(users).all().expect("select users");
        let post_rows: Vec<SelectPosts> = db.select(()).from(posts).all().expect("select posts");

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
            .left_join(posts, eq(users.id, posts.user_id))
            .get()
            .expect("select users on posts.user_id");

        assert_eq!(row.id, id);
        assert_eq!(row.name, "Alex Smith");
        assert_eq!(row.age, 26);
        #[cfg(feature = "uuid")]
        assert!(!row.post_id.is_nil());
        #[cfg(not(feature = "uuid"))]
        assert_eq!(row.post_id, 1);
    }
}
