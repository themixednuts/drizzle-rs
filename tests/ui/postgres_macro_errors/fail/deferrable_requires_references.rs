use drizzle::postgres::prelude::*;

// The error names `deferrable`; it used to talk about on_update.
#[PostgresTable]
struct Posts {
    #[column(primary)]
    id: i32,
    #[column(deferrable)]
    user_id: i32,
}

fn main() {}
