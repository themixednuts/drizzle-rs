use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
}

// The schema would create `users` twice.
#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    accounts: Users,
}

fn main() {}
