use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    id: i32,
    #[column(default = String::new())]
    name: String,
}

fn main() {}
