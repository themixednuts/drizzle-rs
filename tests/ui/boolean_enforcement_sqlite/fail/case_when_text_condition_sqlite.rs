use drizzle::core::expr::case;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = User::default();
    // Text is NOT BooleanLike on any dialect — CASE WHEN should reject text conditions
    let _ = case::<SQLiteValue>()
        .when(user.name, "result");
}
