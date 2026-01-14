//! Test that using SUM on a Text column fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expressions::sum;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();

    // ERROR: Text is not Numeric
    let _ = sum(user.name);
}
