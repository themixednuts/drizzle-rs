use drizzle::postgres::expr::{ilike, regex_match};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Account {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let account = Account::default();
    // ILIKE and `~` need text operands.
    let _ = ilike(account.id, "1%");
    let _ = ilike(account.name, 5i32);
    let _ = regex_match(account.id, "^1");
}
