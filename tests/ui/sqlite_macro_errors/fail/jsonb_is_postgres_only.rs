use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Documents {
    #[column(JSONB)]
    value: String,
}

fn main() {}
