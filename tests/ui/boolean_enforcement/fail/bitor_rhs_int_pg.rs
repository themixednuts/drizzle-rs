use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
    active: bool,
}

fn main() {
    let user = User::default();
    // BitOr (|): RHS must be BooleanLike — Int4 should be rejected
    let _ = eq(user.active, true) | user.age;
}
