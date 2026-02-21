use drizzle::core::expr::not;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();
    // Text column is not BooleanLike — not() should reject it
    let _ = not(user.name);
}
