use drizzle::postgres::prelude::*;

#[PostgresTable]
struct BareDefaults {
    #[column(DEFAULT)]
    literal: i32,
    #[column(DEFAULT_FN)]
    application: i32,
    #[column(DEFAULT_SQL)]
    expression: i32,
}

fn main() {}
