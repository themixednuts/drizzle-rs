use drizzle::mysql::prelude::*;

#[MySQLTable(STRICT)]
struct InvalidSqliteTableOption {
    id: i32,
}

fn main() {}
