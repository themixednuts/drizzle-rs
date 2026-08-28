use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Document {
    #[column(BLOB)]
    #[column(JSON)]
    value: String,
}

fn main() {}
