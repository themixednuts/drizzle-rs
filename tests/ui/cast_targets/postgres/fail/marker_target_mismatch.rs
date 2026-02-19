use drizzle::core::expr::cast;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn main() {
    let user = User::default();
    let _ = cast::<_, _, drizzle::core::types::Int>(user.age, drizzle::postgres::types::Float8);
}
