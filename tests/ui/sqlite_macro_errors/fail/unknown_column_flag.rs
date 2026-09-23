use drizzle::sqlite::prelude::*;

// A misspelled flag used to be stored and ignored: this compiled with no
// primary key.
#[SQLiteTable]
struct Users {
    #[column(primay)]
    id: i32,
}

fn main() {}
