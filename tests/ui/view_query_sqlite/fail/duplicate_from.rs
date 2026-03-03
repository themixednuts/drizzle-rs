use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

#[SQLiteView(
    query(
        select(User::id, User::name),
        from(User),
        from(User),
    ),
    NAME = "bad_view"
)]
struct BadView {
    id: i32,
    name: String,
}

fn main() {}
