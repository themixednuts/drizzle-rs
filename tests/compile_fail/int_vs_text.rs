//! Test that comparing Int column with Text literal fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expr::eq;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();

    // ERROR: Int is not Compatible<Text>
    let _ = eq(user.id, "hello");
}
