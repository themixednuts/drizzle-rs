use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(smallserial, primary)]
    id: i32,
}

fn main() {}
