use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    email: String,
}

#[PostgresTable]
struct Accounts {
    #[column(primary)]
    id: i32,
    email: String,
}

// Index names are unique per schema, not per table.
#[PostgresIndex(name = "email_idx")]
struct UsersEmailIdx(Users::email);

#[PostgresIndex(name = "email_idx")]
struct AccountsEmailIdx(Accounts::email);

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
    accounts: Accounts,
    users_email: UsersEmailIdx,
    accounts_email: AccountsEmailIdx,
}

fn main() {}
