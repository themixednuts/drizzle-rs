use drizzle::sqlite::prelude::*;

#[SQLiteTable(strict)]
struct StrictNumeric {
    #[column(primary)]
    id: i32,
    #[column(numeric)]
    amount: i64,
}

fn main() {
    let _ = StrictNumeric::default();
}
