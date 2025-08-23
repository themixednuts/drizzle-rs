#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod common;

use drizzle_macros::drizzle_test;
use drizzle_rs::prelude::*;
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
#[derive(FromRow, Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct UserResult {
    id: Uuid,
    age: i64,
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[SQLiteTable(name = "json_users", strict)]
struct JsonUser {
    #[blob]
    id: Uuid,
    #[text]
    email: String,
    #[blob(json)]
    profile: Profile,
}
#[derive(SQLSchema)]
struct Schema {
    jsonuser: JsonUser,
}

#[cfg(all(feature = "serde", feature = "uuid"))]
drizzle_test!(json_storage, Schema, {
    let Schema { jsonuser } = schema;

    println!("{}", jsonuser.sql());

    let profile = Profile {
        age: 30,
        name: "John".to_string(),
        interests: vec!["Coding".to_string(), "Reading".to_string()],
    };

    let id = Uuid::new_v4();

    let stmt = db
        .insert(jsonuser)
        .values([InsertJsonUser::new(id, "john@test.com", profile)]);

    println!("{}", stmt.to_sql());

    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((
            jsonuser.id,
            cast(
                drizzle_rs::sqlite::conditions::json_extract(jsonuser.profile, "age"),
                "INTEGER",
            )
            .alias("age"),
        ))
        .from(jsonuser)
        .r#where(eq(jsonuser.id, id));

    println!("{}", stmt.to_sql());

    let user: UserResult = drizzle_exec!(stmt.get());

    assert_eq!(user.id, id);
    assert_eq!(user.age, 30);
});
