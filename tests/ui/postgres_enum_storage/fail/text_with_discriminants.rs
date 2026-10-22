use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
#[postgres_enum(text)]
enum Tier {
    Free = 1,
    Paid = 2,
}

fn main() {}
