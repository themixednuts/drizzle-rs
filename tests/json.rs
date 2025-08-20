#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod common;

use drizzle_rs::{error::DrizzleError, prelude::*};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Profile {
    age: i64,
    name: String,
    interests: Vec<String>,
}

#[derive(FromRow, Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct UserResult {
    id: Uuid,
    age: i64,
}

#[SQLiteTable(name = "json_users", strict)]
struct JsonUser {
    #[blob]
    id: Uuid,
    #[text]
    email: String,
    #[blob(json)]
    profile: Profile,
}

#[tokio::test]
async fn json_storage() {
    #[derive(SQLSchema)]
    struct Schema {
        jsonuser: JsonUser,
    }
    let conn = setup_test_db!();

    exec_sql!(conn, JsonUser::SQL.sql().as_str(), db_params!());

    let profile = Profile {
        age: 30,
        name: "John".to_string(),
        interests: vec!["Coding".to_string(), "Reading".to_string()],
    };

    let id = Uuid::new_v4();

    let (db, Schema { jsonuser }) = drizzle!(conn, Schema);
    drizzle_exec!(
        db.insert(jsonuser)
            .values([InsertJsonUser::new(id, "john@test.com", profile)])
            .execute()
    );

    let stmt = db
        .select((
            jsonuser.id,
            drizzle_rs::sqlite::conditions::json_extract(jsonuser.profile, "age").alias("age"),
        ))
        .from(jsonuser)
        .r#where(eq(jsonuser.id, id));

    // let sql = stmt.to_sql();
    // println!("{sql}");

    let user: UserResult = drizzle_exec!(stmt.get());

    assert_eq!(user.id, id);
    assert_eq!(user.age, 30);
}
