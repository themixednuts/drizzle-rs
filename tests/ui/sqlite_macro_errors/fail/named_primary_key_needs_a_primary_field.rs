use drizzle::sqlite::prelude::*;

#[SQLiteTable(primary_key(name = "users_key"))]
struct Users {
    id: i64,
}

fn main() {}
