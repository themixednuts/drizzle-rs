use drizzle::core::expr::or;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn main() {
    let user = User::default();
    // Int4 is not BooleanLike — or() should reject integer inputs
    let _ = or::<PostgresValue, _, _>([user.age]);
}
