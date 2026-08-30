use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    id: i32,
    #[column(default = if true { 1 } else { 2 })]
    value: i32,
}

fn main() {}
