use drizzle::mysql::prelude::*;

#[MySQLTable]
struct Accounts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    email: String,
    #[column(generated(STORED, "CHAR_LENGTH(email)"))]
    email_length: u32,
}

fn main() {
    let _ = InsertAccounts::new("hello@example.test").with_email_length(18_u32);
}
