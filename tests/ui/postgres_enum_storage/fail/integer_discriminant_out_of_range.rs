use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
#[repr(i64)]
enum OversizedStatus {
    #[default]
    Pending = 0,
    Complete = 5_000_000_000,
}

fn main() {}
