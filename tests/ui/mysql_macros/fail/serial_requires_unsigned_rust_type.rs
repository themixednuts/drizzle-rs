use drizzle::mysql::prelude::*;

#[MySQLTable]
struct SerialRequiresUnsignedRustType {
    #[column(SERIAL)]
    id: i64,
}

fn main() {}
