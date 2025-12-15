#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::conditions::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;
use drizzle_sqlite::SQLiteValue;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Profile {
    age: i64,
    name: String,
    interests: Vec<String>,
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[derive(SQLiteFromRow, Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct UserResult {
    id: Uuid,
    age: i64,
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[SQLiteTable(NAME = "json_users", STRICT)]
struct JsonUser {
    id: Uuid,
    email: String,
    #[column(JSON)]
    profile: Profile,
}
#[cfg(all(feature = "serde", feature = "uuid"))]
#[derive(SQLiteSchema)]
struct Schema {
    jsonuser: JsonUser,
}

#[cfg(all(feature = "serde", feature = "uuid"))]
sqlite_test!(json_storage, Schema, {
    let Schema { jsonuser } = schema;

    println!("{}", jsonuser.sql());

    let profile = Profile {
        age: 30,
        name: "John".to_string(),
        interests: vec!["Coding".to_string(), "Reading".to_string()],
    };

    let id = Uuid::new_v4();

    let stmt =
        db.insert(jsonuser)
            .values([InsertJsonUser::new(id, "john@test.com", profile.clone())]);

    // let stmt2 =
    //     db.insert(jsonuser)
    //         .values([InsertJsonUser::new(id, "john@test.com", jsonb(profile))]);

    println!("{}", stmt.to_sql());

    println!("{}", profile.to_sql());
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((
            jsonuser.id,
            drizzle::sqlite::conditions::json_extract(jsonuser.profile, "age").alias("age"),
        ))
        .from(jsonuser)
        .r#where(eq(jsonuser.id, id));

    println!("{}", stmt.to_sql());

    let user: UserResult = drizzle_exec!(stmt.get());

    assert_eq!(user.id, id);
    assert_eq!(user.age, 30);
});
