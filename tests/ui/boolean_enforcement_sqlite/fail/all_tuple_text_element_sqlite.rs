use drizzle::core::expr::all;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    active: i32,
    name: String,
}

fn main() {
    let user = User::default();
    // `all` enforces the same per-element boolean policy as a bare tuple.
    let _ = all::<SQLiteValue, _>((user.active, user.name));
}
