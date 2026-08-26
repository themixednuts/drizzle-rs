use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Documents {
    #[column(PRIMARY)]
    id: u64,
    #[column(JSON, UNIQUE)]
    payload: serde_json::Value,
}

fn main() {}
