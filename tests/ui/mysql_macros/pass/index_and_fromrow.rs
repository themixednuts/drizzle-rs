use drizzle::core::ToSQL;
use drizzle::mysql::prelude::*;

#[MySQLTable(NAME = "accounts")]
struct Accounts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    email: String,
}

#[MySQLIndex(unique)]
struct AccountsEmailIdx(Accounts::email);

#[derive(MySQLSchema)]
struct AppSchema {
    accounts: Accounts,
    accounts_email_idx: AccountsEmailIdx,
}

#[derive(MySQLFromRow)]
#[from(Accounts)]
struct AccountRow {
    id: u64,
    #[column(Accounts::email)]
    email: String,
}

fn assert_mysql_selector<T>(_: T)
where
    for<'a> T: ToSQL<'a, drizzle::mysql::MySQLValue<'a>>,
{
}

fn main() {
    let schema = AppSchema::new();
    let _ = schema.accounts_email_idx;
    let _ = AccountsEmailIdx::new().to_sql();
    assert_mysql_selector(AccountRow::Select);
}
