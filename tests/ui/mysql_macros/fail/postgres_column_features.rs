use drizzle::mysql::prelude::*;

#[MySQLTable]
struct InvalidPostgresColumnOption {
    #[column(PRIMARY, SERIAL)]
    id: i32,
}

fn main() {}
