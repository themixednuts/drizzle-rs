use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Counters {
    #[column(PRIMARY)]
    id: u64,
    #[column(CHARSET = "utf8mb4")]
    count: u32,
}

fn main() {}
