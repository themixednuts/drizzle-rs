use drizzle::sqlite::prelude::*;

#[SQLiteTable(strict)]
struct StrictAny {
    #[column(primary)]
    id: i32,
    #[column(any)]
    payload: String,
}

fn main() {
    let _ = StrictAny::default();
}
