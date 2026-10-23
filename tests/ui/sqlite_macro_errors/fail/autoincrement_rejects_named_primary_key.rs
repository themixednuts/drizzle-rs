use drizzle::sqlite::prelude::*;

#[SQLiteTable(primary_key(name = "users_key"))]
struct Users {
    #[column(primary, autoincrement)]
    id: i64,
}

fn main() {}
