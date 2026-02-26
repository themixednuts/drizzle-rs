use drizzle::core::expr::case;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn main() {
    let user = User::default();
    // Int4 is not BooleanLike — CASE WHEN should reject integer conditions
    let _ = case::<PostgresValue>()
        .when(user.age, "result");
}
