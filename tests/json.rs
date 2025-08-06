#[cfg(all(feature = "rusqlite", feature = "sqlite", feature = "serde"))]
use drizzle_rs::prelude::*;
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use sqlite::conditions::json_extract;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Profile {
    age: i64,
    name: String,
    interests: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct UserResult {
    id: Uuid,
    age: i64,
}

impl TryFrom<&Row<'_>> for UserResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<UserResult, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            age: row.get(1)?,
        })
    }
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

#[test]
fn json_storage() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();

    conn.execute(JsonUser::SQL, []).unwrap();

    let profile = Profile {
        age: 30,
        name: "John".to_string(),
        interests: vec!["Coding".to_string(), "Reading".to_string()],
    };

    let id = Uuid::new_v4();
    conn.execute(
        "INSERT INTO json_users (id, email, profile) VALUES (?, ?, ?)",
        rusqlite::params![id, "john@test.com", profile],
    )
    .unwrap();

    let db = drizzle!(conn, [JsonUser]);

    let stmt = db
        .select(columns![
            JsonUser::id,
            json_extract(JsonUser::profile, "age")
        ])
        .from::<JsonUser>()
        .r#where(eq(JsonUser::id, id));

    let user: UserResult = stmt.get().unwrap();

    assert_eq!(user.id, id);
    assert_eq!(user.age, 30);
}
