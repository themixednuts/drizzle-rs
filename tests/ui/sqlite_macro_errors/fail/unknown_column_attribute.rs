use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(defualt = 1)]
    value: i32,
}

fn main() {}
