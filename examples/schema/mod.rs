use drizzle_rs::{SQLSchema, sqlite::SQLiteTable};
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
#[SQLiteTable]
pub struct Users {
    #[blob(primary, default_fn = Uuid::new_v4)]
    id: Uuid,
    #[text]
    name: String,
    #[integer]
    age: u64,
}

#[cfg(feature = "uuid")]
#[SQLiteTable]
pub struct Posts {
    #[blob(primary, default_fn = Uuid::new_v4)]
    id: Uuid,
    #[blob(references = Users::id)]
    user_id: Uuid,
    #[text]
    context: Option<String>,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable]
pub struct Users {
    #[integer(primary)]
    id: i64,
    #[text]
    name: String,
    #[integer]
    age: u64,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable]
pub struct Posts {
    #[integer(primary)]
    id: i64,
    #[integer(references = Users::id)]
    user_id: i64,
    #[text]
    context: Option<String>,
}

#[derive(SQLSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
}
