#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
#![cfg(all(feature = "serde", feature = "uuid"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
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
    age: Option<i64>,
}

#[SQLiteTable(NAME = "json_users", STRICT)]
struct JsonUser {
    id: Uuid,
    email: String,
    #[column(JSON)]
    profile: Profile,
    raw: serde_json::Value,
}
#[derive(SQLiteSchema)]
struct Schema {
    jsonuser: JsonUser,
}

#[drizzle::test]
fn json_storage(db: &mut TestDb<Schema>) {
    let Schema { jsonuser } = schema;

    let profile = Profile {
        age: 30,
        name: "John".to_string(),
        interests: vec!["Coding".to_string(), "Reading".to_string()],
    };

    let id = Uuid::new_v4();

    let stmt = db.insert(jsonuser).values([InsertJsonUser::new(
        id,
        "john@test.com",
        profile.clone(),
        serde_json::json!({ "enabled": true }),
    )]);

    // let stmt2 =
    //     db.insert(jsonuser)
    //         .values([InsertJsonUser::new(id, "john@test.com", jsonb(profile))]);

    stmt.execute();

    let stmt = db
        .select((
            jsonuser.id,
            cast(
                drizzle::sqlite::expr::json_extract(jsonuser.profile, "age"),
                drizzle::sqlite::types::Integer,
            )
            .alias("age"),
        ))
        .from(jsonuser)
        .r#where(eq(jsonuser.id, id));

    let user: UserResult = stmt.get();

    assert_eq!(user.id, id);
    assert_eq!(user.age, Some(30));

    // The table-generated row owns the custom JSON codec.
    let stmt = db.select(()).from(jsonuser).r#where(eq(jsonuser.id, id));

    let result: SelectJsonUser = stmt.get();

    assert_eq!(result.id, id);
    assert_eq!(result.email, "john@test.com");
    assert_eq!(result.profile, profile);
    assert_eq!(result.raw, serde_json::json!({ "enabled": true }));
}
