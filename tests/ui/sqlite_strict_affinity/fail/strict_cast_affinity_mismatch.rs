use drizzle::core::expr::cast;
use drizzle::sqlite::prelude::*;

#[SQLiteTable(strict)]
struct StrictUser {
    #[column(primary)]
    id: i32,
    name: String,
}

fn main() {
    let user = StrictUser::default();
    let _ = cast::<_, _, drizzle::core::types::Int>(user.name, drizzle::sqlite::types::Integer);
}
