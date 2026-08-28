use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Document {
    #[column(JSON)]
    value: i32,
}

fn main() {}
