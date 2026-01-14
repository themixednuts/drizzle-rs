//! Test that using AVG on a Text column fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expressions::avg;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();

    // ERROR: Text is not Numeric
    let _ = avg(user.name);
}
