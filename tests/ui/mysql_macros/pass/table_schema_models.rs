use drizzle::mysql::prelude::*;

#[MySQLTable(
    DATABASE = "app_db",
    NAME = "accounts",
    ENGINE = "InnoDB",
    DEFAULT_CHARSET = "utf8mb4",
    COLLATE = "utf8mb4_0900_ai_ci"
)]
struct Accounts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    email: String,
    #[column(DEFAULT = 0)]
    login_count: u32,
}

#[derive(MySQLSchema)]
struct AppSchema {
    accounts: Accounts,
}

fn main() {
    let schema = AppSchema::new();
    let _insert = InsertAccounts::new("hello@example.test").with_login_count(7_u32);
    let _update = UpdateAccounts::default().with_email("renamed@example.test");
    let _select = SelectAccounts {
        id: 1_u64,
        email: "hello@example.test".to_owned(),
        login_count: 7_u32,
    };
    let _ = schema.accounts;
}
