//! Test that COALESCE with incompatible types fails to compile.

use drizzle::sqlite::prelude::*;
use drizzle::core::expressions::coalesce;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();

    // ERROR: Int is not Compatible<Text>
    let _ = coalesce(user.id, "default");
}
