use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
    active: bool,
}

fn main() {
    let user = User::default();
    // RHS of & must be BooleanLike — text column should fail
    let _ = eq(user.active, true) & user.name;
}
