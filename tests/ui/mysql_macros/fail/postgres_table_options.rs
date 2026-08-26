use drizzle::mysql::prelude::*;

#[MySQLTable(UNLOGGED)]
struct InvalidPostgresTableOption {
    id: i32,
}

fn main() {}
