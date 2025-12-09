#![cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]

use drizzle::sqlite::prelude::*;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
#[SQLiteTable]
pub struct Users {
    #[column(primary, default_fn = Uuid::new_v4)]
    id: Uuid,
    name: String,
    age: i64,
}

#[cfg(feature = "uuid")]
#[SQLiteTable]
pub struct Posts {
    #[column(primary, default_fn = Uuid::new_v4)]
    id: Uuid,
    #[column(references = Users::id)]
    user_id: Uuid,
    context: Option<String>,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    id: i64,
    name: String,
    age: i64,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    id: i64,
    #[column(references = Users::id)]
    user_id: i64,
    context: Option<String>,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
}
