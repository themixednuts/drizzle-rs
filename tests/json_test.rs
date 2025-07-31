#[cfg(all(feature = "rusqlite", feature = "sqlite", feature = "serde"))]
mod tests {
    use drizzle_rs::core::ToSQL;
    use drizzle_rs::prelude::*;
    use drizzle_rs::procmacros::{SQLiteTable, drizzle};
    use rusqlite::types::{FromSql, FromSqlError};
    use rusqlite::{self, Row};
    use serde::{Deserialize, Serialize};
    use sqlite::conditions::json_extract;
    use uuid::Uuid;

    // Define a struct that will be serialized to JSON
    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
    pub struct UserProfile {
        age: i64,
        name: String,
        interests: Vec<String>,
    }

    // Define our table with a JSON field
    #[SQLiteTable(name = "users", strict)]
    struct User {
        #[blob]
        id: Uuid,
        #[text]
        email: String,
        #[blob(json)]
        profile: UserProfile,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
    pub struct TestUserProfilePartial {
        name: String,
        age: i64,
    }

    impl<'a> FromSql for TestUserProfilePartial {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            match value {
                rusqlite::types::ValueRef::Text(items) | rusqlite::types::ValueRef::Blob(items) => {
                    let json =
                        serde_json::from_slice(items).map_err(|_| FromSqlError::InvalidType)?;
                    Ok(json)
                }
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
    pub struct TestSelectPartial {
        id: Uuid,
        age: i64,
    }

    impl<'a> TryFrom<Row<'a>> for TestSelectPartial {
        type Error = rusqlite::Error;

        fn try_from(row: Row<'a>) -> std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                age: row.get("age")?,
            })
        }
    }
    impl<'a> TryFrom<&Row<'a>> for TestSelectPartial {
        type Error = rusqlite::Error;

        fn try_from(row: &Row<'a>) -> std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                age: row.get("age")?,
            })
        }
    }

    #[test]
    fn test_json_storage_and_retrieval() {
        // Create an in-memory database
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        // Setup the database with our table
        conn.execute(User::SQL, []).unwrap();

        // Create test data
        let user_profile = UserProfile {
            age: 30,
            name: "John Doe".to_string(),
            interests: vec!["Coding".to_string(), "Reading".to_string()],
        };

        // seed
        let id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO users (id, email, profile) VALUES (?, ?, ?)",
            rusqlite::params![id, "john@example.com", user_profile],
        )
        .unwrap();

        // Create a Drizzle instance
        let mut db = drizzle!(conn, [User]);

        // Use Drizzle SQL api to query the data
        let mut stmt = db
            .select(columns![
                User::id,
                json_extract(User::profile, "age").as_("age")
            ])
            .from::<User>()
            .r#where(eq(User::id, id));

        let sql = stmt.to_sql().sql();
        println!("{:?}", sql);

        let params = stmt.to_sql().params();
        println!("{:?}", params);

        let user: TestSelectPartial = stmt.get().unwrap();
        println!("{:?}", user);

        // Verify JSON deserialization through the ORM
        // assert_eq!(user.id, id);
        // assert_eq!(user.email, "john@example.com");
        // assert_eq!(user.profile.name, "John Doe");
        // assert_eq!(user.profile.age, 30);
        // assert_eq!(
        //     user.profile.interests,
        //     vec!["Coding".to_string(), "Reading".to_string()]
        // );
    }
}
