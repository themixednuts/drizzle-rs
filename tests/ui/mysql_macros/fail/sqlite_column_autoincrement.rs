use drizzle::mysql::prelude::*;

#[MySQLTable]
struct InvalidSqliteColumnOption {
    #[column(PRIMARY, AUTOINCREMENT)]
    id: i32,
}

fn main() {}
