use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Accounts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    email: String,
}

#[MySQLIndex(unique, where = "email <> ''")]
struct InvalidPartialIndex(Accounts::email);

fn main() {}
