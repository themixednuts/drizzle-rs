use drizzle::sqlite::prelude::*;

#[derive(Debug, Clone)]
struct Opaque;

#[SQLiteTable]
struct User {
    id: i32,
    payload: Opaque,
}

fn main() {}
