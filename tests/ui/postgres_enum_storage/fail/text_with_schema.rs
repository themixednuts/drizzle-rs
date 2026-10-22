use drizzle::postgres::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
#[postgres_enum(text, schema = "app")]
enum Channel {
    Email,
    Sms,
}

fn main() {}
