use drizzle::core::expr::any;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    active: bool,
    age: i32,
}

fn main() {
    let user = User::default();
    // `any` enforces the same per-element boolean policy as a bare tuple.
    let _ = any::<PostgresValue, _>((user.active, user.age));
}
