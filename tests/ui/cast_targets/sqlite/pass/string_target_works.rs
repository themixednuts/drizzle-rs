use drizzle::core::expr::cast;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn main() {
    let user = User::default();
    let _ = cast::<_, _, drizzle::sqlite::types::Integer>(user.age, "INTEGER");
}
