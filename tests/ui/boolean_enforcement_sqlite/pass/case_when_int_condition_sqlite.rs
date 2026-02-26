use drizzle::core::expr::case;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    active: i32,
}

fn main() {
    let user = User::default();
    // SQLite Integer IS BooleanLike — CASE WHEN should accept integer conditions
    let _ = case::<SQLiteValue>()
        .when(user.active, "yes")
        .r#else("no");
}
