use drizzle::core::expr::group_concat;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn main() {
    let user = User::default();
    // Integer is not Textual — group_concat should reject it
    let _ = group_concat(user.age);
}
