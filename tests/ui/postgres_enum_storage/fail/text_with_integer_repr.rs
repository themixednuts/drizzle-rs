use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
#[postgres_enum(text)]
#[repr(i32)]
enum Level {
    Low = 1,
    High = 2,
}

fn main() {}
