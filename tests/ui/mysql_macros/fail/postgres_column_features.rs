use drizzle::mysql::prelude::*;

#[MySQLTable]
struct InvalidPostgresColumnOption {
    #[column(PRIMARY, BIGSERIAL)]
    id: i32,
}

fn main() {}
