use drizzle::migrations::Schema as _;
use drizzle::mysql::prelude::*;

#[MySQLTable(NAME = "accounts")]
struct Accounts {
    id: u64,
    #[column(VARCHAR(255))]
    email: String,
    #[column(TEXT)]
    biography: String,
    #[column(JSON)]
    profile: serde_json::Value,
}

#[MySQLIndex]
struct AccountsSearchIdx(
    #[index(prefix = 32, desc)] Accounts::biography,
    #[index(expr = "lower(email)", asc)] Accounts::profile,
    #[index(desc)] Accounts::id,
);

#[derive(MySQLSchema)]
struct AppSchema {
    accounts: Accounts,
    accounts_search_idx: AccountsSearchIdx,
}

fn main() {
    assert_eq!(AccountsSearchIdx::KEY_PARTS.len(), 3);
    assert!(AccountsSearchIdx::DDL_SQL.contains("`biography`(32) DESC"));
    assert!(AccountsSearchIdx::DDL_SQL.contains("(lower(email)) ASC"));
    let _ = AppSchema::new().to_snapshot();
}
