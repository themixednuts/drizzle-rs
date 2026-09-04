use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(bigserial, primary)]
    id: i32,
}

fn main() {}
