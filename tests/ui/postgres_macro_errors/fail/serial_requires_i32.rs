use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(serial, primary)]
    id: i64,
}

fn main() {}
