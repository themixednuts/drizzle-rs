use drizzle::mysql::prelude::*;

#[MySQLTable]
struct InvalidGeneratedDefault {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(generated(STORED, "id + 1"), DEFAULT = 1)]
    generated_value: u64,
}

fn main() {}
