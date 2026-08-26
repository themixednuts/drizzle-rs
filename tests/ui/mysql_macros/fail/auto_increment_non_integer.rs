use drizzle::mysql::prelude::*;

#[MySQLTable]
struct InvalidAutoIncrement {
    #[column(UNIQUE, AUTO_INCREMENT)]
    value: String,
}

fn main() {}
