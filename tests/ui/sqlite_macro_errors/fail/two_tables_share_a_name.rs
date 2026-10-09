use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct Users {
    #[column(primary)]
    id: i64,
}

#[SQLiteTable(NAME = "users")]
struct Accounts {
    #[column(primary)]
    id: i64,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    accounts: Accounts,
}

fn main() {}
