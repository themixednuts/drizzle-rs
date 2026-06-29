use drizzle::core::expr::count;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn main() {
    let user = User::default();
    // Int4 is not BooleanLike — FILTER should reject integer conditions
    let _ = count::<PostgresValue, _>(()).filter(user.age);
}
