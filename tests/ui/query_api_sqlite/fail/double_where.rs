use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    active: bool,
}

fn main() {
    let user = User::default();
    // Calling .where() twice should fail — the typestate moves from NoWhere to HasWhere
    // after the first call, and .where() is only available on NoWhere.
    let _ = drizzle::core::query::QueryBuilder::<drizzle::sqlite::values::SQLiteValue, User>::new()
        .r#where(eq(user.name, "Alice"))
        .r#where(eq(user.active, true));
}
