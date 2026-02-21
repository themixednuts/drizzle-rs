use drizzle::core::expr::cast;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    active: bool,
}

fn main() {
    let user = User::default();
    // Boolean is not Compatible with Int4 — cast should fail
    let _ = cast(user.active, drizzle::postgres::types::Int4);
}
