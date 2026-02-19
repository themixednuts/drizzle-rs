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
    let _ = cast(user.age, drizzle::postgres::types::Int4);
}
