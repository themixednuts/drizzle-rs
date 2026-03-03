use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

fn main() {
    // Calling .offset() without .limit() should fail — .offset() is only
    // available when the Lim typestate is HasLimit.
    let _ = drizzle::core::query::QueryBuilder::<drizzle::sqlite::values::SQLiteValue, User>::new()
        .offset(5);
}
