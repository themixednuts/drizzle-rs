use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(primary, uniqe)]
    id: i32,
}

fn main() {}
