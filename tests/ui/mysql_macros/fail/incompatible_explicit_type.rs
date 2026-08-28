use drizzle::mysql::prelude::*;

#[MySQLTable]
struct InvalidExplicitType {
    #[column(INT)]
    value: String,
}

fn main() {}
