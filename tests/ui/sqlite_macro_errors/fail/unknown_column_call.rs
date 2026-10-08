use drizzle::sqlite::prelude::*;

// `varchar(255)` is not a SQLite column attribute; it used to be ignored.
#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i64,
    #[column(varchar(255))]
    name: String,
}

fn main() {}
