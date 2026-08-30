use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Documents {
    #[column(BLOB, JSON)]
    value: String,
}

fn main() {}
