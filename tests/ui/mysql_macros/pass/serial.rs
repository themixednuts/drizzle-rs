use drizzle::mysql::prelude::*;

#[MySQLTable]
struct SerialIds {
    #[column(SERIAL)]
    id: u64,
    name: String,
}

fn main() {
    let _insert = InsertSerialIds::new("serial");
}
