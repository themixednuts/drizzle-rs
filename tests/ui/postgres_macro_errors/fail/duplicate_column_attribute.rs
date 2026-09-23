use drizzle::postgres::prelude::*;

// Only the first #[column] used to be read; the second one's options were
// dropped without a word.
#[PostgresTable]
struct Users {
    #[column(primary)]
    #[column(unique)]
    id: i32,
}

fn main() {}
