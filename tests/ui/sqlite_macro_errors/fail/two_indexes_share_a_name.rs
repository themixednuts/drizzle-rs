use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
    email: String,
    name: String,
}

#[SQLiteIndex(name = "users_idx")]
struct UsersEmailIdx(Users::email);

#[SQLiteIndex(name = "users_idx")]
struct UsersNameIdx(Users::name);

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    email_idx: UsersEmailIdx,
    name_idx: UsersNameIdx,
}

fn main() {}
