use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Documents {
    #[column(JSON, JSONB)]
    value: String,
}

fn main() {}
