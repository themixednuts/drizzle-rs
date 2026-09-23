use drizzle::sqlite::prelude::*;

#[SQLiteTable(without_rowid)]
struct Users {
    #[column(primary, autoincrement)]
    id: i64,
}

fn main() {}
