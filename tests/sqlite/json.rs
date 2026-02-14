#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#![cfg(all(feature = "serde", feature = "uuid"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Profile {
    age: i64,
    name: String,
    interests: Vec<String>,
}

#[derive(SQLiteFromRow, Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct UserResult {
    id: Uuid,
    age: i64,
}

#[SQLiteTable(NAME = "json_users", STRICT)]
struct JsonUser {
    id: Uuid,
    email: String,
    #[column(JSON)]
    profile: Profile,
}
#[derive(SQLiteSchema)]
struct Schema {
    jsonuser: JsonUser,
}

#[derive(SQLiteFromRow, Debug, PartialEq, Default)]
pub struct JsonReadResult {
    id: Uuid,
    email: String,
    #[json]
    profile: Profile,
}

sqlite_test!(json_storage, Schema, {
    let Schema { jsonuser } = schema;

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

    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((
            jsonuser.id,
            drizzle::sqlite::expressions::json_extract(jsonuser.profile, "age").alias("age"),
        ))
        .from(jsonuser)
        .r#where(eq(jsonuser.id, id));

    let user: UserResult = drizzle_exec!(stmt.get());

    assert_eq!(user.id, id);
    assert_eq!(user.age, 30);

    // Test reading full JSON column back via #[json] on FromRow struct
    let stmt = db
        .select((jsonuser.id, jsonuser.email, jsonuser.profile))
        .from(jsonuser)
        .r#where(eq(jsonuser.id, id));

    let result: JsonReadResult = drizzle_exec!(stmt.get());

    assert_eq!(result.id, id);
    assert_eq!(result.email, "john@test.com");
    assert_eq!(result.profile, profile);
});
