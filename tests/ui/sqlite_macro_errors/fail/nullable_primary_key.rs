use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    email: Option<String>,
}

fn main() {}
