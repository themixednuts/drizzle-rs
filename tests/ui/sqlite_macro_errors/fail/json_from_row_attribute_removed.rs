use drizzle::sqlite::prelude::*;

#[derive(SQLiteFromRow)]
struct Document {
    #[json]
    value: String,
}

fn main() {}
