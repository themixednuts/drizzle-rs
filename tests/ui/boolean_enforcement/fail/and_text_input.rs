use drizzle::core::expr::and;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();
    // Text column is not BooleanLike — and() should reject it
    let _ = and([user.name]);
}
