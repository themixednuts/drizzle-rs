use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Documents {
    #[blob(JSON)]
    value: String,
}

fn main() {}
