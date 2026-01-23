//! Test that using concat on an Int column fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expr::concat;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();

    // ERROR: Int is not Textual
    let _ = concat(user.id, user.name);
}
