use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary, autoincrement)]
    id: String,
}

fn main() {}
