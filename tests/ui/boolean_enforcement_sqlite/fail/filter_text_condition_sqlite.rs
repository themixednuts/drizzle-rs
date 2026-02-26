use drizzle::core::expr::count_all;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();
    // Text is NOT BooleanLike — filter() should reject text conditions
    let _ = count_all::<SQLiteValue>().filter(user.name);
}
