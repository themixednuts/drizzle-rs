use drizzle::sqlite::prelude::*;

struct Opaque;

#[SQLiteTable]
struct User {
    id: i32,
    payload: Opaque,
}

fn main() {}
