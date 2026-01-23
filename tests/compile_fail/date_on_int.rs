//! Test that using date() on an Int column fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expr::date;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();

    // ERROR: Int is not Temporal
    let _ = date(user.id);
}
