use drizzle::sqlite::prelude::*;

// The table handle derives its own traits; this derive used to be dropped.
#[SQLiteTable]
#[derive(Debug)]
struct Users {
    #[column(primary)]
    id: i64,
}

fn main() {}
