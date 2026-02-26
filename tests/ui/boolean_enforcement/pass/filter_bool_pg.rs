use drizzle::core::expr::{count_all, eq};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    active: bool,
}

fn main() {
    let user = User::default();

    // Boolean column is BooleanLike — filter() should accept it directly
    let _ = count_all::<PostgresValue>().filter(user.active);

    // Comparison expression returns Boolean — filter() should accept it too
    let _ = count_all::<PostgresValue>().filter(eq(user.id, 1));
}
