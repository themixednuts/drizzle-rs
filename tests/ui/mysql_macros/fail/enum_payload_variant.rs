use drizzle::mysql::prelude::*;

#[derive(MySQLEnum)]
enum InvalidInlineEnum {
    Value(String),
}

fn main() {}
